use crate::domain::model::{ModelDefinition, ModelProvider, ProviderType};
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::services::speech_service::resolve_local_runtime_model_path;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Deserialize)]
pub struct RegisterLocalRuntimeModelCommand {
    #[serde(alias = "providerName")]
    pub(crate) provider_name: String,
    #[serde(alias = "modelName")]
    pub(crate) model_name: String,
    #[serde(alias = "modelPath")]
    pub(crate) model_path: String,
    #[serde(alias = "capabilityTags")]
    pub(crate) capability_tags: Option<String>,
    pub(crate) notes: Option<String>,
    #[serde(alias = "contextWindow")]
    pub(crate) context_window: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct InstallManagedLocalModelCommand {
    #[serde(alias = "providerName")]
    pub(crate) provider_name: String,
    #[serde(alias = "modelName")]
    pub(crate) model_name: String,
    #[serde(alias = "downloadUrl")]
    pub(crate) download_url: String,
    #[serde(alias = "fileName")]
    pub(crate) file_name: String,
    #[serde(alias = "capabilityTags")]
    pub(crate) capability_tags: Option<String>,
    pub(crate) notes: Option<String>,
    #[serde(alias = "contextWindow")]
    pub(crate) context_window: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelRegistrationResult {
    pub file_path: String,
    pub downloaded: bool,
    pub provider: ModelProvider,
    pub model_definition: ModelDefinition,
}

pub(crate) struct LocalRuntimeRegistrationInput<'a> {
    pub(crate) provider_name: &'a str,
    pub(crate) model_name: &'a str,
    pub(crate) model_path: &'a str,
    pub(crate) capability_tags: Option<&'a str>,
    pub(crate) notes: Option<&'a str>,
    pub(crate) context_window: Option<i64>,
    pub(crate) downloaded: bool,
}

pub(crate) async fn upsert_local_runtime_registration(
    state: &AppState,
    input: LocalRuntimeRegistrationInput<'_>,
) -> Result<LocalModelRegistrationResult, AppError> {
    let normalized_path = resolve_local_runtime_model_path(input.model_path)?;
    let normalized_path_string = normalized_path.display().to_string();

    let existing_provider = model_repo::list_providers(&state.db)
        .await?
        .into_iter()
        .find(|provider| {
            matches!(provider.provider_type, ProviderType::LocalRuntime)
                && provider.base_url == normalized_path_string
        });

    let provider = if let Some(provider) = existing_provider {
        provider
    } else {
        let provider_id = uuid::Uuid::new_v4().to_string();
        model_repo::create_provider(
            &state.db,
            &provider_id,
            input.provider_name,
            ProviderType::LocalRuntime.as_str(),
            &normalized_path_string,
            None,
        )
        .await?
    };

    let existing_model = model_repo::list_model_definitions(&state.db)
        .await?
        .into_iter()
        .find(|model| model.provider_id == provider.id && model.name == input.model_name);

    let model_definition = if let Some(model) = existing_model {
        model
    } else {
        let model_id = uuid::Uuid::new_v4().to_string();
        model_repo::create_model_definition(
            &state.db,
            &model_id,
            &provider.id,
            input.model_name,
            input.context_window,
            input.capability_tags,
            input.notes,
        )
        .await?
    };

    Ok(LocalModelRegistrationResult {
        file_path: normalized_path_string,
        downloaded: input.downloaded,
        provider,
        model_definition,
    })
}

#[tauri::command]
pub async fn browse_for_local_model_file() -> Result<Option<String>, AppError> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(r#"POSIX path of (choose file with prompt "Select local model file")"#)
        .output()
        .map_err(|error| AppError::Validation(format!("Failed to open model picker: {error}")))?;

    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

#[tauri::command]
pub async fn register_local_runtime_model_command(
    state: State<'_, AppState>,
    request: RegisterLocalRuntimeModelCommand,
) -> Result<LocalModelRegistrationResult, AppError> {
    upsert_local_runtime_registration(
        state.inner(),
        LocalRuntimeRegistrationInput {
            provider_name: &request.provider_name,
            model_name: &request.model_name,
            model_path: &request.model_path,
            capability_tags: request.capability_tags.as_deref(),
            notes: request.notes.as_deref(),
            context_window: request.context_window,
            downloaded: false,
        },
    )
    .await
}

#[tauri::command]
pub async fn install_managed_local_model_command(
    state: State<'_, AppState>,
    request: InstallManagedLocalModelCommand,
) -> Result<LocalModelRegistrationResult, AppError> {
    let safe_dir = slugify(&request.provider_name);
    let models_dir = state.app_data_dir.join("models").join(safe_dir);
    tokio::fs::create_dir_all(&models_dir).await?;
    let destination_path = models_dir.join(request.file_name.trim());

    let mut downloaded = false;
    if !destination_path.exists() {
        let response = reqwest::get(request.download_url.trim())
            .await
            .map_err(|error| AppError::Provider(format!("Failed to download model: {error}")))?;
        if !response.status().is_success() {
            return Err(AppError::Provider(format!(
                "Failed to download model: HTTP {}",
                response.status()
            )));
        }

        let mut file = tokio::fs::File::create(&destination_path).await?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|error| {
                AppError::Provider(format!("Failed to read model download stream: {error}"))
            })?;
            file.write_all(&bytes).await?;
        }
        file.flush().await?;
        downloaded = true;
    }

    upsert_local_runtime_registration(
        state.inner(),
        LocalRuntimeRegistrationInput {
            provider_name: &request.provider_name,
            model_name: &request.model_name,
            model_path: destination_path.to_str().ok_or_else(|| {
                AppError::Validation("Installed model path is not valid UTF-8".to_string())
            })?,
            capability_tags: request.capability_tags.as_deref(),
            notes: request.notes.as_deref(),
            context_window: request.context_window,
            downloaded,
        },
    )
    .await
}

fn slugify(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

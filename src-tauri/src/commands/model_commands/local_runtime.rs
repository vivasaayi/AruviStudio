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
    install_managed_local_model(state.inner(), request).await
}

pub(crate) async fn install_managed_local_model(
    state: &AppState,
    request: InstallManagedLocalModelCommand,
) -> Result<LocalModelRegistrationResult, AppError> {
    use std::path::{Component, Path};
    const MAX_MODEL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
    let filename = request.file_name.trim();
    if filename.is_empty() || filename.contains(['/', '\\']) || !matches!(Path::new(filename).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]) {
        return Err(AppError::Validation("Model filename must be a single file name".into()));
    }
    let safe_dir = slugify(&request.provider_name);
    if safe_dir.is_empty() || request.model_name.trim().is_empty() {
        return Err(AppError::Validation("Provider and model names are required".into()));
    }
    let url = reqwest::Url::parse(request.download_url.trim()).map_err(|_| AppError::Validation("Invalid model download URL".into()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(AppError::Validation("Invalid model download URL".into()));
    }
    let models_root = state.app_data_dir.join("models");
    tokio::fs::create_dir_all(&models_root).await?;
    if tokio::fs::symlink_metadata(&models_root).await?.file_type().is_symlink() { return Err(AppError::Validation("Model directory cannot be a symlink".into())); }
    let models_dir = models_root.join(safe_dir);
    tokio::fs::create_dir_all(&models_dir).await?;
    if tokio::fs::symlink_metadata(&models_dir).await?.file_type().is_symlink() { return Err(AppError::Validation("Model directory cannot be a symlink".into())); }
    let destination_path = models_dir.join(filename);

    let mut downloaded = false;
    if let Ok(metadata) = tokio::fs::symlink_metadata(&destination_path).await {
        if !metadata.is_file() || metadata.len() == 0 { return Err(AppError::Validation("Existing model must be a nonempty regular file".into())); }
    } else {
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(3600)).build().map_err(|_| AppError::Provider("Failed to initialize model download".into()))?;
        let response = client.get(url).send()
            .await
            .map_err(|error| AppError::Provider(format!("Failed to download model: {}", error.without_url())))?;
        if !response.status().is_success() {
            return Err(AppError::Provider(format!(
                "Failed to download model: HTTP {}",
                response.status()
            )));
        }

        if response.content_length().is_some_and(|size| size > MAX_MODEL_BYTES) { return Err(AppError::Validation("Model download exceeds size limit".into())); }
        let temporary = tempfile::NamedTempFile::new_in(&models_dir)?;
        let mut file = tokio::fs::File::from_std(temporary.reopen()?);
        let mut stream = response.bytes_stream();
        let mut size = 0u64;
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|error| {
                AppError::Provider(format!("Failed to read model download stream: {}", error.without_url()))
            })?;
            size = size.checked_add(bytes.len() as u64).ok_or_else(|| AppError::Validation("Model download exceeds size limit".into()))?;
            if size > MAX_MODEL_BYTES { return Err(AppError::Validation("Model download exceeds size limit".into())); }
            file.write_all(&bytes).await?;
        }
        if size == 0 { return Err(AppError::Validation("Model download was empty".into())); }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        temporary.persist_noclobber(&destination_path).map_err(|error| AppError::Io(error.error))?;
        downloaded = true;
    }

    upsert_local_runtime_registration(
        state,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, thread};

    fn request(url: String, filename: &str) -> InstallManagedLocalModelCommand {
        InstallManagedLocalModelCommand { provider_name: "Test provider".into(), model_name: "fixture".into(), download_url: url, file_name: filename.into(), capability_tags: None, notes: None, context_window: None }
    }
    fn server(body: &'static [u8], advertised: usize) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0; 4096]; let _ = socket.read(&mut buf);
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {advertised}\r\nConnection: close\r\n\r\n").unwrap();
            socket.write_all(body).unwrap();
        });
        (format!("http://{addr}/model"), handle)
    }

    #[tokio::test]
    async fn managed_download_confines_paths_and_publishes_only_complete_files() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::persistence::db::create_pool(&format!("sqlite:{}", dir.path().join("test.db").display())).await.unwrap();
        let state = AppState::new(pool, dir.path().to_path_buf()).await.unwrap();
        for bad in ["../escaped", "/tmp/escaped", "dir/model", "dir\\model", ".", "..", ""] {
            assert!(install_managed_local_model(&state, request("http://127.0.0.1:1".into(), bad)).await.is_err(), "{bad}");
        }
        assert!(!dir.path().join("escaped").exists());
        let (url, first_server) = server(b"partial", 100);
        assert!(install_managed_local_model(&state, request(url, "model.bin")).await.is_err());
        first_server.join().unwrap();
        let model_dir = dir.path().join("models/test-provider");
        assert_eq!(std::fs::read_dir(&model_dir).unwrap().count(), 0);
        let (url, complete_server) = server(b"complete model", 14);
        let installed = install_managed_local_model(&state, request(url, "model.bin")).await.unwrap();
        complete_server.join().unwrap();
        assert!(installed.downloaded);
        assert_eq!(std::fs::read(&installed.file_path).unwrap(), b"complete model");
        let reused = install_managed_local_model(&state, request("http://127.0.0.1:1".into(), "model.bin")).await.unwrap();
        assert!(!reused.downloaded);
        assert_eq!(reused.provider.id, installed.provider.id);
        assert_eq!(reused.model_definition.id, installed.model_definition.id);
        let bad_mcp = crate::mcp::dispatch_tool(&state, "aruvi_models", serde_json::json!({"action":"install_managed_local_model","arguments":{
            "provider_name":"MCP provider","model_name":"fixture","file_name":"../escape.bin","download_url":"http://127.0.0.1:1/model"
        }})).await;
        assert!(bad_mcp.is_err());
        let (url, mcp_server) = server(b"mcp model", 9);
        let mcp = crate::mcp::dispatch_tool(&state, "aruvi_models", serde_json::json!({"action":"install_managed_local_model","arguments":{
            "provider_name":"MCP provider","model_name":"fixture","file_name":"mcp.bin","download_url":url
        }})).await.unwrap();
        mcp_server.join().unwrap();
        assert_eq!(std::fs::read(mcp["result"]["file_path"].as_str().unwrap()).unwrap(), b"mcp model");
    }
}

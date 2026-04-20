use crate::domain::model::{ModelDefinition, ModelProvider, ProviderType};
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
use crate::services::speech_service::resolve_local_runtime_model_path;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;
use tracing::warn;

#[derive(Debug, Clone, Serialize)]
struct ChatStreamChunkEvent {
    stream_id: String,
    delta: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatStreamDoneEvent {
    stream_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatStreamErrorEvent {
    stream_id: String,
    error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelRegistrationResult {
    pub file_path: String,
    pub downloaded: bool,
    pub provider: ModelProvider,
    pub model_definition: ModelDefinition,
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

pub(crate) async fn upsert_local_runtime_registration(
    state: &AppState,
    provider_name: &str,
    model_name: &str,
    model_path: &str,
    capability_tags: Option<&str>,
    notes: Option<&str>,
    context_window: Option<i64>,
    downloaded: bool,
) -> Result<LocalModelRegistrationResult, AppError> {
    let normalized_path = resolve_local_runtime_model_path(model_path)?;
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
            provider_name,
            ProviderType::LocalRuntime.as_str(),
            &normalized_path_string,
            None,
        )
        .await?
    };

    let existing_model = model_repo::list_model_definitions(&state.db)
        .await?
        .into_iter()
        .find(|model| model.provider_id == provider.id && model.name == model_name);

    let model_definition = if let Some(model) = existing_model {
        model
    } else {
        let model_id = uuid::Uuid::new_v4().to_string();
        model_repo::create_model_definition(
            &state.db,
            &model_id,
            &provider.id,
            model_name,
            context_window,
            capability_tags,
            notes,
        )
        .await?
    };

    Ok(LocalModelRegistrationResult {
        file_path: normalized_path_string,
        downloaded,
        provider,
        model_definition,
    })
}

fn endpoint_url(base_url: &str, path: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        format!("{trimmed}{path}")
    } else {
        format!("{trimmed}/v1{path}")
    }
}

#[tauri::command]
pub async fn create_provider(
    state: State<'_, AppState>,
    name: String,
    provider_type: String,
    base_url: String,
    auth_secret_ref: Option<String>,
) -> Result<ModelProvider, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let stored_secret_ref = secrets::store_provider_secret(&id, auth_secret_ref.as_deref())?;
    model_repo::create_provider(
        &state.db,
        &id,
        &name,
        &provider_type,
        &base_url,
        stored_secret_ref.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ModelProvider>, AppError> {
    model_repo::list_providers(&state.db).await
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    provider_type: Option<String>,
    base_url: Option<String>,
    auth_secret_ref: Option<String>,
    enabled: Option<bool>,
) -> Result<ModelProvider, AppError> {
    let stored_secret_ref = if let Some(secret_input) = auth_secret_ref.as_deref() {
        secrets::store_provider_secret(&id, Some(secret_input))?
    } else {
        None
    };
    model_repo::update_provider(
        &state.db,
        &id,
        name.as_deref(),
        provider_type.as_deref(),
        base_url.as_deref(),
        stored_secret_ref.as_deref(),
        enabled,
    )
    .await
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    model_repo::delete_provider(&state.db, &id).await
}

#[tauri::command]
pub async fn create_model_definition(
    state: State<'_, AppState>,
    provider_id: String,
    name: String,
    context_window: Option<i64>,
    capability_tags: Option<String>,
    notes: Option<String>,
) -> Result<ModelDefinition, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    model_repo::create_model_definition(
        &state.db,
        &id,
        &provider_id,
        &name,
        context_window,
        capability_tags.as_deref(),
        notes.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn list_model_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<ModelDefinition>, AppError> {
    model_repo::list_model_definitions(&state.db).await
}

#[tauri::command]
pub async fn update_model_definition(
    state: State<'_, AppState>,
    id: String,
    provider_id: Option<String>,
    name: Option<String>,
    context_window: Option<i64>,
    capability_tags: Option<String>,
    notes: Option<String>,
    enabled: Option<bool>,
) -> Result<ModelDefinition, AppError> {
    model_repo::update_model_definition(
        &state.db,
        &id,
        provider_id.as_deref(),
        name.as_deref(),
        context_window,
        capability_tags.as_deref(),
        notes.as_deref(),
        enabled,
    )
    .await
}

#[tauri::command]
pub async fn delete_model_definition(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    model_repo::delete_model_definition(&state.db, &id).await
}

#[tauri::command]
pub async fn test_provider_connectivity(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(&state.db, &id).await?;
    if matches!(provider.provider_type, ProviderType::LocalRuntime) {
        let model_path = resolve_local_runtime_model_path(&provider.base_url)?;
        return Ok(format!(
            "Local speech runtime is configured at {}. Whisper models transcribe audio; they do not perform speech synthesis.",
            model_path.display()
        ));
    }
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gw = OpenAiCompatibleProvider::new(provider.base_url, api_key);
    match gw.health_check().await {
        Ok(true) => Ok("Connection successful".to_string()),
        Ok(false) => Ok("Connection failed - server responded but not healthy".to_string()),
        Err(e) => Err(e),
    }
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
    provider_name: String,
    model_name: String,
    model_path: String,
    capability_tags: Option<String>,
    notes: Option<String>,
    context_window: Option<i64>,
) -> Result<LocalModelRegistrationResult, AppError> {
    upsert_local_runtime_registration(
        state.inner(),
        &provider_name,
        &model_name,
        &model_path,
        capability_tags.as_deref(),
        notes.as_deref(),
        context_window,
        false,
    )
    .await
}

#[tauri::command]
pub async fn install_managed_local_model_command(
    state: State<'_, AppState>,
    provider_name: String,
    model_name: String,
    download_url: String,
    file_name: String,
    capability_tags: Option<String>,
    notes: Option<String>,
    context_window: Option<i64>,
) -> Result<LocalModelRegistrationResult, AppError> {
    let safe_dir = slugify(&provider_name);
    let models_dir = state.app_data_dir.join("models").join(safe_dir);
    tokio::fs::create_dir_all(&models_dir).await?;
    let destination_path = models_dir.join(file_name.trim());

    let mut downloaded = false;
    if !destination_path.exists() {
        let response = reqwest::get(download_url.trim())
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
        &provider_name,
        &model_name,
        destination_path.to_str().ok_or_else(|| {
            AppError::Validation("Installed model path is not valid UTF-8".to_string())
        })?,
        capability_tags.as_deref(),
        notes.as_deref(),
        context_window,
        downloaded,
    )
    .await
}

#[tauri::command]
pub async fn run_model_chat_completion(
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
) -> Result<CompletionResponse, AppError> {
    let provider = model_repo::get_provider(&state.db, &provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gateway = OpenAiCompatibleProvider::new(provider.base_url, api_key);

    gateway
        .run_completion(CompletionRequest {
            model,
            messages,
            temperature,
            max_tokens,
        })
        .await
}

#[tauri::command]
pub async fn start_model_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(&state.db, &provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let stream_id = uuid::Uuid::new_v4().to_string();

    let base_url = provider.base_url;
    let stream_id_for_task = stream_id.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = endpoint_url(&base_url, "/chat/completions");
        let body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": temperature.unwrap_or(0.7),
            "max_tokens": max_tokens.unwrap_or(4096),
            "stream": true,
        });

        let mut req = client.post(&url).json(&body);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }

        let response = match req.send().await {
            Ok(response) => response,
            Err(error) => {
                let _ = app.emit(
                    "chat_stream_error",
                    ChatStreamErrorEvent {
                        stream_id: stream_id_for_task.clone(),
                        error: format!("Request failed: {}", error),
                    },
                );
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let _ = app.emit(
                "chat_stream_error",
                ChatStreamErrorEvent {
                    stream_id: stream_id_for_task.clone(),
                    error: format!("API error {}: {}", status, text),
                },
            );
            return;
        }

        let mut buffer = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(error) => {
                    let _ = app.emit(
                        "chat_stream_error",
                        ChatStreamErrorEvent {
                            stream_id: stream_id_for_task.clone(),
                            error: format!("Stream read failed: {}", error),
                        },
                    );
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find('\n') {
                let line = buffer[..index].to_string();
                buffer = buffer[index + 1..].to_string();
                let trimmed = line.trim();
                if !trimmed.starts_with("data:") {
                    continue;
                }
                let payload = trimmed.trim_start_matches("data:").trim();
                if payload == "[DONE]" {
                    let _ = app.emit(
                        "chat_stream_done",
                        ChatStreamDoneEvent {
                            stream_id: stream_id_for_task.clone(),
                        },
                    );
                    return;
                }

                match serde_json::from_str::<serde_json::Value>(payload) {
                    Ok(value) => {
                        if let Some(delta) = value["choices"][0]["delta"]["content"].as_str() {
                            if !delta.is_empty() {
                                let _ = app.emit(
                                    "chat_stream_chunk",
                                    ChatStreamChunkEvent {
                                        stream_id: stream_id_for_task.clone(),
                                        delta: delta.to_string(),
                                    },
                                );
                            }
                        }
                    }
                    Err(error) => {
                        warn!(stream_id = %stream_id_for_task, error = %error, "Failed to parse stream payload");
                    }
                }
            }
        }

        let _ = app.emit(
            "chat_stream_done",
            ChatStreamDoneEvent {
                stream_id: stream_id_for_task,
            },
        );
    });

    Ok(stream_id)
}

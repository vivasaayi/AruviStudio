use crate::domain::model::{ModelDefinition, ModelProvider};
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
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
) -> Result<ModelDefinition, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    model_repo::create_model_definition(&state.db, &id, &provider_id, &name, context_window).await
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
    enabled: Option<bool>,
) -> Result<ModelDefinition, AppError> {
    model_repo::update_model_definition(
        &state.db,
        &id,
        provider_id.as_deref(),
        name.as_deref(),
        context_window,
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
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gw = OpenAiCompatibleProvider::new(provider.base_url, api_key);
    match gw.health_check().await {
        Ok(true) => Ok("Connection successful".to_string()),
        Ok(false) => Ok("Connection failed - server responded but not healthy".to_string()),
        Err(e) => Err(e),
    }
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

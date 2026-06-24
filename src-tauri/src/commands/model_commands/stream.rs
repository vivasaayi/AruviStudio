use super::telemetry::{
    char_count_i64, elapsed_ms, record_model_command_call, ModelCommandCallRecord,
};
use super::ModelChatCommand;
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::secrets;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::Serialize;
use std::time::Instant;
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
pub async fn start_model_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ModelChatCommand,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(&state.db, &request.provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let stream_id = uuid::Uuid::new_v4().to_string();

    let db = state.db.clone();
    let artifact_base_path = state.artifact_base_path.clone();
    let source_kind = request
        .source_kind
        .unwrap_or_else(|| "desktop_stream".to_string());
    let telemetry_source_id = request.source_id.unwrap_or_else(|| stream_id.clone());
    let source_label = request
        .source_label
        .unwrap_or_else(|| "Desktop stream".to_string());
    let model = request.model;
    let messages = request.messages;
    let temperature = request.temperature;
    let max_tokens = request.max_tokens;
    let base_url = provider.base_url.clone();
    let stream_id_for_task = stream_id.clone();
    let telemetry_source_id_for_task = telemetry_source_id.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let mut response_chars = 0_i64;
        let mut response_text = String::new();
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
                let error_text = format!("Request failed: {}", error);
                if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                    db: &db,
                    artifact_base_path: &artifact_base_path,
                    source_kind: &source_kind,
                    source_id: Some(&telemetry_source_id_for_task),
                    source_label: &source_label,
                    provider: &provider,
                    model_name: &model,
                    messages: &messages,
                    max_tokens,
                    temperature,
                    response_chars,
                    token_count_input: None,
                    token_count_output: None,
                    duration_ms: elapsed_ms(started),
                    status: "failed",
                    error_message: Some(&error_text),
                    response_text: None,
                })
                .await
                {
                    warn!(error = %record_error, "Failed to record stream telemetry");
                }
                let _ = app.emit(
                    "chat_stream_error",
                    ChatStreamErrorEvent {
                        stream_id: stream_id_for_task.clone(),
                        error: error_text,
                    },
                );
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let error_text = format!("API error {}: {}", status, text);
            if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                db: &db,
                artifact_base_path: &artifact_base_path,
                source_kind: &source_kind,
                source_id: Some(&telemetry_source_id_for_task),
                source_label: &source_label,
                provider: &provider,
                model_name: &model,
                messages: &messages,
                max_tokens,
                temperature,
                response_chars,
                token_count_input: None,
                token_count_output: None,
                duration_ms: elapsed_ms(started),
                status: "failed",
                error_message: Some(&error_text),
                response_text: None,
            })
            .await
            {
                warn!(error = %record_error, "Failed to record stream telemetry");
            }
            let _ = app.emit(
                "chat_stream_error",
                ChatStreamErrorEvent {
                    stream_id: stream_id_for_task.clone(),
                    error: error_text,
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
                    let error_text = format!("Stream read failed: {}", error);
                    if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                        db: &db,
                        artifact_base_path: &artifact_base_path,
                        source_kind: &source_kind,
                        source_id: Some(&telemetry_source_id_for_task),
                        source_label: &source_label,
                        provider: &provider,
                        model_name: &model,
                        messages: &messages,
                        max_tokens,
                        temperature,
                        response_chars,
                        token_count_input: None,
                        token_count_output: None,
                        duration_ms: elapsed_ms(started),
                        status: "failed",
                        error_message: Some(&error_text),
                        response_text: Some(&response_text),
                    })
                    .await
                    {
                        warn!(error = %record_error, "Failed to record stream telemetry");
                    }
                    let _ = app.emit(
                        "chat_stream_error",
                        ChatStreamErrorEvent {
                            stream_id: stream_id_for_task.clone(),
                            error: error_text,
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
                    if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                        db: &db,
                        artifact_base_path: &artifact_base_path,
                        source_kind: &source_kind,
                        source_id: Some(&telemetry_source_id_for_task),
                        source_label: &source_label,
                        provider: &provider,
                        model_name: &model,
                        messages: &messages,
                        max_tokens,
                        temperature,
                        response_chars,
                        token_count_input: None,
                        token_count_output: None,
                        duration_ms: elapsed_ms(started),
                        status: "completed",
                        error_message: None,
                        response_text: Some(&response_text),
                    })
                    .await
                    {
                        warn!(error = %record_error, "Failed to record stream telemetry");
                    }
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
                                response_chars += char_count_i64(delta);
                                response_text.push_str(delta);
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
                stream_id: stream_id_for_task.clone(),
            },
        );
        if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
            db: &db,
            artifact_base_path: &artifact_base_path,
            source_kind: &source_kind,
            source_id: Some(&telemetry_source_id_for_task),
            source_label: &source_label,
            provider: &provider,
            model_name: &model,
            messages: &messages,
            max_tokens,
            temperature,
            response_chars,
            token_count_input: None,
            token_count_output: None,
            duration_ms: elapsed_ms(started),
            status: "completed",
            error_message: None,
            response_text: Some(&response_text),
        })
        .await
        {
            warn!(error = %record_error, "Failed to record stream telemetry");
        }
    });

    Ok(stream_id)
}

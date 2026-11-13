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
    let stream_id = match &request.stream_id {
        Some(id) => uuid::Uuid::parse_str(id).map_err(|_| AppError::Validation("Invalid stream ID".into()))?.to_string(),
        None => uuid::Uuid::new_v4().to_string(),
    };
    if provider.protocol == "anthropic" {
        return start_anthropic_stream(app, state, provider, request, stream_id).await;
    }
    if provider.protocol != "openai_compatible" {
        return Err(AppError::Validation(format!("Unsupported provider protocol: {}", provider.protocol)));
    }
    let api_key = secrets::resolve_provider_secret(&provider)?;

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


async fn start_anthropic_stream<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    provider: crate::domain::model::ModelProvider,
    request: ModelChatCommand,
    stream_id: String,
) -> Result<String, AppError> {
    if provider.auth_secret_ref.as_deref().is_none_or(|value| value.trim().is_empty()) {
        return Err(AppError::Validation("An explicit Anthropic API key is required".into()));
    }
    let key = secrets::resolve_provider_secret(&provider)?
        .ok_or_else(|| AppError::Validation("Anthropic API key is required".into()))?;
    let client = aruvi_model_client::anthropic::AnthropicClient::new(provider.base_url.clone(), key)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let task_id = stream_id.clone();
    let source_id = request.source_id.unwrap_or_else(|| stream_id.clone());
    let source_kind = request.source_kind.unwrap_or_else(|| "desktop_stream".into());
    let source_label = request.source_label.unwrap_or_else(|| "Desktop stream".into());
    let db = state.db.clone();
    let artifact_base_path = state.artifact_base_path.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let mut response_text = String::new();
        let result = client.stream_completion(
            crate::providers::types::CompletionRequest {
                model: request.model.clone(), messages: request.messages.clone(),
                temperature: request.temperature, max_tokens: request.max_tokens,
            },
            |delta| {
                response_text.push_str(delta);
                let _ = app.emit("chat_stream_chunk", ChatStreamChunkEvent { stream_id: task_id.clone(), delta: delta.to_string() });
            },
        ).await;
        let (status, error_message, input_tokens, output_tokens) = match &result {
            Ok(response) => ("completed", None, response.token_count_input, response.token_count_output),
            Err(error) => ("failed", Some(error.to_string()), None, None),
        };
        if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
            db: &db, artifact_base_path: &artifact_base_path, source_kind: &source_kind,
            source_id: Some(&source_id), source_label: &source_label, provider: &provider,
            model_name: &request.model, messages: &request.messages,
            max_tokens: request.max_tokens, temperature: request.temperature,
            response_chars: char_count_i64(&response_text), token_count_input: input_tokens,
            token_count_output: output_tokens, duration_ms: elapsed_ms(started), status,
            error_message: error_message.as_deref(), response_text: Some(&response_text),
        }).await { warn!(error = %record_error, "Failed to record stream telemetry"); }
        match error_message {
            Some(error) => { let _ = app.emit("chat_stream_error", ChatStreamErrorEvent { stream_id: task_id, error }); }
            None => { let _ = app.emit("chat_stream_done", ChatStreamDoneEvent { stream_id: task_id }); }
        }
    });
    Ok(stream_id)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_helpers::make_test_app;
    use crate::domain::model::{ModelProvider, ProviderType};
    use crate::providers::types::ChatMessage;
    use axum::{routing::post, Json};
    use serde_json::Value;
    use tauri::{Listener, Manager};

    #[tokio::test]
    async fn anthropic_desktop_stream_emits_incremental_events_and_records_usage() {
        // A real HTTP fixture exercises the desktop adapter as well as the shared parser.
        async fn endpoint(headers: axum::http::HeaderMap, Json(body): Json<Value>) -> impl axum::response::IntoResponse {
            assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-synthetic-key-123456");
            assert_eq!(body["stream"], true);
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], concat!(
                "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":4}}}\n\n",
                "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
                "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\" there\"}}\n\n",
                "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":2}}\n\n",
                "data: {\"type\":\"message_stop\"}\n\n"))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, axum::Router::new().route("/v1/messages", post(endpoint))).await.unwrap(); });
        let app = make_test_app("anthropic_desktop_stream").await;
        let provider = ModelProvider { id: "anthropic-fixture".into(), name: "Fixture".into(),
            provider_type: ProviderType::OpenaiCompatible, protocol: "anthropic".into(),
            base_url: format!("http://{address}"), auth_secret_ref: Some("sk-test-synthetic-key-123456".into()),
            enabled: true, created_at: "".into(), updated_at: "".into() };
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
        for event_name in ["chat_stream_chunk", "chat_stream_done", "chat_stream_error"] {
            let sender = sender.clone();
            app.listen_any(event_name, move |event| { sender.send((event_name.into(), serde_json::from_str(event.payload()).unwrap())).unwrap(); });
        }
        let id = uuid::Uuid::new_v4().to_string();
        let request = ModelChatCommand { stream_id: Some(id.clone()), provider_id: provider.id.clone(), model: "claude-test".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "Hello".into() }], temperature: None, max_tokens: Some(20),
            source_kind: None, source_id: None, source_label: None };
        start_anthropic_stream(app.handle().clone(), app.state::<AppState>(), provider, request, id.clone()).await.unwrap();
        let mut text = String::new();
        loop {
            let (kind, payload) = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv()).await.unwrap().unwrap();
            assert_eq!(payload["stream_id"], id);
            if kind == "chat_stream_done" { break; }
            assert_eq!(kind, "chat_stream_chunk", "{payload}");
            text.push_str(payload["delta"].as_str().unwrap());
        }
        assert_eq!(text, "Hello there");
        let row: (String, Option<i64>, Option<i64>) = sqlx::query_as("SELECT status, token_count_input, token_count_output FROM model_calls WHERE source_id=?")
            .bind(&id).fetch_one(&app.state::<AppState>().db).await.unwrap();
        assert_eq!(row, ("completed".into(), Some(4), Some(2)));
        server.abort();
    }
}

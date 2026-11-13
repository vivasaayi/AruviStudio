use super::telemetry::{
    char_count_i64, elapsed_ms, record_model_command_call, ModelCommandCallRecord,
};
use super::ModelChatCommand;
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::state::AppState;
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
    start_provider_stream(app, state, provider, request, stream_id).await
}

async fn start_provider_stream<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    provider: crate::domain::model::ModelProvider,
    request: ModelChatCommand,
    stream_id: String,
) -> Result<String, AppError> {
    let client = crate::services::model_service::create_shared_gateway(&provider)?;
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
            &mut |delta| {
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
        start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider, request, id.clone()).await.unwrap();
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

    #[tokio::test]
    async fn openai_desktop_stream_emits_incremental_events_and_records_usage() {
        // A real HTTP fixture exercises the desktop adapter as well as the shared parser.
        async fn endpoint(headers: axum::http::HeaderMap, Json(body): Json<Value>) -> impl axum::response::IntoResponse {
            assert_eq!(headers.get("authorization").unwrap(), "Bearer sk-test-synthetic-key-123456");
            assert_eq!(body["stream"], true);
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\" there\"}}]}\n\n",
                "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\n",
                "data: [DONE]\n\n"))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, axum::Router::new().route("/v1/chat/completions", post(endpoint))).await.unwrap(); });
        let app = make_test_app("openai_desktop_stream").await;
        let provider = ModelProvider { id: "anthropic-fixture".into(), name: "Fixture".into(),
            provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(),
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
        start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider, request, id.clone()).await.unwrap();
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

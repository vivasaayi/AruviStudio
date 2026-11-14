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
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<aruvi_model_client::CompletionMetadata>,
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

#[tauri::command]
pub async fn cancel_model_chat_stream(state: State<'_, AppState>, stream_id: String) -> Result<bool, AppError> {
    let id = uuid::Uuid::parse_str(&stream_id).map_err(|_| AppError::Validation("Invalid stream ID".into()))?.to_string();
    Ok(cancel_stream(&state.chat_stream_cancellations, &id).await)
}

async fn cancel_stream(
    registry: &tokio::sync::Mutex<std::collections::HashMap<String, Option<tokio::sync::oneshot::Sender<()>>>>,
    id: &str,
) -> bool {
    registry.lock().await.get_mut(id).and_then(Option::take).is_some_and(|sender| sender.send(()).is_ok())
}

async fn start_provider_stream<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    provider: crate::domain::model::ModelProvider,
    request: ModelChatCommand,
    stream_id: String,
) -> Result<String, AppError> {
    let client = crate::services::model_service::create_shared_gateway(&provider)?;
    let (cancel_sender, cancel_receiver) = tokio::sync::oneshot::channel();
    let registry = state.chat_stream_cancellations.clone();
    {
        let mut streams = registry.lock().await;
        if streams.contains_key(&stream_id) { return Err(AppError::Validation("Stream ID is already active".into())); }
        streams.insert(stream_id.clone(), Some(cancel_sender));
    }
    let task_id = stream_id.clone();
    let source_id = request.source_id.unwrap_or_else(|| stream_id.clone());
    let source_kind = request.source_kind.unwrap_or_else(|| "desktop_stream".into());
    let source_label = request.source_label.unwrap_or_else(|| "Desktop stream".into());
    let db = state.db.clone();
    let artifact_base_path = state.artifact_base_path.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let mut response_text = String::new();
        let result = {
            let mut on_delta = |delta: &str| {
                response_text.push_str(delta);
                let _ = app.emit("chat_stream_chunk", ChatStreamChunkEvent { stream_id: task_id.clone(), delta: delta.to_string() });
            };
            let future = client.stream_completion(
                crate::providers::types::CompletionRequest {
                    model: request.model.clone(), messages: request.messages.clone(),
                    temperature: request.temperature, max_tokens: request.max_tokens,
                }, &mut on_delta,
            );
            tokio::pin!(future);
            tokio::select! {
                result = &mut future => Some(result),
                _ = cancel_receiver => None,
            }
        };
        let (status, error_message, input_tokens, output_tokens) = match &result {
            Some(Ok(response)) => ("completed", None, response.token_count_input, response.token_count_output),
            Some(Err(error)) => ("failed", Some(error.to_string()), None, None),
            None => ("failed", Some("Stream cancelled by user".into()), None, None),
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
            Some(error) => { let _ = app.emit("chat_stream_error", ChatStreamErrorEvent { stream_id: task_id.clone(), error }); }
            None => { let _ = app.emit("chat_stream_done", ChatStreamDoneEvent { stream_id: task_id.clone(), metadata: result.and_then(Result::ok).and_then(|response| response.metadata) }); }
        }
        registry.lock().await.remove(&task_id);
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
    async fn cancellation_closes_upstream_and_records_incomplete_stream() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(std::time::Duration::from_secs(8))).unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 4096];
            let header_end = loop {
                let count = socket.read(&mut buffer).unwrap(); assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") { break index + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]).to_ascii_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < header_end + length {
                let count = socket.read(&mut buffer).unwrap(); assert_ne!(count, 0); bytes.extend_from_slice(&buffer[..count]);
            }
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 100000\r\nConnection: close\r\n\r\ndata: {{\"choices\":[{{\"delta\":{{\"content\":\"partial\"}}}}]}}\n\n").unwrap();
            match socket.read(&mut buffer) { Ok(0) => true, Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => true, _ => false }
        });
        let app = make_test_app("cancel_desktop_stream").await;
        let provider = ModelProvider { id: "cancel-fixture".into(), name: "Fixture".into(),
            provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(),
            base_url: format!("http://{address}"), auth_secret_ref: None,
            enabled: true, created_at: "".into(), updated_at: "".into() };
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
        for event_name in ["chat_stream_chunk", "chat_stream_done", "chat_stream_error"] {
            let sender = sender.clone();
            app.listen_any(event_name, move |event| { let _ = sender.send((event_name.into(), serde_json::from_str(event.payload()).unwrap())); });
        }
        let id = uuid::Uuid::new_v4().to_string();
        let request = || ModelChatCommand { stream_id: Some(id.clone()), provider_id: provider.id.clone(), model: "fixture".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "Hello".into() }], temperature: None, max_tokens: Some(20),
            source_kind: None, source_id: None, source_label: None };
        start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider.clone(), request(), id.clone()).await.unwrap();
        let (kind, payload) = tokio::time::timeout(std::time::Duration::from_secs(8), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(kind, "chat_stream_chunk");
        assert_eq!(payload["delta"], "partial");
        assert!(start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider.clone(), request(), id.clone()).await.is_err());
        let registry = app.state::<AppState>().chat_stream_cancellations.clone();
        assert!(cancel_stream(&registry, &id).await);
        assert!(!cancel_stream(&registry, &id).await);
        let (kind, payload) = tokio::time::timeout(std::time::Duration::from_secs(8), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(kind, "chat_stream_error");
        assert_eq!(payload["error"], "Stream cancelled by user");
        let row: (String, i64) = sqlx::query_as("SELECT status, response_chars FROM model_calls WHERE source_id=?")
            .bind(&id).fetch_one(&app.state::<AppState>().db).await.unwrap();
        assert_eq!(row, ("failed".into(), 7));
        assert!(tokio::task::spawn_blocking(move || server.join().unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn anthropic_desktop_stream_emits_incremental_events_and_records_usage() {
        // A real HTTP fixture exercises the desktop adapter as well as the shared parser.
        async fn endpoint(headers: axum::http::HeaderMap, Json(body): Json<Value>) -> impl axum::response::IntoResponse {
            assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-synthetic-key-123456");
            assert_eq!(body["stream"], true);
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], concat!(
                "data: {\"type\":\"message_start\",\"message\":{\"id\":\"stream-id\",\"usage\":{\"input_tokens\":4}}}\n\n",
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
            if kind == "chat_stream_done" {
                assert_eq!(payload["metadata"]["response_id"], "stream-id");
                break;
            }
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
    async fn anthropic_server_tool_stream_emits_error_and_records_failure() {
        async fn endpoint() -> impl axum::response::IntoResponse {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], concat!(
                "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n",
                "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"server_tool_use\"}}\n\n",
                "data: {\"type\":\"message_stop\"}\n\n"))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, axum::Router::new().route("/v1/messages", post(endpoint))).await.unwrap(); });
        let app = make_test_app("anthropic_server_tool_desktop_stream").await;
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
        let (kind, payload) = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(kind, "chat_stream_chunk");
        assert_eq!(payload["delta"], "partial");
        let (kind, payload) = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(kind, "chat_stream_error");
        assert!(payload["error"].as_str().unwrap().contains("tool-use"));
        let status: String = sqlx::query_scalar("SELECT status FROM model_calls WHERE source_id=?")
            .bind(&id).fetch_one(&app.state::<AppState>().db).await.unwrap();
        assert_eq!(status, "failed");
        server.abort();
    }

    #[tokio::test]
    async fn openai_desktop_stream_emits_incremental_events_and_records_usage() {
        // A real HTTP fixture exercises the desktop adapter as well as the shared parser.
        async fn endpoint(headers: axum::http::HeaderMap, Json(body): Json<Value>) -> impl axum::response::IntoResponse {
            assert_eq!(headers.get("authorization").unwrap(), "Bearer sk-test-synthetic-key-123456");
            assert_eq!(body["stream"], true);
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], concat!(
                "data: {\"id\":\"stream-id\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
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
            if kind == "chat_stream_done" {
                assert_eq!(payload["metadata"]["response_id"], "stream-id");
                break;
            }
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

use super::telemetry::{
    char_count_i64, elapsed_ms, record_model_command_call, ModelCommandCallRecord,
};
use super::ModelChatCommand;
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_http_status: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct DesktopCopilotFile {
    path: String,
    language: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct DesktopCopilotCommand {
    stream_id: String,
    provider_id: String,
    model: String,
    history: Vec<crate::providers::types::ChatMessage>,
    system_prompt: String,
    patch_mode: bool,
    file: Option<DesktopCopilotFile>,
    context_budget: usize,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
    source_id: String,
}

fn desktop_copilot_request(input: DesktopCopilotCommand) -> Result<ModelChatCommand, AppError> {
    if input.context_budget > 200_000 || input.system_prompt.len() > 16_000 || input.file.as_ref().is_some_and(|file| file.content.len() > 2 * 1024 * 1024 || file.path.len() > 4096 || file.language.len() > 256) {
        return Err(AppError::Validation("Copilot context exceeds its size limit".into()));
    }
    let budget = if input.context_budget > 1000 { input.context_budget } else { 12_000 };
    let mut messages = vec![crate::providers::types::ChatMessage { role: "system".into(), content: if input.system_prompt.trim().is_empty() { "You are Aruvi Copilot.".into() } else { input.system_prompt.trim().to_string() } }];
    if input.patch_mode {
        messages.push(crate::providers::types::ChatMessage { role: "system".into(), content: "Return ONLY JSON with this schema:\n{\n  \"type\": \"patch_proposal\",\n  \"summary\": \"short summary\",\n  \"patches\": [{\n    \"path\": \"relative/path/from/repo/root\",\n    \"base_sha256\": null,\n    \"description\": \"why this patch exists\",\n    \"patch\": \"@@ -oldStart,oldCount +newStart,newCount @@\\n context/removals/additions ...\"\n  }]\n}\nRules:\n- No markdown, no prose outside JSON.\n- Use unified diff hunks in \"patch\" only.\n- Keep patch list minimal and precise.".into() });
    }
    if let Some(file) = input.file {
        let chars: Vec<char> = file.content.chars().collect();
        let content = if chars.len() <= budget { file.content } else {
            let head = budget * 7 / 10;
            let tail = budget - head;
            format!("{}\n\n...<truncated for context budget>...\n\n{}", chars[..head].iter().collect::<String>(), chars[chars.len()-tail..].iter().collect::<String>())
        };
        messages.push(crate::providers::types::ChatMessage { role: "system".into(), content: format!("Active file context\nPath: {}\nLanguage: {}\n\n{}", file.path, file.language, content) });
    }
    messages.extend(input.history);
    Ok(ModelChatCommand { stream_id: Some(input.stream_id), provider_id: input.provider_id, model: input.model, messages, temperature: input.temperature, max_tokens: input.max_tokens, source_kind: Some("desktop_ide".into()), source_id: Some(input.source_id), source_label: Some("Desktop IDE".into()) })
}

#[tauri::command]
pub async fn start_desktop_copilot_stream(app: AppHandle, state: State<'_, AppState>, request: DesktopCopilotCommand) -> Result<String, AppError> {
    start_model_chat_stream(app, state, desktop_copilot_request(request)?).await
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
    crate::services::model_service::validate_text_model_name(&request.model)?;
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
        let provider_http_status = result.as_ref().and_then(|result| result.as_ref().err()).and_then(|error| error.http_status());
        match error_message {
            Some(error) => { let _ = app.emit("chat_stream_error", ChatStreamErrorEvent { stream_id: task_id.clone(), error, provider_http_status }); }
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

    #[test]
    fn desktop_copilot_builds_bounded_file_and_patch_messages_in_rust() {
        let input = serde_json::json!({"stream_id": "stream", "provider_id": "provider", "model": "model", "history": [{"role":"user","content":"Explain"}], "system_prompt":" custom ", "patch_mode":true, "file":{"path":"file.rs","language":"rust","content": format!("HEAD{}TAIL", "é".repeat(2000))}, "context_budget":1100, "temperature":0.2, "max_tokens":100, "source_id":"session"});
        let result = desktop_copilot_request(serde_json::from_value(input).unwrap()).unwrap();
        assert_eq!(result.messages[0].content, "custom");
        assert!(result.messages[1].content.contains("patch_proposal"));
        assert!(result.messages[2].content.contains("HEAD"));
        assert!(result.messages[2].content.contains("TAIL"));
        assert!(result.messages[2].content.contains("truncated for context budget"));
        assert_eq!(result.messages.last().unwrap().content, "Explain");
        assert_eq!(result.source_kind.as_deref(), Some("desktop_ide"));
        assert_eq!(result.source_id.as_deref(), Some("session"));
        let oversized = serde_json::json!({"stream_id":"s","provider_id":"p","model":"m","history":[],"system_prompt":"","patch_mode":false,"file":null,"context_budget":200001,"temperature":null,"max_tokens":null,"source_id":"x"});
        assert!(desktop_copilot_request(serde_json::from_value(oversized).unwrap()).is_err());
    }

    #[tokio::test]
    async fn desktop_copilot_structured_request_streams_and_records_telemetry() {
        async fn endpoint(Json(body): Json<Value>) -> impl axum::response::IntoResponse {
            assert_eq!(body["stream"], true);
            assert!(body["messages"].as_array().unwrap().iter().any(|message| message["content"].as_str().unwrap_or("").contains("unsaved fixture")));
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], "data: {\"choices\":[{\"delta\":{\"content\":\"reply\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n")
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, axum::Router::new().route("/v1/chat/completions", post(endpoint))).await.unwrap(); });
        let app = make_test_app("desktop_copilot_structured").await;
        let provider = ModelProvider { id: "copilot-fixture".into(), name: "Fixture".into(), provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: format!("http://{address}"), auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
        for event_name in ["chat_stream_chunk", "chat_stream_done", "chat_stream_error"] {
            let sender = sender.clone();
            app.listen_any(event_name, move |event| { let _ = sender.send((event_name.into(), serde_json::from_str(event.payload()).unwrap())); });
        }
        let id = uuid::Uuid::new_v4().to_string();
        let input = serde_json::json!({"stream_id":id,"provider_id":provider.id,"model":"fixture","history":[{"role":"user","content":"Explain"}],"system_prompt":"","patch_mode":false,"file":{"path":"file.rs","language":"rust","content":"unsaved fixture"},"context_budget":12000,"temperature":0.2,"max_tokens":100,"source_id":"copilot-session"});
        let request = desktop_copilot_request(serde_json::from_value(input).unwrap()).unwrap();
        start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider, request, id.clone()).await.unwrap();
        let mut text = String::new();
        loop {
            let (event, payload) = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv()).await.unwrap().unwrap();
            assert_eq!(payload["stream_id"], id);
            if event == "chat_stream_done" { break; }
            assert_eq!(event, "chat_stream_chunk"); text.push_str(payload["delta"].as_str().unwrap());
        }
        assert_eq!(text, "reply");
        let row: (String, String) = sqlx::query_as("SELECT source_kind,status FROM model_calls WHERE source_id=?").bind("copilot-session").fetch_one(&app.state::<AppState>().db).await.unwrap();
        assert_eq!(row, ("desktop_ide".into(), "completed".into()));
        server.abort();
    }

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
            provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(),
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
        assert!(payload.get("provider_http_status").is_none());
        let row: (String, i64) = sqlx::query_as("SELECT status, response_chars FROM model_calls WHERE source_id=?")
            .bind(&id).fetch_one(&app.state::<AppState>().db).await.unwrap();
        assert_eq!(row, ("failed".into(), 7));
        assert!(tokio::task::spawn_blocking(move || server.join().unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn desktop_stream_reports_provider_http_status_without_body() {
        async fn endpoint() -> impl axum::response::IntoResponse {
            (axum::http::StatusCode::TOO_MANY_REQUESTS, "synthetic-provider-private-body")
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, axum::Router::new().route("/v1/chat/completions", post(endpoint))).await.unwrap(); });
        let app = make_test_app("desktop_provider_status").await;
        let provider = ModelProvider { id: "status-fixture".into(), name: "Fixture".into(),
            provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(),
            base_url: format!("http://{address}"), auth_secret_ref: None,
            enabled: true, created_at: "".into(), updated_at: "".into() };
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Value>();
        app.listen_any("chat_stream_error", move |event| { let _ = sender.send(serde_json::from_str(event.payload()).unwrap()); });
        let id = uuid::Uuid::new_v4().to_string();
        let request = ModelChatCommand { stream_id: Some(id.clone()), provider_id: provider.id.clone(), model: "fixture".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "Hello".into() }], temperature: None, max_tokens: Some(20),
            source_kind: None, source_id: None, source_label: None };
        start_provider_stream(app.handle().clone(), app.state::<AppState>(), provider, request, id.clone()).await.unwrap();
        let payload = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(payload["stream_id"], id);
        assert_eq!(payload["provider_http_status"], 429);
        assert!(!payload.to_string().contains("synthetic-provider-private-body"));
        server.abort();
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
            provider_type: ProviderType::OpenaiCompatible, protocol: "anthropic".into(), completion_profile: "standard".into(),
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
            provider_type: ProviderType::OpenaiCompatible, protocol: "anthropic".into(), completion_profile: "standard".into(),
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
            provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(),
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

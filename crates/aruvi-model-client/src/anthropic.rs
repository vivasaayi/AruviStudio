//! Anthropic Messages API client for text conversations.
use crate::{ClientError, CompletionRequest, CompletionResponse, ModelGateway};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

const API_VERSION: &str = "2023-06-01";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self, ClientError> {
        Self::with_options(base_url, api_key, crate::ClientOptions::default())
    }

    /// Construct a client with explicitly bounded connection and total request timeouts.
    pub fn with_options(base_url: String, api_key: String, options: crate::ClientOptions) -> Result<Self, ClientError> {
        if api_key.trim().is_empty() { return Err(ClientError::InvalidInput("Anthropic API key is required".into())); }
        let url = reqwest::Url::parse(&base_url).map_err(|_| ClientError::InvalidInput("Invalid Anthropic base URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err(ClientError::InvalidInput("Invalid Anthropic base URL".into()));
        }
        let client = crate::http::build_client(options)?;
        Ok(Self { base_url, api_key, client })
    }

    fn endpoint(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") { format!("{base}{path}") } else { format!("{base}/v1{path}") }
    }

    // Text-only requests cannot represent tool calls or their results, even
    // when a provider completes a server-side tool and reports end_turn.
    fn reject_tool_block(block: &Value) -> Result<(), ClientError> {
        let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
        if matches!(kind, "tool_use" | "server_tool_use") || kind.ends_with("_tool_result") {
            return Err(ClientError::Provider("Anthropic tool-use responses are not supported by the text client".into()));
        }
        Ok(())
    }

    // Some compatible proxies omit stop_reason. When supplied, never let a
    // partial, paused, refused or unsupported tool turn appear complete.
    fn validate_stop_reason(reason: Option<&Value>) -> Result<(), ClientError> {
        match reason {
            None | Some(Value::Null) => Ok(()),
            Some(Value::String(reason)) => match reason.as_str() {
                "end_turn" | "stop_sequence" => Ok(()),
                "max_tokens" | "model_context_window_exceeded" => Err(ClientError::Provider("Anthropic response was truncated by a token limit".into())),
                "pause_turn" => Err(ClientError::Provider("Anthropic response requires continuation".into())),
                "refusal" => Err(ClientError::Provider("Anthropic refused the request".into())),
                "tool_use" => Err(ClientError::Provider("Anthropic tool-use responses are not supported by the text client".into())),
                _ => Err(ClientError::Provider("Anthropic returned an unsupported stop reason".into())),
            },
            Some(_) => Err(ClientError::Provider("Invalid Anthropic stop reason".into())),
        }
    }

    fn request_body(request: CompletionRequest) -> Result<Value, ClientError> {
        if request.model.trim().is_empty() || request.max_tokens.is_some_and(|value| value <= 0) {
            return Err(ClientError::InvalidInput("A model and positive max_tokens are required".into()));
        }
        let mut system = Vec::new();
        let mut messages = Vec::new();
        for message in request.messages {
            match message.role.as_str() {
                "system" if messages.is_empty() => system.push(message.content),
                "user" | "assistant" => messages.push(json!({"role":message.role,"content":message.content})),
                "system" => return Err(ClientError::InvalidInput("System messages must precede conversation messages for Anthropic".into())),
                _ => return Err(ClientError::InvalidInput(format!("Unsupported Anthropic message role: {}", message.role))),
            }
        }
        if messages.is_empty() { return Err(ClientError::InvalidInput("At least one conversation message is required".into())); }
        let mut body = json!({"model":request.model,"max_tokens":request.max_tokens.unwrap_or(4096),"messages":messages});
        if !system.is_empty() { body["system"] = Value::String(system.join("\n\n")); }
        if let Some(temperature) = request.temperature {
            if !temperature.is_finite() || !(0.0..=1.0).contains(&temperature) { return Err(ClientError::InvalidInput("Anthropic temperature must be between 0 and 1".into())); }
            body["temperature"] = json!(temperature);
        }
        Ok(body)
    }

    /// Streams text deltas and returns the final text and usage. Dropping the
    /// future cancels the request; no background task is spawned by the client.
    pub async fn stream_completion<F>(
        &self,
        request: CompletionRequest,
        mut on_delta: F,
    ) -> Result<CompletionResponse, ClientError>
    where
        F: FnMut(&str) + Send,
    {
        let mut body = Self::request_body(request)?;
        body["stream"] = Value::Bool(true);
        let response = self.client.post(self.endpoint("/messages"))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body).send().await
            .map_err(|error| ClientError::Provider(format!("Anthropic stream request failed: {error}")))?;
        if !response.status().is_success() {
            return Err(ClientError::Provider(format!("Anthropic API error {}", response.status())));
        }
        let mut state = StreamState::default();
        crate::sse::read_sse(response, "Anthropic", |data| {
            let event: Value = serde_json::from_slice(data).map_err(|_| ClientError::Provider("Invalid Anthropic stream event".into()))?;
            state.event(&event, &mut on_delta)?;
            Ok(state.stopped)
        }).await?;
        state.finish()
    }

    async fn read_json(mut response: reqwest::Response) -> Result<Value, ClientError> {
        if !response.status().is_success() {
            return Err(ClientError::Provider(format!("Anthropic API error {}", response.status())));
        }
        if response.content_length().is_some_and(|length| length > MAX_RESPONSE_BYTES as u64) {
            return Err(ClientError::Provider("Anthropic response exceeds size limit".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| ClientError::Provider(format!("Anthropic response failed: {error}")))? {
            if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() { return Err(ClientError::Provider("Anthropic response exceeds size limit".into())); }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| ClientError::Provider("Invalid Anthropic JSON response".into()))
    }
}

#[derive(Default)]
struct StreamState {
    content: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    stopped: bool,
    metadata: Option<crate::CompletionMetadata>,
}

impl StreamState {
    fn event<F: FnMut(&str)>(&mut self, event: &Value, on_delta: &mut F) -> Result<(), ClientError> {
        match event.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(message) = event.get("message") { crate::CompletionMetadata::merge(&mut self.metadata, message, message.get("stop_reason")); }
                self.input_tokens = event.pointer("/message/usage/input_tokens").and_then(Value::as_i64);
                self.output_tokens = event.pointer("/message/usage/output_tokens").and_then(Value::as_i64);
            }
            Some("message_delta") => {
                AnthropicClient::validate_stop_reason(event.pointer("/delta/stop_reason"))?;
                crate::CompletionMetadata::merge(&mut self.metadata, event, event.pointer("/delta/stop_reason"));
                self.input_tokens = event.pointer("/usage/input_tokens").and_then(Value::as_i64).or(self.input_tokens);
                self.output_tokens = event.pointer("/usage/output_tokens").and_then(Value::as_i64).or(self.output_tokens);
            }
            Some("content_block_start") => {
                if let Some(block) = event.get("content_block") { AnthropicClient::reject_tool_block(block)?; }
                if event.pointer("/content_block/type").and_then(Value::as_str) == Some("text") {
                    if let Some(text) = event.pointer("/content_block/text").and_then(Value::as_str).filter(|text| !text.is_empty()) {
                        self.content.push_str(text); on_delta(text);
                    }
                }
            }
            Some("content_block_delta") if event.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") => {
                let text = event.pointer("/delta/text").and_then(Value::as_str).ok_or_else(|| ClientError::Provider("Invalid Anthropic text delta".into()))?;
                self.content.push_str(text); on_delta(text);
            }
            Some("error") => return Err(ClientError::Provider("Anthropic stream returned an error".into())),
            Some("message_stop") => self.stopped = true,
            _ => {} // Ping and future event types may be safely ignored.
        }
        Ok(())
    }

    fn finish(self) -> Result<CompletionResponse, ClientError> {
        if self.content.is_empty() { return Err(ClientError::Provider("Anthropic returned no text content".into())); }
        Ok(CompletionResponse { content: self.content, token_count_input: self.input_tokens, token_count_output: self.output_tokens, metadata: self.metadata })
    }
}

#[async_trait]
impl ModelGateway for AnthropicClient {
    fn transport_capabilities(&self) -> crate::TransportCapabilities {
        crate::TransportCapabilities { text_completion: true, text_streaming: true, model_discovery: true, transcription: false, speech: false }
    }
    async fn stream_completion(&self, request: CompletionRequest, on_delta: &mut (dyn for<'a> FnMut(&'a str) + Send)) -> Result<CompletionResponse, ClientError> {
        AnthropicClient::stream_completion(self, request, |delta: &str| on_delta(delta)).await
    }
    async fn health_check(&self) -> Result<bool, ClientError> {
        let response = self.client.get(self.endpoint("/models")).header("x-api-key", &self.api_key).header("anthropic-version", API_VERSION).send().await
            .map_err(|error| ClientError::Provider(format!("Anthropic health check failed: {error}")))?;
        Ok(response.status().is_success())
    }

    async fn list_models_page(&self, cursor: Option<&str>) -> Result<crate::ModelPage, ClientError> {
        if cursor.is_some_and(|value| value.trim().is_empty() || value.len() > 1024) {
            return Err(ClientError::InvalidInput("Invalid model discovery cursor".into()));
        }
        let mut request = self.client.get(self.endpoint("/models"))
            .header("x-api-key", &self.api_key).header("anthropic-version", API_VERSION);
        if let Some(cursor) = cursor { request = request.query(&[("after_id", cursor)]); }
        let response = request.send().await.map_err(|error| ClientError::Provider(format!("Anthropic discovery request failed: {error}")))?;
        let body = crate::discovery::read_response(response).await?;
        let models = crate::discovery::parse_models(&body)?;
        let has_more = body.get("has_more").and_then(Value::as_bool)
            .ok_or_else(|| ClientError::Provider("Anthropic discovery is missing pagination state".into()))?;
        let next_cursor = if has_more {
            let next = body.get("last_id").and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty() && value.len() <= 1024 && Some(*value) != cursor)
                .ok_or_else(|| ClientError::Provider("Anthropic discovery returned an invalid next cursor".into()))?;
            Some(next.to_owned())
        } else { None };
        Ok(crate::ModelPage { models, next_cursor })
    }

    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
        let body = Self::request_body(request)?;
        let response = self.client.post(self.endpoint("/messages")).header("x-api-key", &self.api_key).header("anthropic-version", API_VERSION).json(&body).send().await
            .map_err(|error| ClientError::Provider(format!("Anthropic request failed: {error}")))?;
        let json = Self::read_json(response).await?;
        Self::validate_stop_reason(json.get("stop_reason"))?;
        let blocks = json.get("content").and_then(Value::as_array).ok_or_else(|| ClientError::Provider("Anthropic response is missing content".into()))?;
        for block in blocks { Self::reject_tool_block(block)?; }
        let content = blocks.iter().filter(|block| block.get("type").and_then(Value::as_str) == Some("text")).filter_map(|block| block.get("text").and_then(Value::as_str)).collect::<String>();
        if content.is_empty() { return Err(ClientError::Provider("Anthropic returned no text content".into())); }
        let mut metadata = None;
        crate::CompletionMetadata::merge(&mut metadata, &json, json.get("stop_reason"));
        Ok(CompletionResponse { content, token_count_input: json.pointer("/usage/input_tokens").and_then(Value::as_i64), token_count_output: json.pointer("/usage/output_tokens").and_then(Value::as_i64), metadata })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;
    use std::{io::{Read, Write}, net::TcpListener, thread};

    fn message(role: &str, content: &str) -> ChatMessage { ChatMessage { role: role.into(), content: content.into() } }

    #[tokio::test]
    async fn maps_system_messages_authentication_and_usage() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new(); let mut buf = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buf[..n]);
                if let Some(i) = data.windows(4).position(|part| part == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
            assert!(headers.starts_with("post /v1/messages "));
            assert!(headers.contains("x-api-key: test-key"));
            assert!(headers.contains("anthropic-version: 2023-06-01"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < end + length { let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buf[..n]); }
            let body: Value = serde_json::from_slice(&data[end..end + length]).unwrap();
            assert_eq!(body["system"], "First\n\nSecond");
            assert_eq!(body["messages"], json!([{"role":"user","content":"Hello"}]));
            assert_eq!(body["max_tokens"], 20);
            let reply = r#"{"id":"msg-1","model":"fixture-v1","stop_reason":"end_turn","content":[{"type":"text","text":"Hello"},{"type":"text","text":" there"}],"usage":{"input_tokens":5,"output_tokens":2,"cache_read_input_tokens":4}}"#;
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",reply.len(),reply).unwrap();
        });
        let client = AnthropicClient::new(format!("http://{addr}/v1"), "test-key".into()).unwrap();
        let response = client.run_completion(CompletionRequest { model:"fixture".into(), messages:vec![message("system","First"),message("system","Second"),message("user","Hello")], temperature:Some(0.2), max_tokens:Some(20) }).await.unwrap();
        assert_eq!(response.content, "Hello there");
        let meta = response.metadata.as_ref().unwrap();
        assert_eq!((meta.response_id.as_deref(), meta.model.as_deref(), meta.finish_reason.as_deref()), (Some("msg-1"), Some("fixture-v1"), Some("end_turn")));
        assert_eq!(meta.usage.as_ref().unwrap()["cache_read_input_tokens"], 4);
        assert_eq!((response.token_count_input,response.token_count_output), (Some(5),Some(2)));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_unrepresentable_conversation_without_sending_request() {
        let client = AnthropicClient::new("http://127.0.0.1:1".into(), "test-key".into()).unwrap();
        let request = CompletionRequest { model:"fixture".into(), messages:vec![message("user","Hello"),message("system","late")], temperature:None, max_tokens:Some(1) };
        assert!(matches!(client.run_completion(request).await, Err(ClientError::InvalidInput(_))));
        assert!(AnthropicClient::new("https://user:secret@example.invalid".into(), "key".into()).is_err());
        assert!(AnthropicClient::new("https://example.invalid".into(), "  ".into()).is_err());
    }

    #[tokio::test]
    async fn refuses_redirect_without_forwarding_api_key() {
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let target_addr = target.local_addr().unwrap();
        let source = TcpListener::bind("127.0.0.1:0").unwrap();
        let source_addr = source.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = source.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            write!(socket, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{target_addr}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let client = AnthropicClient::new(format!("http://{source_addr}"), "test-key".into()).unwrap();
        let result = client.run_completion(CompletionRequest { model:"fixture".into(), messages:vec![message("user","Hello")], temperature:None, max_tokens:Some(1) }).await;
        assert!(result.is_err());
        assert!(target.accept().is_err());
        server.join().unwrap();
    }

    async fn stream_fixture(body: &str) -> (Result<CompletionResponse, ClientError>, Vec<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let response = body.as_bytes().to_vec();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new(); let mut buf = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buf[..n]);
                if let Some(i) = data.windows(4).position(|part| part == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
            assert!(headers.contains("x-api-key: test-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < end + length { let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buf[..n]); }
            let request: Value = serde_json::from_slice(&data[end..end + length]).unwrap();
            assert_eq!(request["stream"], true);
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
            // Split UTF-8 and SSE records at arbitrary byte boundaries.
            for part in response.chunks(3) { if socket.write_all(part).is_err() { break; } }
        });
        let client: Box<dyn ModelGateway> = Box::new(AnthropicClient::new(format!("http://{addr}"), "test-key".into()).unwrap());
        assert!(client.transport_capabilities().text_streaming);
        assert!(!client.transport_capabilities().speech);
        let mut chunks = Vec::new();
        let result = client.stream_completion(CompletionRequest { model:"fixture".into(), messages:vec![message("user","Hello")], temperature:None, max_tokens:Some(20) }, &mut |delta| chunks.push(delta.to_owned())).await;
        server.join().unwrap();
        (result, chunks)
    }

    #[tokio::test]
    async fn unified_gateway_reports_unsupported_voice_without_network() {
        let client: Box<dyn ModelGateway> = Box::new(AnthropicClient::new("http://127.0.0.1:1".into(), "test-key".into()).unwrap());
        let speech = crate::SpeechRequest { model: "voice".into(), input: "hi".into(), voice: None, speed: None, response_format: "wav".into() };
        assert!(matches!(client.generate_speech(speech).await, Err(ClientError::UnsupportedOperation("speech generation"))));
        let request = CompletionRequest { model: "fixture".into(), messages: vec![message("user", "Return JSON")], temperature: None, max_tokens: Some(10) };
        assert!(!client.supports_json_object_completion());
        assert!(matches!(client.run_json_completion(request).await, Err(ClientError::UnsupportedOperation("JSON-object completion"))));
        let audio = crate::TranscriptionRequest { model: "voice".into(), audio: vec![], mime_type: "audio/wav".into(), filename: "a.wav".into(), language: None };
        assert!(matches!(client.transcribe(audio).await, Err(ClientError::UnsupportedOperation("transcription"))));
    }

    #[tokio::test]
    async fn streams_fragmented_utf8_and_preserves_usage() {
        let body = concat!(
            "event: message_start\r\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-stream\",\"model\":\"fixture-v1\",\"usage\":{\"input_tokens\":7,\"cache_read_input_tokens\":2}}}\r\n\r\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hé\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"llo\"}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":3}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n");
        let (result, chunks) = stream_fixture(body).await;
        let response = result.unwrap();
        assert_eq!(chunks, ["Hé", "llo"]);
        assert_eq!(response.content, "Héllo");
        assert_eq!((response.token_count_input, response.token_count_output), (Some(7), Some(3)));
        let meta = response.metadata.unwrap();
        assert_eq!((meta.response_id.as_deref(), meta.model.as_deref(), meta.finish_reason.as_deref()), (Some("msg-stream"), Some("fixture-v1"), Some("end_turn")));
        assert_eq!(meta.usage.unwrap(), json!({"input_tokens":7,"output_tokens":3,"cache_read_input_tokens":2}));
    }

    #[tokio::test]
    async fn stop_reasons_do_not_turn_partial_responses_into_success() {
        for reason in ["max_tokens", "model_context_window_exceeded", "pause_turn", "refusal", "tool_use", "future_unknown_reason"] {
            let body = format!("data: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":\"partial\"}}}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"{reason}\"}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n");
            let (result, chunks) = stream_fixture(&body).await;
            assert_eq!(chunks, ["partial"]);
            assert!(result.is_err(), "stream accepted {reason}");

            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let body = json!({"content":[{"type":"text","text":"partial"}],"stop_reason":reason}).to_string();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096]; let _ = socket.read(&mut request);
                let _ = write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            });
            let client = AnthropicClient::new(format!("http://{addr}"), "test-key".into()).unwrap();
            let request = CompletionRequest { model:"fixture".into(), messages:vec![message("user","Hello")], temperature:None, max_tokens:Some(20) };
            assert!(client.run_completion(request).await.is_err(), "completion accepted {reason}");
            server.join().unwrap();
        }
        assert!(AnthropicClient::validate_stop_reason(Some(&json!("end_turn"))).is_ok());
        assert!(AnthropicClient::validate_stop_reason(Some(&json!("stop_sequence"))).is_ok());
        assert!(AnthropicClient::validate_stop_reason(Some(&json!(5))).is_err());
    }

    #[tokio::test]
    async fn rejects_server_tools_in_successful_responses_and_streams() {
        for kind in ["tool_use", "server_tool_use", "web_search_tool_result", "code_execution_tool_result"] {
            let body = format!("data: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":\"partial\"}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"{kind}\"}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n");
            let (result, chunks) = stream_fixture(&body).await;
            assert_eq!(chunks, ["partial"]);
            assert!(result.is_err(), "stream accepted {kind}");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let body = json!({"content":[{"type":"text","text":"partial"},{"type":kind}],"stop_reason":"end_turn"}).to_string();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096]; let _ = socket.read(&mut request);
                let _ = write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            });
            let client = AnthropicClient::new(format!("http://{address}"), "test-key".into()).unwrap();
            let request = CompletionRequest { model:"fixture".into(), messages:vec![message("user","Hello")], temperature:None, max_tokens:Some(20) };
            assert!(client.run_completion(request).await.is_err(), "completion accepted {kind}");
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn stream_rejects_errors_truncation_and_tool_use() {
        for body in [
            "data: {\"type\":\"error\",\"error\":{\"message\":\"overloaded\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n",
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"tool_use\"}}\n\n",
            "data: malformed\n\n",
        ] { assert!(stream_fixture(body).await.0.is_err()); }
    }

}

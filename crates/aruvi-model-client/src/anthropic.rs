//! Anthropic Messages API client for text conversations.
use crate::{ClientError, CompletionRequest, CompletionResponse, ModelGateway};
use async_trait::async_trait;
use reqwest::{Client, redirect::Policy};
use serde_json::{json, Value};
use std::time::Duration;

const API_VERSION: &str = "2023-06-01";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self, ClientError> {
        if api_key.trim().is_empty() { return Err(ClientError::InvalidInput("Anthropic API key is required".into())); }
        let url = reqwest::Url::parse(&base_url).map_err(|_| ClientError::InvalidInput("Invalid Anthropic base URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err(ClientError::InvalidInput("Invalid Anthropic base URL".into()));
        }
        let client = Client::builder().connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(300)).redirect(Policy::none()).build()
            .map_err(|error| ClientError::Provider(format!("Failed to initialize HTTP client: {error}")))?;
        Ok(Self { base_url, api_key, client })
    }

    fn endpoint(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") { format!("{base}{path}") } else { format!("{base}/v1{path}") }
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

#[async_trait]
impl ModelGateway for AnthropicClient {
    async fn health_check(&self) -> Result<bool, ClientError> {
        let response = self.client.get(self.endpoint("/models")).header("x-api-key", &self.api_key).header("anthropic-version", API_VERSION).send().await
            .map_err(|error| ClientError::Provider(format!("Anthropic health check failed: {error}")))?;
        Ok(response.status().is_success())
    }

    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
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
        let response = self.client.post(self.endpoint("/messages")).header("x-api-key", &self.api_key).header("anthropic-version", API_VERSION).json(&body).send().await
            .map_err(|error| ClientError::Provider(format!("Anthropic request failed: {error}")))?;
        let json = Self::read_json(response).await?;
        let blocks = json.get("content").and_then(Value::as_array).ok_or_else(|| ClientError::Provider("Anthropic response is missing content".into()))?;
        if blocks.iter().any(|block| block.get("type").and_then(Value::as_str) == Some("tool_use")) {
            return Err(ClientError::Provider("Anthropic tool-use responses are not supported by the text client".into()));
        }
        let content = blocks.iter().filter(|block| block.get("type").and_then(Value::as_str) == Some("text")).filter_map(|block| block.get("text").and_then(Value::as_str)).collect::<String>();
        if content.is_empty() { return Err(ClientError::Provider("Anthropic returned no text content".into())); }
        Ok(CompletionResponse { content, token_count_input: json.pointer("/usage/input_tokens").and_then(Value::as_i64), token_count_output: json.pointer("/usage/output_tokens").and_then(Value::as_i64) })
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
            let reply = r#"{"content":[{"type":"text","text":"Hello"},{"type":"text","text":" there"}],"usage":{"input_tokens":5,"output_tokens":2}}"#;
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",reply.len(),reply).unwrap();
        });
        let client = AnthropicClient::new(format!("http://{addr}/v1"), "test-key".into()).unwrap();
        let response = client.run_completion(CompletionRequest { model:"fixture".into(), messages:vec![message("system","First"),message("system","Second"),message("user","Hello")], temperature:Some(0.2), max_tokens:Some(20) }).await.unwrap();
        assert_eq!(response.content, "Hello there");
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
}

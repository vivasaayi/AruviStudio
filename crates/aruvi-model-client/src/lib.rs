//! Provider-independent model request types and reusable HTTP clients.
//! Applications own credentials, persistence and telemetry.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod openai_compatible;
pub mod anthropic;
mod sse;
mod discovery;
mod http;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    InvalidInput(String),
    /// A non-success HTTP response. The provider response body is never retained.
    #[error("{message}")]
    Http { status: u16, message: String },
    #[error("Provider does not support multipart transcription uploads")]
    UnsupportedTranscription,
    #[error("Client transport does not implement {0}")]
    UnsupportedOperation(&'static str),
}

impl ClientError {
    /// Return a remote HTTP status when the operation reached an HTTP response.
    /// Transport failures and provider errors sent inside a successful response
    /// have no HTTP status here. A status does not imply an automatic retry.
    pub fn http_status(&self) -> Option<u16> {
        match self { Self::Http { status, .. } => Some(*status), _ => None }
    }

    pub(crate) fn http(label: &str, status: reqwest::StatusCode) -> Self {
        Self::Http { status: status.as_u16(), message: format!("{label} {status}") }
    }
}

/// HTTP timeouts applied to all operations, including streamed responses.
/// Dropping a request future also cancels it without spawning a background task.
#[derive(Debug, Clone, Copy)]
pub struct ClientOptions {
    pub connect_timeout: std::time::Duration,
    pub request_timeout: std::time::Duration,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self { connect_timeout: std::time::Duration::from_secs(15), request_timeout: std::time::Duration::from_secs(300) }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    pub model: String,
    pub audio: Vec<u8>,
    pub mime_type: String,
    pub filename: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechRequest {
    pub model: String,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CompletionMetadata>,
}

/// Selected provider metadata, using the provider’s native finish reason and usage fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompletionMetadata {
    pub response_id: Option<String>,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
    pub usage: Option<serde_json::Value>,
}

impl CompletionMetadata {
    pub(crate) fn merge(target: &mut Option<Self>, source: &serde_json::Value, finish_reason: Option<&serde_json::Value>) {
        let id = source.get("id").and_then(serde_json::Value::as_str);
        let model = source.get("model").and_then(serde_json::Value::as_str);
        let reason = finish_reason.and_then(serde_json::Value::as_str);
        let usage = source.get("usage").and_then(serde_json::Value::as_object);
        if id.is_none() && model.is_none() && reason.is_none() && usage.is_none() { return; }
        let meta = target.get_or_insert_with(Self::default);
        if let Some(value) = id { meta.response_id = Some(value.to_owned()); }
        if let Some(value) = model { meta.model = Some(value.to_owned()); }
        if let Some(value) = reason { meta.finish_reason = Some(value.to_owned()); }
        if let Some(value) = usage {
            let accumulated = meta.usage.get_or_insert_with(|| serde_json::json!({}));
            if let Some(fields) = accumulated.as_object_mut() { fields.extend(value.clone()); }
        }
    }
}

/// A model as advertised by the provider. Metadata is provider-specific and
/// does not imply that Studio or this client implements every capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModel {
    pub id: String,
    pub display_name: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPage {
    pub models: Vec<DiscoveredModel>,
    pub next_cursor: Option<String>,
}

/// Operations implemented by this transport, not a guarantee that a remote
/// provider or particular model offers them. Discover and configure models
/// separately, and handle runtime provider errors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransportCapabilities {
    pub text_completion: bool,
    pub text_streaming: bool,
    pub model_discovery: bool,
    pub transcription: bool,
    pub speech: bool,
}

#[async_trait]
pub trait ModelGateway: Send + Sync {
    fn transport_capabilities(&self) -> TransportCapabilities { TransportCapabilities::default() }
    /// Whether this transport can request JSON-object output. A particular
    /// endpoint or model may still reject the option at runtime.
    fn supports_json_object_completion(&self) -> bool { false }
    async fn health_check(&self) -> Result<bool, ClientError>;
    /// Fetch one page; pass the returned cursor to retrieve the next page.
    async fn list_models_page(&self, _cursor: Option<&str>) -> Result<ModelPage, ClientError> {
        Err(ClientError::Provider("This provider does not support model discovery".into()))
    }
    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError>;
    /// Request a JSON object without assuming every remote model supports it.
    /// The response is still validated by the adapter; no schema is implied.
    async fn run_json_completion(&self, _request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
        Err(ClientError::UnsupportedOperation("JSON-object completion"))
    }
    /// The final result must be checked even if text has already been emitted.
    /// Dropping the returned future cancels the request.
    async fn stream_completion(&self, _request: CompletionRequest, _on_delta: &mut (dyn for<'a> FnMut(&'a str) + Send)) -> Result<CompletionResponse, ClientError> {
        Err(ClientError::UnsupportedOperation("text streaming"))
    }
    async fn transcribe(&self, _request: TranscriptionRequest) -> Result<String, ClientError> {
        Err(ClientError::UnsupportedOperation("transcription"))
    }
    async fn generate_speech(&self, _request: SpeechRequest) -> Result<Vec<u8>, ClientError> {
        Err(ClientError::UnsupportedOperation("speech generation"))
    }
}

#[cfg(test)]
mod response_compatibility_tests {
    use super::*;

    struct LegacyGateway;

    #[async_trait]
    impl ModelGateway for LegacyGateway {
        async fn health_check(&self) -> Result<bool, ClientError> { Ok(true) }
        async fn run_completion(&self, _request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
            Ok(CompletionResponse::default())
        }
    }

    #[test]
    fn existing_gateway_implementations_default_json_support_to_false() {
        let gateway: Box<dyn ModelGateway> = Box::new(LegacyGateway);
        assert!(!gateway.supports_json_object_completion());
    }

    #[test]
    fn old_json_without_metadata_round_trips_without_a_new_null_field() {
        let old = serde_json::json!({"content":"hello","token_count_input":null,"token_count_output":2});
        let response: CompletionResponse = serde_json::from_value(old.clone()).unwrap();
        assert!(response.metadata.is_none());
        assert_eq!(serde_json::to_value(response).unwrap(), old);
    }
}

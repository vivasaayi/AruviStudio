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
    #[error("Provider does not support multipart transcription uploads")]
    UnsupportedTranscription,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
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

#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn health_check(&self) -> Result<bool, ClientError>;
    /// Fetch one page; pass the returned cursor to retrieve the next page.
    async fn list_models_page(&self, _cursor: Option<&str>) -> Result<ModelPage, ClientError> {
        Err(ClientError::Provider("This provider does not support model discovery".into()))
    }
    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError>;
}

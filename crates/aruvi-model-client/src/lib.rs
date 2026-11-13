//! Provider-independent model request types and reusable HTTP clients.
//! Applications own credentials, persistence and telemetry.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod openai_compatible;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("{0}")]
    Provider(String),
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

#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn health_check(&self) -> Result<bool, ClientError>;
    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError>;
}

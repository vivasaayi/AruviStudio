use crate::error::AppError;
use crate::providers::gateway::ModelGateway;
use crate::providers::types::{CompletionRequest, CompletionResponse};
use aruvi_model_client::openai_compatible::OpenAiCompatibleClient;
use aruvi_model_client::ModelGateway as SharedModelGateway;
use async_trait::async_trait;

pub struct OpenAiCompatibleProvider(OpenAiCompatibleClient);

impl OpenAiCompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self(OpenAiCompatibleClient::new(base_url, api_key))
    }
}

#[async_trait]
impl ModelGateway for OpenAiCompatibleProvider {
    async fn health_check(&self) -> Result<bool, AppError> {
        self.0.health_check().await.map_err(|error| AppError::Provider(error.to_string()))
    }

    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, AppError> {
        self.0.run_completion(request).await.map_err(|error| AppError::Provider(error.to_string()))
    }
}

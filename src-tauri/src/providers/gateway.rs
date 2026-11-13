use crate::error::AppError;
use crate::providers::types::{CompletionRequest, CompletionResponse};
use async_trait::async_trait;

#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn health_check(&self) -> Result<bool, AppError>;
    async fn list_models_page(&self, cursor: Option<&str>) -> Result<aruvi_model_client::ModelPage, AppError>;
    async fn run_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, AppError>;
}

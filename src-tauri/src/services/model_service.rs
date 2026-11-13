use crate::domain::model::ModelProvider;
use crate::error::AppError;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{CompletionRequest, CompletionResponse};
use crate::secrets;
use aruvi_model_client::anthropic::AnthropicClient;
use aruvi_model_client::ModelGateway as SharedModelGateway;
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct ModelService;

impl ModelService {
    pub fn new(_db: Arc<SqlitePool>) -> Self {
        Self
    }

    /// Create a model gateway for a provider
    pub fn create_gateway(
        &self,
        provider: &ModelProvider,
    ) -> Result<Box<dyn ModelGateway>, AppError> {
        create_gateway(provider)
    }
}

struct SharedGatewayAdapter(Box<dyn SharedModelGateway>);

#[async_trait]
impl ModelGateway for SharedGatewayAdapter {
    async fn health_check(&self) -> Result<bool, AppError> {
        self.0
            .health_check()
            .await
            .map_err(|error| AppError::Provider(error.to_string()))
    }

    async fn run_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, AppError> {
        self.0
            .run_completion(request)
            .await
            .map_err(|error| AppError::Provider(error.to_string()))
    }
}

/// Select the transport from the saved provider protocol, resolving credentials
/// through Studio's existing secret store.
pub fn create_gateway(provider: &ModelProvider) -> Result<Box<dyn ModelGateway>, AppError> {
    if provider.provider_type.as_str() != "openai_compatible" {
        return Err(AppError::Validation(format!(
            "Unsupported provider type: {}",
            provider.provider_type
        )));
    }
    if provider.protocol == "anthropic"
        && provider
            .auth_secret_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::Validation(
            "An explicit Anthropic API key is required".into(),
        ));
    }
    let api_key = secrets::resolve_provider_secret(provider)?;
    match provider.protocol.as_str() {
        "openai_compatible" => Ok(Box::new(OpenAiCompatibleProvider::new(
            provider.base_url.clone(),
            api_key,
        ))),
        "anthropic" => {
            let key = api_key
                .ok_or_else(|| AppError::Validation("Anthropic API key is required".into()))?;
            let client = AnthropicClient::new(provider.base_url.clone(), key)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            Ok(Box::new(SharedGatewayAdapter(Box::new(client))))
        }
        protocol => Err(AppError::Validation(format!(
            "Unsupported provider protocol: {protocol}"
        ))),
    }
}

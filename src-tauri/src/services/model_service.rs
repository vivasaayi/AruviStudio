use crate::domain::model::ModelProvider;
use crate::error::AppError;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::secrets;
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
        match provider.provider_type.as_str() {
            "openai_compatible" => {
                let api_key = secrets::resolve_provider_secret(provider)?;
                let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
                Ok(Box::new(gateway))
            }
            _ => Err(AppError::Validation(format!(
                "Unsupported provider type: {}",
                provider.provider_type
            ))),
        }
    }
}

use crate::domain::model::{ModelDefinition, ModelProvider};
use crate::error::AppError;
use crate::persistence::model_repo;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::secrets;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct ModelService {
    db: Arc<SqlitePool>,
}

impl ModelService {
    pub fn new(db: Arc<SqlitePool>) -> Self {
        Self { db }
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

    /// Test provider connectivity
    pub async fn test_provider_connectivity(&self, provider_id: &str) -> Result<String, AppError> {
        let provider = model_repo::get_provider(&self.db, provider_id).await?;
        let gateway = self.create_gateway(&provider)?;

        match gateway.health_check().await {
            Ok(true) => Ok("Connection successful".to_string()),
            Ok(false) => Ok("Connection failed - server responded but not healthy".to_string()),
            Err(e) => Err(e),
        }
    }

    /// Get all providers
    pub async fn list_providers(&self) -> Result<Vec<ModelProvider>, AppError> {
        model_repo::list_providers(&self.db).await
    }

    /// Get all model definitions
    pub async fn list_model_definitions(&self) -> Result<Vec<ModelDefinition>, AppError> {
        model_repo::list_model_definitions(&self.db).await
    }

    /// Create a new provider
    pub async fn create_provider(
        &self,
        name: &str,
        provider_type: &str,
        base_url: &str,
        auth_secret_ref: Option<&str>,
    ) -> Result<ModelProvider, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let stored_secret_ref = secrets::store_provider_secret(&id, auth_secret_ref)?;
        model_repo::create_provider(
            &self.db,
            &id,
            name,
            provider_type,
            base_url,
            stored_secret_ref.as_deref(),
        )
        .await
    }

    /// Create a new model definition
    pub async fn create_model_definition(
        &self,
        provider_id: &str,
        name: &str,
        context_window: Option<i64>,
    ) -> Result<ModelDefinition, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        model_repo::create_model_definition(&self.db, &id, provider_id, name, context_window).await
    }
}

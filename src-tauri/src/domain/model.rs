use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelProvider {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub auth_secret_ref: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProviderType {
    OpenaiCompatible,
    LocalRuntime,
}

impl ProviderType {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::OpenaiCompatible => "openai_compatible",
            ProviderType::LocalRuntime => "local_runtime",
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelDefinition {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub context_window: Option<i64>,
    pub capability_tags: Vec<String>,
    pub notes: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

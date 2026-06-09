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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelCall {
    pub id: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub source_label: String,
    pub workflow_run_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub work_item_id: Option<String>,
    pub product_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub stage: Option<String>,
    pub provider_id: String,
    pub provider_name: String,
    pub provider_type: String,
    pub provider_base_url: String,
    pub model_id: Option<String>,
    pub model_name: String,
    pub call_index: i64,
    pub request_message_count: i64,
    pub prompt_chars: i64,
    pub response_chars: i64,
    pub request_snapshot_path: Option<String>,
    pub response_snapshot_path: Option<String>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Artifact {
    pub id: String,
    pub work_item_id: String,
    pub workflow_run_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub artifact_type: String,
    pub storage_path: String,
    pub summary: String,
    pub content_type: String,
    pub size_bytes: Option<i64>,
    pub created_at: String,
}

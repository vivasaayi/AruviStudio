use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Finding {
    pub id: String,
    pub work_item_id: String,
    pub source_agent_run_id: Option<String>,
    pub category: FindingCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub status: FindingStatus,
    pub is_blocking: bool,
    pub linked_followup_work_item_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum FindingCategory {
    Security,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    Resolved,
    WontFix,
    Deferred,
}

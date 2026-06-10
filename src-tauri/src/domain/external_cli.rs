use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCliRun {
    pub id: String,
    pub work_item_id: String,
    pub provider: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub prompt: String,
    pub cwd: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub stdout_chars: i64,
    pub stderr_chars: i64,
    pub output_artifact_id: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ExternalCliInvocation {
    pub work_item_id: String,
    pub provider: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub prompt: String,
    pub cwd: String,
}

use crate::error::AppError;
use crate::persistence::observability_repo::{self, LogEntry};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_logs(
    state: State<'_, AppState>,
    level: Option<String>,
    target: Option<String>,
    workflow_run_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<LogEntry>, AppError> {
    observability_repo::get_logs(
        &state.db,
        level.as_deref(),
        target.as_deref(),
        workflow_run_id.as_deref(),
        limit.unwrap_or(100),
    )
    .await
}

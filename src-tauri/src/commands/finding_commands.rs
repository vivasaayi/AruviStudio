use crate::domain::finding::Finding;
use crate::error::AppError;
use crate::persistence::finding_repo;
use crate::state::AppState;
use tauri::State;
use tracing::debug;

#[tauri::command]
pub async fn list_work_item_findings(
    state: State<'_, AppState>,
    work_item_id: String,
) -> Result<Vec<Finding>, AppError> {
    debug!(work_item_id = %work_item_id, "list_work_item_findings requested");
    finding_repo::list_work_item_findings(&state.db, &work_item_id).await
}

use crate::domain::artifact::Artifact;
use crate::error::AppError;
use crate::persistence::artifact_repo;
use crate::state::AppState;
use tauri::State;
use tracing::debug;

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_work_item_artifacts(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
) -> Result<Vec<Artifact>, AppError> {
    let work_item_id = work_item_id
        .or(workItemId)
        .ok_or_else(|| AppError::Validation("missing work item id".to_string()))?;
    debug!(work_item_id = %work_item_id, "list_work_item_artifacts requested");
    artifact_repo::list_work_item_artifacts(&state.db, &work_item_id).await
}

#[tauri::command]
pub async fn read_artifact_content(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<String, AppError> {
    let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
    tokio::fs::read_to_string(&artifact.storage_path)
        .await
        .map_err(AppError::Io)
}

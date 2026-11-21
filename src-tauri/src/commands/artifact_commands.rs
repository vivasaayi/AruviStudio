use crate::domain::artifact::Artifact;
use crate::error::AppError;
use crate::persistence::artifact_repo;
use crate::state::AppState;
use tauri::State;
use tracing::debug;

#[tauri::command]
pub async fn list_work_item_artifacts(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
) -> Result<Vec<Artifact>, AppError> {
    let work_item_id =
        work_item_id.ok_or_else(|| AppError::Validation("missing work item id".to_string()))?;
    debug!(work_item_id = %work_item_id, "list_work_item_artifacts requested");
    artifact_repo::list_work_item_artifacts(&state.db, &work_item_id).await
}

#[tauri::command]
pub async fn read_artifact_content(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<String, AppError> {
    let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
    crate::services::artifact_service::read_text(
        &state.artifact_base_path,
        std::path::Path::new(&artifact.storage_path),
        crate::services::artifact_service::TEXT_LIMIT,
    ).await
}

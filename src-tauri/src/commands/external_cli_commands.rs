use crate::domain::external_cli::ExternalCliRun;
use crate::error::AppError;
use crate::persistence::external_cli_repo;
use crate::services::external_cli_service;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
#[allow(non_snake_case)]
pub async fn invoke_external_cli_for_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    provider: String,
) -> Result<ExternalCliRun, AppError> {
    let work_item_id = work_item_id
        .or(workItemId)
        .ok_or_else(|| AppError::Validation("missing work item id".to_string()))?;
    external_cli_service::run_external_cli_for_work_item(
        &state.db,
        &state.artifact_base_path,
        &work_item_id,
        &provider,
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_external_cli_runs_for_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
) -> Result<Vec<ExternalCliRun>, AppError> {
    let work_item_id = work_item_id
        .or(workItemId)
        .ok_or_else(|| AppError::Validation("missing work item id".to_string()))?;
    external_cli_repo::list_external_cli_runs_for_work_item(&state.db, &work_item_id).await
}

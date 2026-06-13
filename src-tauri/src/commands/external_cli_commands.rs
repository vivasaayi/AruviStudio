use crate::domain::external_cli::{ExternalCliRun, ExternalCliRunEvent};
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

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_external_cli_run_events(
    state: State<'_, AppState>,
    run_id: Option<String>,
    runId: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<ExternalCliRunEvent>, AppError> {
    let run_id = run_id
        .or(runId)
        .ok_or_else(|| AppError::Validation("missing external CLI run id".to_string()))?;
    external_cli_repo::list_external_cli_run_events(&state.db, &run_id, limit.unwrap_or(500)).await
}

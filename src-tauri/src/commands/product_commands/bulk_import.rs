use crate::domain::bulk_import::{BulkImportJob, BulkImportJobStatus};
use crate::error::AppError;
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_bulk_import_schema() -> Result<Value, AppError> {
    Ok(bulk_import_service::bulk_import_schema())
}

#[tauri::command]
pub async fn submit_bulk_import(
    state: State<'_, AppState>,
    file_path: Option<String>,
    format: Option<String>,
    product_id: Option<String>,
) -> Result<BulkImportJob, AppError> {
    let file_path =
        file_path.ok_or_else(|| AppError::Validation("missing file path".to_string()))?;
    bulk_import_service::submit_bulk_import(
        state.inner().clone(),
        BulkImportRequest {
            file_path,
            format,
            product_id,
        },
    )
    .await
}

#[tauri::command]
pub async fn get_bulk_import_status(
    state: State<'_, AppState>,
    job_id: Option<String>,
) -> Result<BulkImportJobStatus, AppError> {
    let job_id = job_id.ok_or_else(|| AppError::Validation("missing job id".to_string()))?;
    bulk_import_service::get_bulk_import_status(&state.db, &job_id).await
}

#[tauri::command]
pub async fn list_bulk_import_jobs(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<BulkImportJob>, AppError> {
    bulk_import_service::list_bulk_import_jobs(&state.db, limit).await
}

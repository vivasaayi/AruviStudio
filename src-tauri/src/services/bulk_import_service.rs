use crate::domain::bulk_import::{BulkImportJob, BulkImportJobStatus};
use crate::error::AppError;
use crate::persistence::bulk_import_repo;
use crate::services::bulk_import_builder::{prepare_csv_import, prepare_json_import};
use crate::state::AppState;
use sqlx::SqlitePool;
use std::path::Path;
use tracing::{error, info};

const DEFAULT_BATCH_SIZE: usize = 1_000;

pub use crate::services::bulk_import_schema::bulk_import_schema;

#[derive(Debug, Clone)]
pub struct BulkImportRequest {
    pub file_path: String,
    pub format: Option<String>,
    pub product_id: Option<String>,
}

pub async fn submit_bulk_import(
    state: AppState,
    request: BulkImportRequest,
) -> Result<BulkImportJob, AppError> {
    let import_format = resolve_format(&request.file_path, request.format.as_deref())?;
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = bulk_import_repo::create_job(
        &state.db,
        &job_id,
        request.file_path.as_str(),
        import_format.as_str(),
    )
    .await?;
    let worker_state = state.clone();
    let worker_request = BulkImportRequest {
        file_path: request.file_path,
        format: Some(import_format),
        product_id: request.product_id,
    };
    let worker_job_id = job.id.clone();

    tokio::spawn(async move {
        if let Err(error) =
            run_bulk_import_job(worker_state.clone(), worker_job_id.clone(), worker_request).await
        {
            let message = error.to_string();
            error!(job_id = %worker_job_id, error = %message, "bulk import job failed");
            let _ = bulk_import_repo::add_job_error(
                &worker_state.db,
                &worker_job_id,
                None,
                "job",
                "",
                &message,
            )
            .await;
            let _ =
                bulk_import_repo::mark_job_failed(&worker_state.db, &worker_job_id, &message).await;
        }
    });

    Ok(job)
}

pub async fn get_bulk_import_status(
    pool: &SqlitePool,
    job_id: &str,
) -> Result<BulkImportJobStatus, AppError> {
    Ok(BulkImportJobStatus {
        job: bulk_import_repo::get_job(pool, job_id).await?,
        errors: bulk_import_repo::list_job_errors(pool, job_id, 25).await?,
    })
}

pub async fn list_bulk_import_jobs(
    pool: &SqlitePool,
    limit: Option<i64>,
) -> Result<Vec<BulkImportJob>, AppError> {
    bulk_import_repo::list_jobs(pool, limit.unwrap_or(20)).await
}

async fn run_bulk_import_job(
    state: AppState,
    job_id: String,
    request: BulkImportRequest,
) -> Result<(), AppError> {
    let import_format = request
        .format
        .as_deref()
        .ok_or_else(|| AppError::Validation("missing import format".to_string()))?;
    info!(job_id = %job_id, file_path = %request.file_path, import_format = %import_format, "bulk import job started");
    let content = tokio::fs::read_to_string(&request.file_path).await?;
    let prepared = match import_format {
        "json" => prepare_json_import(&content, request.product_id.as_deref())?,
        "csv" => prepare_csv_import(&content, request.product_id.as_deref())?,
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported bulk import format '{other}'. Use json or csv."
            )))
        }
    };
    if prepared.rows.total_records() == 0 {
        return Err(AppError::Validation(
            "Bulk import file did not contain any records.".to_string(),
        ));
    }

    bulk_import_repo::mark_job_running(&state.db, &job_id, prepared.rows.total_records()).await?;
    for product_id in &prepared.required_existing_product_ids {
        bulk_import_repo::ensure_product_exists(&state.db, product_id).await?;
    }
    bulk_import_repo::upsert_all(&state.db, &job_id, &prepared.rows, DEFAULT_BATCH_SIZE).await?;
    let completed = bulk_import_repo::mark_job_completed(&state.db, &job_id).await?;
    info!(
        job_id = %job_id,
        total_records = completed.total_records,
        processed_records = completed.processed_records,
        "bulk import job completed"
    );
    Ok(())
}

fn resolve_format(file_path: &str, requested: Option<&str>) -> Result<String, AppError> {
    let value = requested
        .map(ToString::to_string)
        .or_else(|| {
            Path::new(file_path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(ToString::to_string)
        })
        .ok_or_else(|| {
            AppError::Validation(
                "Bulk import format is required when file extension is absent.".to_string(),
            )
        })?;
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => Ok("json".to_string()),
        "csv" => Ok("csv".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported bulk import format '{other}'. Use json or csv."
        ))),
    }
}

#[cfg(test)]
mod tests;

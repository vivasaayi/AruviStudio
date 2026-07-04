use crate::domain::bulk_import::{BulkImportJob, BulkImportJobError};
use crate::error::AppError;
use sqlx::SqlitePool;

pub(crate) const JOB_SELECT_COLUMNS: &str = "id, source_path, import_format, status, total_records, processed_records, product_count, product_area_count, capability_count, feature_count, work_item_count, failed_records, error_message, created_at, started_at, completed_at, updated_at";
const ERROR_SELECT_COLUMNS: &str =
    "id, job_id, row_index, record_type, record_id, message, created_at";

pub async fn create_job(
    pool: &SqlitePool,
    id: &str,
    source_path: &str,
    import_format: &str,
) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "INSERT INTO bulk_import_jobs (id, source_path, import_format)
         VALUES (?, ?, ?)
         RETURNING {JOB_SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(source_path)
    .bind(import_format)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_job_running(
    pool: &SqlitePool,
    job_id: &str,
    total_records: i64,
) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "UPDATE bulk_import_jobs
         SET status='running', total_records=?, started_at=COALESCE(started_at, datetime('now')), updated_at=datetime('now')
         WHERE id=?
         RETURNING {JOB_SELECT_COLUMNS}"
    ))
    .bind(total_records)
    .bind(job_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub struct UpdateBulkImportJobProgressInput<'a> {
    pub job_id: &'a str,
    pub processed_delta: i64,
    pub product_delta: i64,
    pub product_area_delta: i64,
    pub capability_delta: i64,
    pub feature_delta: i64,
    pub work_item_delta: i64,
}

pub async fn update_job_progress(
    pool: &SqlitePool,
    input: UpdateBulkImportJobProgressInput<'_>,
) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "UPDATE bulk_import_jobs
         SET processed_records=processed_records + ?,
             product_count=product_count + ?,
             product_area_count=product_area_count + ?,
             capability_count=capability_count + ?,
             feature_count=feature_count + ?,
             work_item_count=work_item_count + ?,
             updated_at=datetime('now')
         WHERE id=?
         RETURNING {JOB_SELECT_COLUMNS}"
    ))
    .bind(input.processed_delta)
    .bind(input.product_delta)
    .bind(input.product_area_delta)
    .bind(input.capability_delta)
    .bind(input.feature_delta)
    .bind(input.work_item_delta)
    .bind(input.job_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_job_completed(
    pool: &SqlitePool,
    job_id: &str,
) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "UPDATE bulk_import_jobs
         SET status='completed', completed_at=datetime('now'), updated_at=datetime('now')
         WHERE id=?
         RETURNING {JOB_SELECT_COLUMNS}"
    ))
    .bind(job_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_job_failed(
    pool: &SqlitePool,
    job_id: &str,
    message: &str,
) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "UPDATE bulk_import_jobs
         SET status='failed',
             failed_records=CASE WHEN failed_records = 0 THEN 1 ELSE failed_records END,
             error_message=?,
             completed_at=datetime('now'),
             updated_at=datetime('now')
         WHERE id=?
         RETURNING {JOB_SELECT_COLUMNS}"
    ))
    .bind(message)
    .bind(job_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_interrupted_jobs(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE bulk_import_jobs
         SET status='failed',
             failed_records=CASE WHEN failed_records = 0 THEN 1 ELSE failed_records END,
             error_message='Bulk import was interrupted before completion.',
             completed_at=datetime('now'),
             updated_at=datetime('now')
         WHERE status IN ('pending', 'running')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_job_error(
    pool: &SqlitePool,
    job_id: &str,
    row_index: Option<i64>,
    record_type: &str,
    record_id: &str,
    message: &str,
) -> Result<BulkImportJobError, AppError> {
    sqlx::query_as::<_, BulkImportJobError>(&format!(
        "INSERT INTO bulk_import_job_errors (id, job_id, row_index, record_type, record_id, message)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING {ERROR_SELECT_COLUMNS}"
    ))
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(job_id)
    .bind(row_index)
    .bind(record_type)
    .bind(record_id)
    .bind(message)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_job(pool: &SqlitePool, job_id: &str) -> Result<BulkImportJob, AppError> {
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "SELECT {JOB_SELECT_COLUMNS} FROM bulk_import_jobs WHERE id=?"
    ))
    .bind(job_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Bulk import job {job_id} not found")))
}

pub async fn list_jobs(pool: &SqlitePool, limit: i64) -> Result<Vec<BulkImportJob>, AppError> {
    let limit = limit.clamp(1, 100);
    sqlx::query_as::<_, BulkImportJob>(&format!(
        "SELECT {JOB_SELECT_COLUMNS}
         FROM bulk_import_jobs
         ORDER BY created_at DESC
         LIMIT ?"
    ))
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_job_errors(
    pool: &SqlitePool,
    job_id: &str,
    limit: i64,
) -> Result<Vec<BulkImportJobError>, AppError> {
    let limit = limit.clamp(1, 100);
    sqlx::query_as::<_, BulkImportJobError>(&format!(
        "SELECT {ERROR_SELECT_COLUMNS}
         FROM bulk_import_job_errors
         WHERE job_id=?
         ORDER BY created_at ASC
         LIMIT ?"
    ))
    .bind(job_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

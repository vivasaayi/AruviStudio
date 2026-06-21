use crate::domain::bulk_import::{
    BulkImportCapabilityRow, BulkImportJob, BulkImportJobError, BulkImportProductAreaRow,
    BulkImportProductRow, BulkImportRows, BulkImportWorkItemRow,
};
use crate::error::AppError;
use sqlx::SqlitePool;

const JOB_SELECT_COLUMNS: &str = "id, source_path, import_format, status, total_records, processed_records, product_count, product_area_count, capability_count, feature_count, work_item_count, failed_records, error_message, created_at, started_at, completed_at, updated_at";
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

pub async fn update_job_progress(
    pool: &SqlitePool,
    job_id: &str,
    processed_delta: i64,
    product_delta: i64,
    product_area_delta: i64,
    capability_delta: i64,
    feature_delta: i64,
    work_item_delta: i64,
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
    .bind(processed_delta)
    .bind(product_delta)
    .bind(product_area_delta)
    .bind(capability_delta)
    .bind(feature_delta)
    .bind(work_item_delta)
    .bind(job_id)
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

pub async fn ensure_product_exists(pool: &SqlitePool, product_id: &str) -> Result<(), AppError> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM products WHERE id=?")
        .bind(product_id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::Validation(format!(
            "Product {product_id} does not exist and the import file did not define it."
        )));
    }
    Ok(())
}

pub async fn upsert_products(
    pool: &SqlitePool,
    job_id: &str,
    rows: &[BulkImportProductRow],
    batch_size: usize,
) -> Result<(), AppError> {
    for chunk in rows.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for row in chunk {
            sqlx::query(
                "INSERT INTO products (
                    id, name, description, vision, goals, tags, lifecycle, health,
                    owner_label, investment_status, roadmap, evidence
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    name=excluded.name,
                    description=excluded.description,
                    vision=excluded.vision,
                    goals=excluded.goals,
                    tags=excluded.tags,
                    lifecycle=excluded.lifecycle,
                    health=excluded.health,
                    owner_label=excluded.owner_label,
                    investment_status=excluded.investment_status,
                    roadmap=excluded.roadmap,
                    evidence=excluded.evidence,
                    updated_at=datetime('now')",
            )
            .bind(&row.id)
            .bind(&row.name)
            .bind(&row.description)
            .bind(&row.vision)
            .bind(&row.goals_json)
            .bind(&row.tags_json)
            .bind(&row.lifecycle)
            .bind(&row.health)
            .bind(&row.owner_label)
            .bind(&row.investment_status)
            .bind(&row.roadmap)
            .bind(&row.evidence)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        update_job_progress(
            pool,
            job_id,
            chunk.len().try_into().unwrap_or(i64::MAX),
            chunk.len().try_into().unwrap_or(i64::MAX),
            0,
            0,
            0,
            0,
        )
        .await?;
    }
    Ok(())
}

pub async fn upsert_product_areas(
    pool: &SqlitePool,
    job_id: &str,
    rows: &[BulkImportProductAreaRow],
    batch_size: usize,
) -> Result<(), AppError> {
    for chunk in rows.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for row in chunk {
            sqlx::query(
                "INSERT INTO modules (
                    id, product_id, node_kind, name, description, purpose, explanation,
                    examples, implementation_notes, test_guidance, sort_order
                 )
                 VALUES (?, ?, 'product_area', ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    product_id=excluded.product_id,
                    node_kind='product_area',
                    name=excluded.name,
                    description=excluded.description,
                    purpose=excluded.purpose,
                    explanation=excluded.explanation,
                    examples=excluded.examples,
                    implementation_notes=excluded.implementation_notes,
                    test_guidance=excluded.test_guidance,
                    sort_order=excluded.sort_order,
                    updated_at=datetime('now')",
            )
            .bind(&row.id)
            .bind(&row.product_id)
            .bind(&row.name)
            .bind(&row.description)
            .bind(&row.purpose)
            .bind(&row.explanation)
            .bind(&row.examples)
            .bind(&row.implementation_notes)
            .bind(&row.test_guidance)
            .bind(row.sort_order)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        update_job_progress(
            pool,
            job_id,
            chunk.len().try_into().unwrap_or(i64::MAX),
            0,
            chunk.len().try_into().unwrap_or(i64::MAX),
            0,
            0,
            0,
        )
        .await?;
    }
    Ok(())
}

pub async fn upsert_capabilities(
    pool: &SqlitePool,
    job_id: &str,
    rows: &[BulkImportCapabilityRow],
    batch_size: usize,
) -> Result<(), AppError> {
    for chunk in rows.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for row in chunk {
            sqlx::query(
                "INSERT INTO capabilities (
                    id, module_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    module_id=excluded.module_id,
                    parent_capability_id=excluded.parent_capability_id,
                    level=excluded.level,
                    node_kind=excluded.node_kind,
                    sort_order=excluded.sort_order,
                    name=excluded.name,
                    description=excluded.description,
                    acceptance_criteria=excluded.acceptance_criteria,
                    explanation=excluded.explanation,
                    examples=excluded.examples,
                    priority=excluded.priority,
                    risk=excluded.risk,
                    technical_notes=excluded.technical_notes,
                    implementation_notes=excluded.implementation_notes,
                    test_guidance=excluded.test_guidance,
                    updated_at=datetime('now')",
            )
            .bind(&row.id)
            .bind(&row.module_id)
            .bind(row.parent_capability_id.as_deref())
            .bind(row.level)
            .bind(&row.node_kind)
            .bind(row.sort_order)
            .bind(&row.name)
            .bind(&row.description)
            .bind(&row.acceptance_criteria)
            .bind(&row.explanation)
            .bind(&row.examples)
            .bind(&row.priority)
            .bind(&row.risk)
            .bind(&row.technical_notes)
            .bind(&row.implementation_notes)
            .bind(&row.test_guidance)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        let capability_delta = chunk
            .iter()
            .filter(|row| row.node_kind == "capability")
            .count()
            .try_into()
            .unwrap_or(i64::MAX);
        let feature_delta = chunk
            .iter()
            .filter(|row| row.node_kind == "feature")
            .count()
            .try_into()
            .unwrap_or(i64::MAX);
        update_job_progress(
            pool,
            job_id,
            chunk.len().try_into().unwrap_or(i64::MAX),
            0,
            0,
            capability_delta,
            feature_delta,
            0,
        )
        .await?;
    }
    Ok(())
}

pub async fn upsert_work_items(
    pool: &SqlitePool,
    job_id: &str,
    rows: &[BulkImportWorkItemRow],
    batch_size: usize,
) -> Result<(), AppError> {
    for chunk in rows.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for row in chunk {
            sqlx::query(
                "INSERT INTO work_items (
                    id, product_id, module_id, capability_id, source_node_id, source_node_type,
                    parent_work_item_id, title, problem_statement, description,
                    acceptance_criteria, constraints, work_item_type, priority, complexity,
                    status, sort_order
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    product_id=excluded.product_id,
                    module_id=excluded.module_id,
                    capability_id=excluded.capability_id,
                    source_node_id=excluded.source_node_id,
                    source_node_type=excluded.source_node_type,
                    parent_work_item_id=excluded.parent_work_item_id,
                    title=excluded.title,
                    problem_statement=excluded.problem_statement,
                    description=excluded.description,
                    acceptance_criteria=excluded.acceptance_criteria,
                    constraints=excluded.constraints,
                    work_item_type=excluded.work_item_type,
                    priority=excluded.priority,
                    complexity=excluded.complexity,
                    status=excluded.status,
                    sort_order=excluded.sort_order,
                    updated_at=datetime('now')",
            )
            .bind(&row.id)
            .bind(&row.product_id)
            .bind(row.module_id.as_deref())
            .bind(row.capability_id.as_deref())
            .bind(row.source_node_id.as_deref())
            .bind(row.source_node_type.as_deref())
            .bind(row.parent_work_item_id.as_deref())
            .bind(&row.title)
            .bind(&row.problem_statement)
            .bind(&row.description)
            .bind(&row.acceptance_criteria)
            .bind(&row.constraints)
            .bind(&row.work_item_type)
            .bind(&row.priority)
            .bind(&row.complexity)
            .bind(&row.status)
            .bind(row.sort_order)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        update_job_progress(
            pool,
            job_id,
            chunk.len().try_into().unwrap_or(i64::MAX),
            0,
            0,
            0,
            0,
            chunk.len().try_into().unwrap_or(i64::MAX),
        )
        .await?;
    }
    Ok(())
}

pub async fn upsert_all(
    pool: &SqlitePool,
    job_id: &str,
    rows: &BulkImportRows,
    batch_size: usize,
) -> Result<(), AppError> {
    upsert_products(pool, job_id, &rows.products, batch_size).await?;
    upsert_product_areas(pool, job_id, &rows.product_areas, batch_size).await?;
    upsert_capabilities(pool, job_id, &rows.capabilities, batch_size).await?;
    upsert_work_items(pool, job_id, &rows.work_items, batch_size).await?;
    Ok(())
}

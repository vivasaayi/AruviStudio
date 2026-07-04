use crate::domain::bulk_import::{
    BulkImportCapabilityRow, BulkImportProductAreaRow, BulkImportProductRow, BulkImportRows,
    BulkImportWorkItemRow,
};
use crate::error::AppError;
pub use crate::persistence::bulk_import_job_repo::{
    add_job_error, create_job, get_job, list_job_errors, list_jobs, mark_interrupted_jobs,
    mark_job_completed, mark_job_failed, mark_job_running, update_job_progress,
    UpdateBulkImportJobProgressInput,
};
use sqlx::SqlitePool;

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
            UpdateBulkImportJobProgressInput {
                job_id,
                processed_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                product_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                product_area_delta: 0,
                capability_delta: 0,
                feature_delta: 0,
                work_item_delta: 0,
            },
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
                "INSERT INTO product_areas (
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
            UpdateBulkImportJobProgressInput {
                job_id,
                processed_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                product_delta: 0,
                product_area_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                capability_delta: 0,
                feature_delta: 0,
                work_item_delta: 0,
            },
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
                    id, product_area_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    product_area_id=excluded.product_area_id,
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
            .bind(&row.product_area_id)
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
            UpdateBulkImportJobProgressInput {
                job_id,
                processed_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                product_delta: 0,
                product_area_delta: 0,
                capability_delta,
                feature_delta,
                work_item_delta: 0,
            },
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
                    id, product_id, product_area_id, capability_id, source_node_id, source_node_type,
                    parent_work_item_id, title, problem_statement, description,
                    acceptance_criteria, constraints, work_item_type, priority, complexity,
                    status, sort_order
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    product_id=excluded.product_id,
                    product_area_id=excluded.product_area_id,
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
            .bind(row.product_area_id.as_deref())
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
            UpdateBulkImportJobProgressInput {
                job_id,
                processed_delta: chunk.len().try_into().unwrap_or(i64::MAX),
                product_delta: 0,
                product_area_delta: 0,
                capability_delta: 0,
                feature_delta: 0,
                work_item_delta: chunk.len().try_into().unwrap_or(i64::MAX),
            },
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

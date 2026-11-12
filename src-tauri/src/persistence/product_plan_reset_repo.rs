use crate::domain::product::ProductPlanResetResult;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn reset_product_plan(
    pool: &SqlitePool,
    product_id: &str,
    delete_delivery: bool,
) -> Result<ProductPlanResetResult, AppError> {
    reset_product_plan_checked(pool, product_id, delete_delivery, None).await
}

pub async fn reset_product_plan_checked(
    pool: &SqlitePool, product_id: &str, delete_delivery: bool, expected_name: Option<&str>,
) -> Result<ProductPlanResetResult, AppError> {
    let mut tx = pool.begin().await?;
    // Acquire a write lock before counting so the result describes the same snapshot that is reset.
    sqlx::query("UPDATE products SET updated_at=updated_at WHERE id=?").bind(product_id).execute(&mut *tx).await?;
    let product_exists: Option<String> = sqlx::query_scalar("SELECT name FROM products WHERE id=?")
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?;
    if product_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Product {product_id} not found"
        )));
    }

    if expected_name.is_some_and(|expected| product_exists.as_deref() != Some(expected)) {
        return Err(AppError::Validation("Product name changed. Refresh before resetting its plan.".into()));
    }

    let product_areas_deleted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM product_areas WHERE product_id=?")
            .bind(product_id)
            .fetch_one(&mut *tx)
            .await?;
    let capabilities_deleted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM capabilities c
         JOIN product_areas pa ON pa.id=c.product_area_id
         WHERE pa.product_id=?",
    )
    .bind(product_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut result = ProductPlanResetResult {
        product_id: product_id.to_string(),
        product_areas_deleted,
        capabilities_deleted,
        work_items_deleted: 0,
        agent_work_runs_deleted: 0,
        agent_work_items_deleted: 0,
        agent_work_events_deleted: 0,
        agent_work_evidence_deleted: 0,
        agent_work_dependencies_deleted: 0,
        agent_work_locks_deleted: 0,
        agent_work_batches_deleted: 0,
    };

    if delete_delivery {
        result.work_items_deleted =
            sqlx::query_scalar("SELECT COUNT(*) FROM work_items WHERE product_id=?")
                .bind(product_id)
                .fetch_one(&mut *tx)
                .await?;
        result.agent_work_runs_deleted =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_work_runs WHERE product_id=?")
                .bind(product_id)
                .fetch_one(&mut *tx)
                .await?;
        result.agent_work_items_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_items item
             JOIN agent_work_runs run ON run.id=item.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
        result.agent_work_events_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_events event
             JOIN agent_work_runs run ON run.id=event.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
        result.agent_work_evidence_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_evidence evidence
             JOIN agent_work_runs run ON run.id=evidence.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
        result.agent_work_dependencies_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_dependencies dependency
             JOIN agent_work_runs run ON run.id=dependency.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
        result.agent_work_locks_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_locks l
             JOIN agent_work_runs run ON run.id=l.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
        result.agent_work_batches_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_batches batch
             JOIN agent_work_runs run ON run.id=batch.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;
    }

    if delete_delivery {
        sqlx::query("DELETE FROM agent_work_runs WHERE product_id=?")
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM work_items WHERE product_id=?")
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
    }
    let area_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM product_areas WHERE product_id=?")
        .bind(product_id).fetch_all(&mut *tx).await?;
    for area_id in area_ids {
        crate::persistence::hierarchy_delete_repo::delete_hierarchy_node_in_transaction(&mut tx, "product_area", &area_id, None).await?;
    }
    sqlx::query("UPDATE products SET updated_at=datetime('now') WHERE id=?")
        .bind(product_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(result)
}

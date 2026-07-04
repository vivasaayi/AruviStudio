use crate::domain::product::ProductPlanResetResult;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn reset_product_plan(
    pool: &SqlitePool,
    product_id: &str,
    delete_delivery: bool,
) -> Result<ProductPlanResetResult, AppError> {
    let product_exists: Option<String> = sqlx::query_scalar("SELECT id FROM products WHERE id=?")
        .bind(product_id)
        .fetch_optional(pool)
        .await?;
    if product_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Product {product_id} not found"
        )));
    }

    let product_areas_deleted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM product_areas WHERE product_id=?")
            .bind(product_id)
            .fetch_one(pool)
            .await?;
    let capabilities_deleted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM capabilities c
         JOIN product_areas pa ON pa.id=c.product_area_id
         WHERE pa.product_id=?",
    )
    .bind(product_id)
    .fetch_one(pool)
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
                .fetch_one(pool)
                .await?;
        result.agent_work_runs_deleted =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_work_runs WHERE product_id=?")
                .bind(product_id)
                .fetch_one(pool)
                .await?;
        result.agent_work_items_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_items item
             JOIN agent_work_runs run ON run.id=item.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
        result.agent_work_events_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_events event
             JOIN agent_work_runs run ON run.id=event.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
        result.agent_work_evidence_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_evidence evidence
             JOIN agent_work_runs run ON run.id=evidence.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
        result.agent_work_dependencies_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_dependencies dependency
             JOIN agent_work_runs run ON run.id=dependency.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
        result.agent_work_locks_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_locks l
             JOIN agent_work_runs run ON run.id=l.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
        result.agent_work_batches_deleted = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM agent_work_batches batch
             JOIN agent_work_runs run ON run.id=batch.run_id
             WHERE run.product_id=?",
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;
    }

    let mut tx = pool.begin().await?;
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
    sqlx::query("DELETE FROM product_areas WHERE product_id=?")
        .bind(product_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE products SET updated_at=datetime('now') WHERE id=?")
        .bind(product_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(result)
}

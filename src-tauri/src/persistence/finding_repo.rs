use crate::domain::finding::Finding;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn list_work_item_findings(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Vec<Finding>, AppError> {
    sqlx::query_as::<_, Finding>("SELECT id,work_item_id,source_agent_run_id,category,severity,title,description,status,is_blocking,linked_followup_work_item_id,created_at,updated_at FROM findings WHERE work_item_id=? ORDER BY created_at DESC")
        .bind(work_item_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

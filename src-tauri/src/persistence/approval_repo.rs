use crate::domain::approval::Approval;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_approval(
    pool: &SqlitePool,
    id: &str,
    work_item_id: &str,
    workflow_run_id: Option<&str>,
    approval_type: &str,
    status: &str,
    notes: &str,
) -> Result<Approval, AppError> {
    let acted_at = if status != "pending" {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };
    sqlx::query_as::<_, Approval>("INSERT INTO approvals (id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at) VALUES (?,?,?,?,?,?,?) RETURNING id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at,created_at")
        .bind(id).bind(work_item_id).bind(workflow_run_id).bind(approval_type).bind(status).bind(notes).bind(acted_at)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_approvals(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Vec<Approval>, AppError> {
    sqlx::query_as::<_, Approval>("SELECT id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at,created_at FROM approvals WHERE work_item_id=? ORDER BY created_at DESC")
        .bind(work_item_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

use crate::{domain::approval::Approval, error::AppError};
use sqlx::SqlitePool;
use crate::services::work_item_service::WorkItemBaseline;

/// Update the task and its decision record together. The optional baseline is
/// used by the browser to reject stale decisions and active executions.
pub(crate) async fn record_task_decision(
    pool: &SqlitePool,
    work_item_id: &str,
    approved: bool,
    notes: &str,
    expected: Option<&WorkItemBaseline>,
) -> Result<Approval, AppError> {
    let mut transaction = pool.begin().await?;
    let result = sqlx::query("UPDATE work_items SET status=?,updated_at=datetime('now') WHERE id=? AND (?=0 OR (status=? AND updated_at=? AND title=? AND description=? AND problem_statement=? AND acceptance_criteria=? AND constraints=? AND priority=? AND complexity=? AND work_item_type=? AND product_id IS ? AND product_area_id IS ? AND capability_id IS ? AND source_node_id IS ? AND source_node_type IS ? AND parent_work_item_id IS ? AND repo_override_id IS ? AND active_repo_id IS ? AND branch_name IS ? AND sort_order=? AND NOT EXISTS (SELECT 1 FROM workflow_runs WHERE work_item_id=work_items.id AND status IN ('running','paused') AND current_stage NOT IN ('done','failed','cancelled')) AND NOT EXISTS (SELECT 1 FROM external_cli_runs WHERE work_item_id=work_items.id AND status='running')))")
        .bind(if approved { "approved" } else { "draft" }).bind(work_item_id).bind(expected.is_some())
        .bind(expected.map(|value| value.status.as_str())).bind(expected.map(|value| value.updated_at.as_str()))
        .bind(expected.map(|value| value.title.as_str())).bind(expected.map(|value| value.description.as_str()))
        .bind(expected.map(|value| value.problem_statement.as_str())).bind(expected.map(|value| value.acceptance_criteria.as_str())).bind(expected.map(|value| value.constraints.as_str()))
        .bind(expected.map(|value| value.priority.as_str())).bind(expected.map(|value| value.complexity.as_str())).bind(expected.map(|value| value.work_item_type.as_str()))
        .bind(expected.and_then(|value| value.product_id.as_deref())).bind(expected.and_then(|value| value.product_area_id.as_deref())).bind(expected.and_then(|value| value.capability_id.as_deref()))
        .bind(expected.and_then(|value| value.source_node_id.as_deref())).bind(expected.and_then(|value| value.source_node_type.as_deref())).bind(expected.and_then(|value| value.parent_work_item_id.as_deref()))
        .bind(expected.and_then(|value| value.repo_override_id.as_deref())).bind(expected.and_then(|value| value.active_repo_id.as_deref())).bind(expected.and_then(|value| value.branch_name.as_deref())).bind(expected.map(|value| value.sort_order))
        .execute(&mut *transaction).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Validation("Work item changed, was removed, or has an active workflow or CLI run; refresh and review".into()));
    }
    let approval = sqlx::query_as::<_, Approval>("INSERT INTO approvals (id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at) VALUES (?,?,NULL,'task_approval',?,?,?) RETURNING id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at,created_at")
        .bind(uuid::Uuid::new_v4().to_string()).bind(work_item_id)
        .bind(if approved { "approved" } else { "rejected" }).bind(notes).bind(chrono::Utc::now().to_rfc3339())
        .fetch_one(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(approval)
}

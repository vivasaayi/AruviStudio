use crate::{domain::approval::Approval, error::AppError};
use sqlx::{SqliteConnection, SqlitePool};
use crate::services::work_item_service::WorkItemBaseline;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BatchDecisionItem {
    pub id: String,
    pub expected: WorkItemBaseline,
}

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
    let approval = record_task_decision_in_conn(&mut transaction, work_item_id, approved, notes, expected).await?;
    transaction.commit().await?;
    Ok(approval)
}

async fn record_task_decision_in_conn(
    conn: &mut SqliteConnection, work_item_id: &str, approved: bool, notes: &str,
    expected: Option<&WorkItemBaseline>,
) -> Result<Approval, AppError> {
    let result = sqlx::query("UPDATE work_items SET status=?,updated_at=datetime('now') WHERE id=? AND (?=0 OR (status=? AND updated_at=? AND title=? AND description=? AND problem_statement=? AND acceptance_criteria=? AND constraints=? AND priority=? AND complexity=? AND work_item_type=? AND product_id IS ? AND product_area_id IS ? AND capability_id IS ? AND source_node_id IS ? AND source_node_type IS ? AND parent_work_item_id IS ? AND repo_override_id IS ? AND active_repo_id IS ? AND branch_name IS ? AND sort_order=? AND NOT EXISTS (SELECT 1 FROM workflow_runs WHERE work_item_id=work_items.id AND status IN ('running','paused') AND current_stage NOT IN ('done','failed','cancelled')) AND NOT EXISTS (SELECT 1 FROM external_cli_runs WHERE work_item_id=work_items.id AND status='running')))")
        .bind(if approved { "approved" } else { "draft" }).bind(work_item_id).bind(expected.is_some())
        .bind(expected.map(|value| value.status.as_str())).bind(expected.map(|value| value.updated_at.as_str()))
        .bind(expected.map(|value| value.title.as_str())).bind(expected.map(|value| value.description.as_str()))
        .bind(expected.map(|value| value.problem_statement.as_str())).bind(expected.map(|value| value.acceptance_criteria.as_str())).bind(expected.map(|value| value.constraints.as_str()))
        .bind(expected.map(|value| value.priority.as_str())).bind(expected.map(|value| value.complexity.as_str())).bind(expected.map(|value| value.work_item_type.as_str()))
        .bind(expected.and_then(|value| value.product_id.as_deref())).bind(expected.and_then(|value| value.product_area_id.as_deref())).bind(expected.and_then(|value| value.capability_id.as_deref()))
        .bind(expected.and_then(|value| value.source_node_id.as_deref())).bind(expected.and_then(|value| value.source_node_type.as_deref())).bind(expected.and_then(|value| value.parent_work_item_id.as_deref()))
        .bind(expected.and_then(|value| value.repo_override_id.as_deref())).bind(expected.and_then(|value| value.active_repo_id.as_deref())).bind(expected.and_then(|value| value.branch_name.as_deref())).bind(expected.map(|value| value.sort_order))
        .execute(&mut *conn).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Validation("Work item changed, was removed, or has an active workflow or CLI run; refresh and review".into()));
    }
    let approval = sqlx::query_as::<_, Approval>("INSERT INTO approvals (id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at) VALUES (?,?,NULL,'task_approval',?,?,?) RETURNING id,work_item_id,workflow_run_id,approval_type,status,notes,acted_at,created_at")
        .bind(uuid::Uuid::new_v4().to_string()).bind(work_item_id)
        .bind(if approved { "approved" } else { "rejected" }).bind(notes).bind(chrono::Utc::now().to_rfc3339())
        .fetch_one(&mut *conn).await?;
    Ok(approval)
}

/// Decide up to one visible page atomically. Any stale or cross-product item
/// rolls back every status update and approval in the batch.
pub(crate) async fn record_batch_task_decisions(
    pool: &SqlitePool, product_id: &str, approved: bool, notes: &str, items: &[BatchDecisionItem],
) -> Result<Vec<Approval>, AppError> {
    if product_id.trim().is_empty() || items.is_empty() || items.len() > 100 {
        return Err(AppError::Validation("Select between 1 and 100 work items for one product".into()));
    }
    if notes.len() > 4000 || (!approved && notes.trim().is_empty()) {
        return Err(AppError::Validation("A rejection reason is required and notes must be at most 4000 bytes".into()));
    }
    let mut ids = std::collections::HashSet::new();
    if items.iter().any(|item| item.id.trim().is_empty() || !ids.insert(&item.id) || item.expected.product_id.as_deref() != Some(product_id)) {
        return Err(AppError::Validation("Work items must be unique and belong to the selected product".into()));
    }
    let mut transaction = pool.begin().await?;
    let mut approvals = Vec::with_capacity(items.len());
    for item in items {
        approvals.push(record_task_decision_in_conn(&mut transaction, &item.id, approved, notes, Some(&item.expected)).await?);
    }
    transaction.commit().await?;
    Ok(approvals)
}

pub(crate) fn schedule_batch_workflows(state: &crate::state::AppState, ids: Vec<String>) {
    let service = state.workflow_service.clone();
    tokio::spawn(async move {
        for id in ids {
            if let Err(error) = service.lock().await.start_work_item_workflow(&id).await {
                tracing::error!(work_item_id=%id, error=%error, "auto-start after batch approval failed");
            }
        }
    });
}

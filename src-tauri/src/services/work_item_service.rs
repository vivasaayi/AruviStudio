// Work item service - business logic for work items and status transitions
use crate::{domain::work_item::WorkItem, error::AppError, persistence::work_item_repo, state::AppState};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct WorkspaceBaseline {
    pub repo_override_id: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteBaseline {
    pub title: String,
    pub description: String,
    pub problem_statement: String,
    pub acceptance_criteria: String,
    pub constraints: String,
    pub status: String,
    pub updated_at: String,
    pub priority: String,
    pub complexity: String,
    pub work_item_type: String,
    pub product_id: Option<String>,
    pub product_area_id: Option<String>,
    pub capability_id: Option<String>,
    pub source_node_id: Option<String>,
    pub source_node_type: Option<String>,
    pub parent_work_item_id: Option<String>,
    pub repo_override_id: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch_name: Option<String>,
    pub sort_order: i64,
}

/// Delete a work item only when it and its direct children have no active workflow.
pub(crate) async fn delete_work_item(state: &AppState, id: &str, expected: Option<&DeleteBaseline>) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM work_items WHERE id=? AND NOT EXISTS (SELECT 1 FROM workflow_runs r JOIN work_items w ON w.id=r.work_item_id WHERE (w.id=? OR w.parent_work_item_id=?) AND r.status IN ('running','paused') AND r.current_stage NOT IN ('done','failed','cancelled')) AND NOT EXISTS (SELECT 1 FROM external_cli_runs c JOIN work_items w ON w.id=c.work_item_id WHERE (w.id=work_items.id OR w.parent_work_item_id=work_items.id) AND c.status='running') AND (?=0 OR (title=? AND description=? AND problem_statement=? AND acceptance_criteria=? AND constraints=? AND status=? AND updated_at=? AND priority=? AND complexity=? AND work_item_type=? AND product_id IS ? AND product_area_id IS ? AND capability_id IS ? AND source_node_id IS ? AND source_node_type IS ? AND parent_work_item_id IS ? AND repo_override_id IS ? AND active_repo_id IS ? AND branch_name IS ? AND sort_order=?))")
        .bind(id).bind(id).bind(id).bind(expected.is_some())
        .bind(expected.map(|v| v.title.as_str())).bind(expected.map(|v| v.description.as_str()))
        .bind(expected.map(|v| v.problem_statement.as_str())).bind(expected.map(|v| v.acceptance_criteria.as_str()))
        .bind(expected.map(|v| v.constraints.as_str())).bind(expected.map(|v| v.status.as_str())).bind(expected.map(|v| v.updated_at.as_str()))
        .bind(expected.map(|v| v.priority.as_str())).bind(expected.map(|v| v.complexity.as_str())).bind(expected.map(|v| v.work_item_type.as_str()))
        .bind(expected.and_then(|v| v.product_id.as_deref())).bind(expected.and_then(|v| v.product_area_id.as_deref()))
        .bind(expected.and_then(|v| v.capability_id.as_deref())).bind(expected.and_then(|v| v.source_node_id.as_deref())).bind(expected.and_then(|v| v.source_node_type.as_deref())).bind(expected.and_then(|v| v.parent_work_item_id.as_deref()))
        .bind(expected.and_then(|v| v.repo_override_id.as_deref())).bind(expected.and_then(|v| v.active_repo_id.as_deref())).bind(expected.and_then(|v| v.branch_name.as_deref())).bind(expected.map(|v| v.sort_order))
        .execute(&state.db).await?;
    if result.rows_affected() != 1 { return Err(AppError::Validation("Work item changed, was removed, or it or a child has an active workflow or CLI run; refresh and review".into())); }
    Ok(())
}

/// Assign a registered workspace without changing an active workflow's target.
pub(crate) async fn assign_workspace(
    state: &AppState, id: &str, repository_id: Option<&str>, branch_name: Option<&str>,
    expected: Option<&WorkspaceBaseline>,
) -> Result<WorkItem, AppError> {
    let repository_id = repository_id.map(str::trim).filter(|s| !s.is_empty());
    let branch_name = branch_name.map(str::trim).filter(|s| !s.is_empty());
    if repository_id.is_none() && branch_name.is_some() {
        return Err(AppError::Validation("A branch requires a workspace".into()));
    }
    if branch_name.is_some_and(|branch| !git2::Reference::is_valid_name(&format!("refs/heads/{branch}"))) {
        return Err(AppError::Validation("Invalid workspace branch name".into()));
    }
    let result = sqlx::query("UPDATE work_items SET repo_override_id=?,active_repo_id=?,branch_name=?,updated_at=datetime('now') WHERE id=? AND (? IS NULL OR EXISTS (SELECT 1 FROM repositories WHERE id=?)) AND NOT EXISTS (SELECT 1 FROM workflow_runs WHERE work_item_id=? AND status IN ('running','paused') AND current_stage NOT IN ('done','failed','cancelled')) AND NOT EXISTS (SELECT 1 FROM external_cli_runs c WHERE c.work_item_id=work_items.id AND c.status='running') AND (?=0 OR (repo_override_id IS ? AND active_repo_id IS ? AND branch_name IS ?))")
        .bind(repository_id).bind(repository_id).bind(branch_name).bind(id).bind(repository_id).bind(repository_id).bind(id)
        .bind(expected.is_some()).bind(expected.and_then(|value| value.repo_override_id.as_deref()))
        .bind(expected.and_then(|value| value.active_repo_id.as_deref())).bind(expected.and_then(|value| value.branch_name.as_deref()))
        .execute(&state.db).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Validation("Work item or workspace changed, is unavailable, or has an active workflow or CLI run; refresh and review".into()));
    }
    work_item_repo::get_work_item(&state.db, id).await
}

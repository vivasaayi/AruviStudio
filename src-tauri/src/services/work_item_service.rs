// Work item service - business logic for work items and status transitions
use crate::{domain::work_item::WorkItem, error::AppError, persistence::work_item_repo, state::AppState};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct WorkspaceBaseline {
    pub repo_override_id: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch_name: Option<String>,
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
    let result = sqlx::query("UPDATE work_items SET repo_override_id=?,active_repo_id=?,branch_name=?,updated_at=datetime('now') WHERE id=? AND (? IS NULL OR EXISTS (SELECT 1 FROM repositories WHERE id=?)) AND NOT EXISTS (SELECT 1 FROM workflow_runs WHERE work_item_id=? AND status IN ('running','paused') AND current_stage NOT IN ('done','failed','cancelled')) AND (?=0 OR (repo_override_id IS ? AND active_repo_id IS ? AND branch_name IS ?))")
        .bind(repository_id).bind(repository_id).bind(branch_name).bind(id).bind(repository_id).bind(repository_id).bind(id)
        .bind(expected.is_some()).bind(expected.and_then(|value| value.repo_override_id.as_deref()))
        .bind(expected.and_then(|value| value.active_repo_id.as_deref())).bind(expected.and_then(|value| value.branch_name.as_deref()))
        .execute(&state.db).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Validation("Work item or workspace changed, is unavailable, or has an active workflow; refresh and review".into()));
    }
    work_item_repo::get_work_item(&state.db, id).await
}

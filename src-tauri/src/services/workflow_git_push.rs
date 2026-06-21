use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::execution::git_ops::GitOperations;
use crate::persistence::{repository_repo, work_item_repo, workflow_repo};
use sqlx::SqlitePool;
use tracing::{info, warn};

pub(crate) async fn execute_git_push_stage(
    db: &SqlitePool,
    workflow_run_id: &str,
) -> Result<(), AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;

    if let Some(repo_id) = &work_item.active_repo_id {
        let repo = repository_repo::get_repository(db, repo_id).await?;
        let branch_name = work_item
            .branch_name
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let commit_message = format!("Implement work item: {}", work_item.title);

        match GitOperations::create_commit(&repo.local_path, &commit_message) {
            Ok(commit_id) => {
                GitOperations::push_to_remote(&repo.local_path, "origin", &branch_name)?;
                info!(
                    "Successfully pushed commit {} for work item {}",
                    commit_id, work_item.id
                );
            }
            Err(AppError::Internal(message)) if message.contains("No changes to commit") => {
                warn!(
                    work_item_id = %work_item.id,
                    repo_id = %repo.id,
                    "Skipping git push because no repository changes were produced for this work item"
                );
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

pub(crate) async fn push_workflow_changes(
    db: &SqlitePool,
    workflow_run: &WorkflowRun,
) -> Result<(), AppError> {
    let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;
    if let Some(repo_id) = &work_item.active_repo_id {
        let repo = repository_repo::get_repository(db, repo_id).await?;
        GitOperations::stage_all_changes(&repo.local_path)?;

        let commit_message = format!("Implement work item: {}", work_item.title);
        let commit_id = GitOperations::create_commit(&repo.local_path, &commit_message)?;

        GitOperations::push_to_remote(
            &repo.local_path,
            "origin",
            &work_item.branch_name.unwrap_or_else(|| "main".to_string()),
        )?;

        info!(
            "Successfully pushed commit {} for work item {}",
            commit_id, work_item.id
        );
    }

    Ok(())
}

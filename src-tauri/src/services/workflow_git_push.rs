use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::execution::git_ops::GitOperations;
use crate::persistence::{repository_repo, work_item_repo, workflow_repo};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

async fn queue_automatic_preview(
    db: &SqlitePool,
    repository_id: &str,
    product_id: Option<&str>,
    work_item_id: &str,
    workflow_run_id: &str,
    commit_sha: &str,
) -> Result<(), AppError> {
    let Some(product_id) = product_id else {
        return Ok(());
    };
    let binding = sqlx::query_as::<_, (String, String)>(
        "SELECT id,target_id FROM ci_target_bindings WHERE product_id=? AND repository_id=? AND auto_preview=1",
    )
    .bind(product_id)
    .bind(repository_id)
    .fetch_optional(db)
    .await?;
    let Some((binding_id, target_id)) = binding else {
        return Ok(());
    };
    let dispatch_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO ci_dispatches(id,binding_id,work_item_id,workflow_run_id,commit_sha) VALUES(?,?,?,?,?) ON CONFLICT(binding_id,commit_sha) DO NOTHING")
        .bind(&dispatch_id).bind(&binding_id).bind(work_item_id).bind(workflow_run_id).bind(commit_sha).execute(db).await?;
    let payload = serde_json::json!({"method":"enqueue","target":target_id,"commit":commit_sha,"key":format!("studio-{workflow_run_id}-{commit_sha}"),"context":{"trigger":"workflow-checkpoint","product_id":product_id,"work_item_id":work_item_id,"workflow_run_id":workflow_run_id}});
    match crate::commands::ci_commands::request(payload) {
        Ok(result) => {
            sqlx::query("UPDATE ci_dispatches SET status='queued',ci_run_id=?,error_message=NULL,updated_at=datetime('now') WHERE binding_id=? AND commit_sha=?")
                .bind(result["run_id"].as_i64()).bind(binding_id).bind(commit_sha).execute(db).await?;
            info!(work_item_id, commit_sha, "queued automatic Preview build");
        }
        Err(error) => {
            sqlx::query("UPDATE ci_dispatches SET error_message=?,updated_at=datetime('now') WHERE binding_id=? AND commit_sha=?")
                .bind(&error).bind(binding_id).bind(commit_sha).execute(db).await?;
            warn!(
                work_item_id,
                commit_sha, error, "Preview dispatch retained for retry"
            );
        }
    }
    Ok(())
}

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
                queue_automatic_preview(
                    db,
                    &repo.id,
                    work_item.product_id.as_deref(),
                    &work_item.id,
                    workflow_run_id,
                    &commit_id.to_string(),
                )
                .await?;
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
        queue_automatic_preview(
            db,
            &repo.id,
            work_item.product_id.as_deref(),
            &work_item.id,
            &workflow_run.id,
            &commit_id.to_string(),
        )
        .await?;
    }

    Ok(())
}

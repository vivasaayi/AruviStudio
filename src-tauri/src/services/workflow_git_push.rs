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
    let bindings = sqlx::query_as::<_, (String, String)>(
        "SELECT b.id,b.target_id FROM ci_target_bindings b JOIN repository_attachments a ON a.scope_type='product' AND a.scope_id=b.product_id AND a.repository_id=b.repository_id WHERE b.product_id=? AND b.repository_id=? AND b.auto_preview=1 AND b.target_id LIKE '%-preview' ORDER BY b.target_id",
    )
    .bind(product_id)
    .bind(repository_id)
    .fetch_all(db)
    .await?;
    if bindings.is_empty() {
        return Ok(());
    }
    for (binding_id, target_id) in bindings {
        let dispatch_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO ci_dispatches(id,binding_id,work_item_id,workflow_run_id,commit_sha) VALUES(?,?,?,?,?) ON CONFLICT(binding_id,commit_sha) DO NOTHING")
            .bind(&dispatch_id).bind(&binding_id).bind(work_item_id).bind(workflow_run_id).bind(commit_sha).execute(db).await?;
        let payload = serde_json::json!({"method":"enqueue","target":target_id,"commit":commit_sha,"key":format!("studio-{workflow_run_id}-{target_id}-{commit_sha}"),"context":{"trigger":"workflow-checkpoint","product_id":product_id,"repository_id":repository_id,"work_item_id":work_item_id,"workflow_run_id":workflow_run_id}});
        match crate::commands::ci_commands::request(payload) {
            Ok(result) => {
                sqlx::query("UPDATE ci_dispatches SET status='queued',ci_run_id=?,error_message=NULL,updated_at=datetime('now') WHERE binding_id=? AND commit_sha=?")
                    .bind(result["run_id"].as_i64()).bind(&binding_id).bind(commit_sha).execute(db).await?;
                info!(
                    work_item_id,
                    commit_sha, target_id, "queued automatic Preview build"
                );
            }
            Err(error) => {
                sqlx::query("UPDATE ci_dispatches SET error_message=?,updated_at=datetime('now') WHERE binding_id=? AND commit_sha=?")
                    .bind(&error).bind(&binding_id).bind(commit_sha).execute(db).await?;
                warn!(
                    work_item_id,
                    commit_sha, target_id, error, "Preview dispatch retained for retry"
                );
            }
        }
    }
    Ok(())
}

pub(crate) async fn is_retryable_git_delivery(db: &SqlitePool, workflow_run_id: &str) -> Result<bool, AppError> {
    let run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    if run.status != "failed" || run.current_stage != "failed" { return Ok(false); }
    let latest: Option<String> = sqlx::query_scalar("SELECT id FROM workflow_runs WHERE work_item_id=? ORDER BY rowid DESC LIMIT 1")
        .bind(&run.work_item_id).fetch_optional(db).await?;
    if latest.as_deref() != Some(workflow_run_id) { return Ok(false); }
    let failure_stage: Option<String> = sqlx::query_scalar("SELECT from_stage FROM workflow_stage_history WHERE workflow_run_id=? AND to_stage='failed' ORDER BY rowid DESC LIMIT 1")
        .bind(workflow_run_id).fetch_optional(db).await?;
    if failure_stage.as_deref() != Some("git_push") { return Ok(false); }
    Ok(crate::services::workflow_delivery_manifest::load(db, workflow_run_id).await?.is_some_and(|manifest| !manifest.files.is_empty()))
}

pub(crate) async fn execute_git_push_stage(db: &SqlitePool, workflow_run_id: &str) -> Result<(), AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;
    let repo_id = work_item.active_repo_id.as_deref().ok_or_else(|| AppError::Validation("No active repository is available for workflow delivery".to_string()))?;
    let repo = repository_repo::get_repository(db, repo_id).await?;
    let branch = work_item.branch_name.as_deref().unwrap_or("main");
    let mut manifest = crate::services::workflow_delivery_manifest::load(db, workflow_run_id).await?
        .ok_or_else(|| AppError::Validation("No workflow delivery manifest exists; review the repository and rerun coding before delivery".to_string()))?;
    if manifest.repository_id != repo_id { return Err(AppError::Validation("The workflow delivery repository changed after coding".to_string())); }
    let commit = match manifest.commit.clone() {
        Some(commit) => commit,
        None => {
            let message = format!("Implement work item: {}", work_item.title);
            let commit = GitOperations::create_workflow_commit(&repo.local_path, branch, &manifest, &message)?;
            manifest.commit = Some(commit.clone());
            if let Err(error) = crate::services::workflow_delivery_manifest::save(db, workflow_run_id, &manifest).await {
                return Err(AppError::Internal(format!("Created local commit {commit}, but could not record it; inspect the repository before retrying: {error}")));
            }
            commit
        }
    };
    GitOperations::push_to_remote(&repo.local_path, "origin", branch, &commit)?;
    info!(work_item_id = %work_item.id, commit_id = %commit, "Successfully pushed workflow commit");
    queue_automatic_preview(db, repo_id, work_item.product_id.as_deref(), &work_item.id, workflow_run_id, &commit).await?;
    Ok(())
}

pub(crate) async fn push_workflow_changes(db: &SqlitePool, workflow_run: &WorkflowRun) -> Result<(), AppError> {
    execute_git_push_stage(db, &workflow_run.id).await
}

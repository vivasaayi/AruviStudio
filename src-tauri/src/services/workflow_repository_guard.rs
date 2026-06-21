use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::persistence::{repository_repo, work_item_repo, workflow_repo};
use sqlx::SqlitePool;

fn stage_requires_repository(stage: &WorkflowStage) -> bool {
    matches!(
        stage,
        WorkflowStage::Coding
            | WorkflowStage::UnitTestGeneration
            | WorkflowStage::IntegrationTestGeneration
            | WorkflowStage::UiTestPlanning
            | WorkflowStage::QaValidation
            | WorkflowStage::SecurityReview
            | WorkflowStage::PerformanceReview
            | WorkflowStage::PushPreparation
            | WorkflowStage::GitPush
    )
}

fn stage_requires_git_repository(stage: &WorkflowStage) -> bool {
    matches!(
        stage,
        WorkflowStage::PushPreparation | WorkflowStage::GitPush
    )
}

pub(crate) async fn ensure_repository_ready_for_stage(
    db: &SqlitePool,
    workflow_run_id: &str,
    stage: &WorkflowStage,
) -> Result<(), AppError> {
    if !stage_requires_repository(stage) {
        return Ok(());
    }

    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;
    let resolved_repo = if let Some(repo_id) = work_item.active_repo_id.as_deref() {
        Some(repository_repo::get_repository(db, repo_id).await?)
    } else {
        repository_repo::resolve_repository_for_work_item(db, &work_item.id).await?
    };

    let Some(repo) = resolved_repo else {
        return Err(AppError::Validation(format!(
            "Repository readiness failed for stage {}. No repository is attached to work item, product area, or product scope. Complete a bootstrap setup work item or attach a repository before starting delivery.",
            stage.as_str()
        )));
    };

    if work_item.active_repo_id.as_deref() != Some(repo.id.as_str()) {
        sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
            .bind(&repo.id)
            .bind(&work_item.id)
            .execute(db)
            .await?;
    }

    let repo_path = std::path::Path::new(&repo.local_path);
    if !repo_path.exists() {
        return Err(AppError::Validation(format!(
            "Repository readiness failed for stage {}. Attached repository path does not exist: {}",
            stage.as_str(),
            repo.local_path
        )));
    }

    if stage_requires_git_repository(stage) && !repo_path.join(".git").exists() {
        return Err(AppError::Validation(format!(
            "Repository readiness failed for stage {}. Attached repository is not git initialized: {}",
            stage.as_str(),
            repo.local_path
        )));
    }

    Ok(())
}

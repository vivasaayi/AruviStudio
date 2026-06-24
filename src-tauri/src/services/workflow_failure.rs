use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::workflow_repo;
use sqlx::SqlitePool;

pub async fn mark_workflow_failed(
    db: &SqlitePool,
    workflow_run_id: &str,
    stage: &WorkflowStage,
    cause: &AppError,
) -> Result<(), AppError> {
    let reason = format!("Stage {} failed: {}", stage.as_str(), cause);
    let current = workflow_repo::get_workflow_run(db, workflow_run_id).await?;

    if current.current_stage != WorkflowStage::Failed.as_str() {
        workflow_repo::update_workflow_stage(db, workflow_run_id, WorkflowStage::Failed.as_str())
            .await?;

        let transition_id = uuid::Uuid::new_v4().to_string();
        workflow_repo::record_stage_transition(
            db,
            &transition_id,
            workflow_run_id,
            &current.current_stage,
            WorkflowStage::Failed.as_str(),
            TransitionTrigger::Automatic.as_str(),
            &reason,
        )
        .await?;
    }

    workflow_repo::update_workflow_lifecycle(db, workflow_run_id, "failed", Some(&reason), true)
        .await?;
    Ok(())
}

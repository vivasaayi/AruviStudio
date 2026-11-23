use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::workflow_repo;
use sqlx::SqlitePool;

/// Record an operator override atomically. This changes database state only; it
/// cannot cancel an already-running executor or undo its external effects.
pub async fn mark_workflow_failed_by_operator(
    db: &SqlitePool,
    workflow_run_id: &str,
    expected_stage: &str,
    expected_status: &str,
    reason: &str,
) -> Result<(), AppError> {
    if !matches!(expected_status, "running" | "paused")
        || matches!(expected_stage, "done" | "cancelled")
        || expected_stage.trim().is_empty()
        || reason.trim().is_empty() || reason.len() > 4000
    {
        return Err(AppError::Validation("A nonterminal workflow and a reason of at most 4000 bytes are required".into()));
    }
    let mut tx = db.begin().await?;
    let updated = sqlx::query("UPDATE workflow_runs SET current_stage='failed',status='failed',error_message=?,ended_at=datetime('now'),updated_at=datetime('now') WHERE id=? AND current_stage=? AND status=?")
        .bind(reason).bind(workflow_run_id).bind(expected_stage).bind(expected_status)
        .execute(&mut *tx).await?;
    if updated.rows_affected() != 1 {
        return Err(AppError::Validation("Workflow changed or is no longer active; refresh before continuing".into()));
    }
    // A legacy partially recorded failure may already have a failed stage.
    // Reconcile its active status without inventing a failed -> failed transition.
    if expected_stage != "failed" {
        sqlx::query("INSERT INTO workflow_stage_history (id,workflow_run_id,from_stage,to_stage,trigger,notes,transitioned_at) VALUES (?,?,?,'failed','user_override',?,datetime('now'))")
            .bind(uuid::Uuid::new_v4().to_string()).bind(workflow_run_id).bind(expected_stage).bind(reason)
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn mark_workflow_failed(
    db: &SqlitePool,
    workflow_run_id: &str,
    stage: &WorkflowStage,
    cause: &AppError,
) -> Result<(), AppError> {
    let reason = format!("Stage {} failed: {}", stage.as_str(), cause);
    let current = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    if matches!(current.current_stage.as_str(), "done" | "cancelled") {
        return Err(AppError::Validation("Workflow is already terminal; refresh before recording failure".into()));
    }

    // A delayed executor must not fail a stage that another actor has already
    // advanced to. The transition below checks the same expected stage again
    // atomically, in case it changes after this read.
    if current.current_stage != WorkflowStage::Failed.as_str()
        && current.current_stage != stage.as_str()
    {
        return Err(AppError::Validation("Workflow stage changed before failure was recorded; refresh before continuing".into()));
    }

    if current.current_stage != WorkflowStage::Failed.as_str() {
        workflow_repo::commit_workflow_transition(
            db,
            workflow_run_id,
            &current.current_stage,
            WorkflowStage::Failed.as_str(),
            "failed",
            Some(&reason),
            true,
            TransitionTrigger::Automatic.as_str(),
            &reason,
        )
        .await?;
    } else if current.status == "running" {
        // Reconcile an older partially recorded failure without changing a
        // completed or cancelled run that another actor already finalized.
        let result = sqlx::query("UPDATE workflow_runs SET status='failed',error_message=?,ended_at=datetime('now'),updated_at=datetime('now') WHERE id=? AND current_stage='failed' AND status='running'")
            .bind(&reason).bind(workflow_run_id).execute(db).await?;
        if result.rows_affected() != 1 {
            return Err(AppError::Validation("Workflow changed while recording failure; refresh before continuing".into()));
        }
    }
    Ok(())
}

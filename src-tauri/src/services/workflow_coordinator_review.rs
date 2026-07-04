use crate::domain::agent::AgentRunStatus;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::workflow_repo;
use crate::services::{agent_service, workflow_coordinator_policy, workflow_stage_transition};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub async fn prepare_coordinator_review(
    db: &SqlitePool,
    workflow_run_id: &str,
    target_stage: WorkflowStage,
) -> Result<(), AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
        .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;

    workflow_repo::set_pending_stage_name(db, workflow_run_id, Some(target_stage.as_str())).await?;
    workflow_stage_transition::transition_stage(
        db,
        workflow_run_id,
        current_stage,
        WorkflowStage::CoordinatorReview,
        TransitionTrigger::Automatic,
        format!("Coordinator review before {}", target_stage.as_str()),
    )
    .await
}

pub async fn execute_coordinator_review(
    db: &SqlitePool,
    agent_service: &Arc<Mutex<agent_service::AgentService>>,
    workflow_run_id: &str,
) -> Result<Option<WorkflowStage>, AppError> {
    info!(workflow_run_id = %workflow_run_id, "Executing coordinator review stage");
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let pending_stage_name = workflow_run.pending_stage_name.clone().ok_or_else(|| {
        AppError::Validation("Coordinator review requires a pending target stage".to_string())
    })?;
    let target_stage = WorkflowStage::from_str(&pending_stage_name)
        .map_err(|e| AppError::Validation(format!("Invalid pending workflow stage: {}", e)))?;

    if !workflow_coordinator_policy::has_active_coordinator_for_workflow(db, workflow_run_id)
        .await?
    {
        warn!(
            workflow_run_id = %workflow_run_id,
            target_stage = %target_stage.as_str(),
            "No active coordinator found at coordinator review time; bypassing to pending stage"
        );
        workflow_repo::set_pending_stage_name(db, workflow_run_id, None).await?;
        workflow_stage_transition::transition_stage(
            db,
            workflow_run_id,
            WorkflowStage::CoordinatorReview,
            target_stage.clone(),
            TransitionTrigger::Automatic,
            format!(
                "Coordinator unavailable; bypassed to {}",
                target_stage.as_str()
            ),
        )
        .await?;
        return Ok(Some(target_stage));
    }

    let agent_run = {
        let agent_service = agent_service.lock().await;
        agent_service
            .run_agent_for_stage(workflow_run_id, "coordinator_review")
            .await?
    };

    if agent_run.status == AgentRunStatus::Completed {
        workflow_repo::set_pending_stage_name(db, workflow_run_id, None).await?;
        workflow_stage_transition::transition_stage(
            db,
            workflow_run_id,
            WorkflowStage::CoordinatorReview,
            target_stage.clone(),
            TransitionTrigger::AgentCompletion,
            format!("Coordinator review completed for {}", target_stage.as_str()),
        )
        .await?;
        return Ok(Some(target_stage));
    }

    Ok(None)
}

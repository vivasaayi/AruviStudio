use crate::domain::agent::AgentRunStatus;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::services::{agent_service, workflow_stage_transition};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

pub async fn run_agent_stage_and_transition(
    db: &SqlitePool,
    agent_service: &Arc<Mutex<agent_service::AgentService>>,
    workflow_run_id: &str,
    agent_stage_name: &str,
    from_stage: WorkflowStage,
    to_stage: WorkflowStage,
    completion_notes: &str,
) -> Result<bool, AppError> {
    let agent_run = {
        let agent_service = agent_service.lock().await;
        agent_service
            .run_agent_for_stage(workflow_run_id, agent_stage_name)
            .await?
    };
    debug!(
        workflow_run_id = %workflow_run_id,
        agent_run_id = %agent_run.id,
        agent_run_status = ?agent_run.status,
        agent_stage = %agent_stage_name,
        "Completed agent run for workflow stage"
    );

    if agent_run.status != AgentRunStatus::Completed {
        debug!(
            workflow_run_id = %workflow_run_id,
            agent_run_status = ?agent_run.status,
            from_stage = %from_stage.as_str(),
            "Agent run not completed; staying in current workflow stage"
        );
        return Ok(false);
    }

    workflow_stage_transition::transition_stage(
        db,
        workflow_run_id,
        from_stage,
        to_stage,
        TransitionTrigger::AgentCompletion,
        completion_notes.to_string(),
    )
    .await?;
    Ok(true)
}

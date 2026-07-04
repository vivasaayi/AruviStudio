use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::{agent_repo, workflow_repo};
use crate::workflows::transitions;
use sqlx::SqlitePool;

pub(crate) async fn transition_stage(
    db: &SqlitePool,
    workflow_run_id: &str,
    from: WorkflowStage,
    to: WorkflowStage,
    trigger: TransitionTrigger,
    notes: String,
) -> Result<(), AppError> {
    let notes = annotate_transition_notes(db, workflow_run_id, &to, notes).await?;
    tracing::info!(
        "Transitioning workflow {} from {} to {} ({})",
        workflow_run_id,
        from.as_str(),
        to.as_str(),
        notes
    );

    if !transitions::is_valid_transition(&from, &to) {
        return Err(AppError::Validation(format!(
            "Invalid transition from {} to {}",
            from.as_str(),
            to.as_str()
        )));
    }

    workflow_repo::update_workflow_stage(db, workflow_run_id, to.as_str()).await?;
    let (status, error_message, mark_ended) = match to {
        WorkflowStage::Done => ("completed", None, true),
        WorkflowStage::Failed => ("failed", Some(notes.as_str()), true),
        WorkflowStage::Cancelled => ("cancelled", Some(notes.as_str()), true),
        _ => ("running", None, false),
    };
    workflow_repo::update_workflow_lifecycle(
        db,
        workflow_run_id,
        status,
        error_message,
        mark_ended,
    )
    .await?;

    let transition_id = uuid::Uuid::new_v4().to_string();
    workflow_repo::record_stage_transition(
        db,
        &transition_id,
        workflow_run_id,
        from.as_str(),
        to.as_str(),
        trigger.as_str(),
        &notes,
    )
    .await?;

    Ok(())
}

async fn annotate_transition_notes(
    db: &SqlitePool,
    workflow_run_id: &str,
    to_stage: &WorkflowStage,
    notes: String,
) -> Result<String, AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let team_name = match workflow_run.assigned_team_id.as_deref() {
        Some(team_id) => agent_repo::get_agent_team(db, team_id)
            .await
            .ok()
            .map(|team| team.name),
        None => None,
    };
    let coordinator_name = match workflow_run.coordinator_agent_id.as_deref() {
        Some(agent_id) => agent_repo::get_agent_definition(db, agent_id)
            .await
            .ok()
            .map(|agent| agent.name),
        None => None,
    };

    let mut annotated = notes;
    if let Some(team_name) = team_name {
        annotated.push_str(&format!(" | Team: {team_name}"));
    }
    if let Some(coordinator_name) = coordinator_name {
        let label = if to_stage.is_approval_gate() {
            "Approval owner"
        } else {
            "Coordinator"
        };
        annotated.push_str(&format!(" | {label}: {coordinator_name}"));
    }

    Ok(annotated)
}

use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::persistence::{agent_repo, work_item_repo, workflow_repo};
use sqlx::SqlitePool;

fn is_coordinator_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "manager" | "team_lead" | "coordinator"
    )
}

pub(crate) async fn requires_coordinator_review(
    db: &SqlitePool,
    stage: &WorkflowStage,
) -> Result<bool, AppError> {
    if let Some(policy) = agent_repo::get_workflow_stage_policy(db, stage.as_str()).await? {
        return Ok(policy.coordinator_required);
    }
    Ok(stage.requires_coordinator_review())
}

pub(crate) async fn has_active_coordinator_for_workflow(
    db: &SqlitePool,
    workflow_run_id: &str,
) -> Result<bool, AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;

    if let Some(coordinator_id) = workflow_run.coordinator_agent_id.as_deref() {
        if let Ok(agent) = agent_repo::get_agent_definition(db, coordinator_id).await {
            if agent.enabled
                && agent.employment_status == "active"
                && is_coordinator_role(&agent.role)
            {
                return Ok(true);
            }
        }
    }

    if let Some(team_id) = workflow_run.assigned_team_id.as_deref() {
        return Ok(agent_repo::find_team_coordinator(db, team_id)
            .await?
            .is_some());
    }

    let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;
    if let Some(team) = agent_repo::resolve_team_for_work_item(db, &work_item).await? {
        return Ok(agent_repo::find_team_coordinator(db, &team.id)
            .await?
            .is_some());
    }

    Ok(false)
}

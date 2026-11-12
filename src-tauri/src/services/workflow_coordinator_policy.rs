use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::persistence::{agent_repo, work_item_repo, workflow_repo};
use sqlx::SqlitePool;

pub(crate) fn is_coordinator_role(role: &str) -> bool {
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

// Keep a previously chosen coordinator only while it remains active and, if a team is
// assigned, still belongs to that team. Otherwise select the team's current coordinator.
pub(crate) async fn resolve_active_coordinator(
    db: &SqlitePool,
    preferred_id: Option<&str>,
    team_id: Option<&str>,
) -> Result<Option<crate::domain::agent::AgentDefinition>, AppError> {
    if let Some(team_id) = team_id {
        if let Some(preferred_id) = preferred_id {
            let members = agent_repo::list_agents_for_team(db, team_id).await?;
            if let Some(agent) = members.into_iter().find(|agent| agent.id == preferred_id && agent.enabled && agent.employment_status == "active" && is_coordinator_role(&agent.role)) {
                return Ok(Some(agent));
            }
        }
        return agent_repo::find_team_coordinator(db, team_id).await;
    }
    if let Some(preferred_id) = preferred_id {
        match agent_repo::get_agent_definition(db, preferred_id).await {
            Ok(agent) if agent.enabled && agent.employment_status == "active" && is_coordinator_role(&agent.role) => return Ok(Some(agent)),
            Ok(_) | Err(AppError::NotFound(_)) => {},
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

pub(crate) async fn has_active_coordinator_for_workflow(
    db: &SqlitePool,
    workflow_run_id: &str,
) -> Result<bool, AppError> {
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let team_id = if let Some(id) = workflow_run.assigned_team_id.as_deref() {
        Some(id.to_string())
    } else {
        let work_item = work_item_repo::get_work_item(db, &workflow_run.work_item_id).await?;
        agent_repo::resolve_team_for_work_item(db, &work_item).await?.map(|team| team.id)
    };
    Ok(resolve_active_coordinator(db, workflow_run.coordinator_agent_id.as_deref(), team_id.as_deref()).await?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coordinator_must_be_active_and_belong_to_assigned_team() {
        let dir = std::env::temp_dir().join(format!("aruvi-coordinator-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let pool = crate::persistence::db::create_pool(&format!("sqlite://{}", dir.join("test.db").display())).await.unwrap();
        for sql in [
            "INSERT INTO agent_teams (id,name) VALUES ('team-a','Team A'),('team-b','Team B')",
            "INSERT INTO agent_definitions (id,name,role) VALUES ('first','First','manager'),('second','Second','coordinator'),('outsider','Outsider','manager')",
            "INSERT INTO agent_team_memberships (id,team_id,agent_id,is_lead) VALUES ('first-link','team-a','first',1),('second-link','team-a','second',0),('outside-link','team-b','outsider',1)",
        ] { sqlx::query(sql).execute(&pool).await.unwrap(); }
        assert_eq!(resolve_active_coordinator(&pool, Some("first"), Some("team-a")).await.unwrap().unwrap().id, "first");
        sqlx::query("UPDATE agent_definitions SET enabled=0 WHERE id='first'").execute(&pool).await.unwrap();
        assert_eq!(resolve_active_coordinator(&pool, Some("first"), Some("team-a")).await.unwrap().unwrap().id, "second");
        assert_eq!(resolve_active_coordinator(&pool, Some("outsider"), Some("team-a")).await.unwrap().unwrap().id, "second");
        assert_eq!(resolve_active_coordinator(&pool, Some("outsider"), None).await.unwrap().unwrap().id, "outsider");
        sqlx::query("UPDATE agent_definitions SET enabled=1 WHERE id='first'").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM agent_team_memberships WHERE id='first-link'").execute(&pool).await.unwrap();
        assert_eq!(resolve_active_coordinator(&pool, Some("first"), Some("team-a")).await.unwrap().unwrap().id, "second");
        sqlx::query("UPDATE agent_definitions SET employment_status='inactive' WHERE id='second'").execute(&pool).await.unwrap();
        assert!(resolve_active_coordinator(&pool, Some("first"), Some("team-a")).await.unwrap().is_none());
        assert!(resolve_active_coordinator(&pool, Some("second"), None).await.unwrap().is_none());
        pool.close().await;
        std::fs::remove_dir_all(dir).unwrap();
    }
}

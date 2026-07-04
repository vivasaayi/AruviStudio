use super::{AgentService, TeamExecutionContext};
use crate::domain::agent::{AgentDefinition, AgentTeam};
use crate::domain::work_item::WorkItem;
use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::persistence::agent_repo;
use tracing::{debug, error};

impl AgentService {
    pub(super) async fn resolve_execution_context(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
    ) -> Result<TeamExecutionContext, AppError> {
        let team = if let Some(team_id) = workflow_run.assigned_team_id.as_deref() {
            agent_repo::get_agent_team(&self.db, team_id).await.ok()
        } else {
            None
        }
        .or(agent_repo::resolve_team_for_work_item(&self.db, work_item).await?);

        let coordinator =
            if let Some(coordinator_id) = workflow_run.coordinator_agent_id.as_deref() {
                agent_repo::get_agent_definition(&self.db, coordinator_id)
                    .await
                    .ok()
            } else {
                None
            }
            .or(match &team {
                Some(team_entry) => {
                    agent_repo::find_team_coordinator(&self.db, &team_entry.id).await?
                }
                None => None,
            });

        if stage_name == "coordinator_review" {
            let stage_agent = coordinator.clone().ok_or_else(|| {
                AppError::NotFound("No active coordinator found for coordinator review".to_string())
            })?;
            if !is_coordinator_role(&stage_agent.role) {
                return Err(AppError::Validation(format!(
                    "Invalid coordinator role for coordinator review: {}",
                    stage_agent.role
                )));
            }
            let expected_role = stage_agent.role.clone();
            return Ok(TeamExecutionContext {
                team,
                coordinator,
                stage_agent,
                expected_role,
            });
        }

        let (stage_agent, expected_role) =
            self.find_agent_for_stage(stage_name, team.as_ref()).await?;

        Ok(TeamExecutionContext {
            team,
            coordinator,
            stage_agent,
            expected_role,
        })
    }

    /// Find the appropriate agent for a stage.
    async fn find_agent_for_stage(
        &self,
        stage_name: &str,
        team: Option<&AgentTeam>,
    ) -> Result<(AgentDefinition, String), AppError> {
        debug!(stage_name = %stage_name, team_id = ?team.as_ref().map(|entry| entry.id.as_str()), "Finding agent for stage");
        let expected_roles = self.stage_role_candidates(stage_name).await;

        if let Some(team) = team {
            let team_agents = agent_repo::list_agents_for_team(&self.db, &team.id).await?;
            if let Some(agent) = select_agent_for_roles(team_agents, &expected_roles) {
                let matched_role = agent.role.clone();
                debug!(stage_name = %stage_name, team_id = %team.id, agent_id = %agent.id, matched_role = %matched_role, "Found team agent for stage");
                return Ok((agent, matched_role));
            }
        }

        let all_agents = agent_repo::list_agent_definitions(&self.db).await?;
        if let Some(agent) = select_agent_for_roles(all_agents, &expected_roles) {
            let matched_role = agent.role.clone();
            debug!(stage_name = %stage_name, agent_id = %agent.id, matched_role = %matched_role, "Found fallback agent for stage");
            return Ok((agent, matched_role));
        }

        error!(stage_name = %stage_name, expected_roles = ?expected_roles, "No enabled agent found for stage");
        Err(AppError::NotFound(format!(
            "No enabled agent found for stage {} (expected roles: {})",
            stage_name,
            expected_roles.join(", ")
        )))
    }

    async fn stage_role_candidates(&self, stage_name: &str) -> Vec<String> {
        if let Ok(Some(policy)) = agent_repo::get_workflow_stage_policy(&self.db, stage_name).await
        {
            let mut roles = policy.primary_roles;
            roles.extend(policy.fallback_roles);
            if !roles.is_empty() {
                return roles;
            }
        }

        match stage_name {
            "coordinator_review" => vec!["manager", "team_lead"],
            "requirement_analysis" => {
                vec!["manager", "architect", "analyst", "requirement_analysis"]
            }
            "planning" => vec!["architect", "manager", "planning"],
            "coding" => vec!["developer", "coding"],
            "unit_test_generation" => vec!["unit_tester", "unit_test_generation"],
            "integration_test_generation" => {
                vec!["integration_tester", "integration_test_generation"]
            }
            "ui_test_planning" => vec!["ui_tester", "ui_test_planning"],
            "qa_validation" => vec!["code_reviewer", "qa", "qa_validation"],
            "security_review" => vec!["security_analyzer", "security_review"],
            "performance_review" => vec!["performance_optimizer", "performance_review"],
            "push_preparation" => vec!["manager", "devops", "planning"],
            "git_push" => vec!["devops", "sre"],
            other => vec![other],
        }
        .into_iter()
        .map(str::to_string)
        .collect()
    }
}

fn select_agent_for_roles(
    agents: Vec<AgentDefinition>,
    expected_roles: &[String],
) -> Option<AgentDefinition> {
    agents
        .into_iter()
        .filter(|agent| agent.enabled && agent.employment_status == "active")
        .min_by_key(|agent| {
            expected_roles
                .iter()
                .position(|expected_role| expected_role.eq_ignore_ascii_case(&agent.role))
                .unwrap_or(usize::MAX)
        })
        .filter(|agent| {
            expected_roles
                .iter()
                .any(|expected_role| expected_role.eq_ignore_ascii_case(&agent.role))
        })
}

fn is_coordinator_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "manager" | "team_lead" | "coordinator"
    )
}

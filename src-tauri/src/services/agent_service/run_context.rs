use super::{AgentService, TeamExecutionContext};
use crate::domain::work_item::WorkItem;
use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::persistence::{agent_repo, artifact_repo};
use crate::services::agent_execution_limits::AgentExecutionBoundaries;
use crate::services::agent_repository_context;
use std::collections::HashMap;
use tracing::{debug, info};

impl AgentService {
    /// Build context for the agent.
    pub(super) async fn build_agent_context(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
        execution_context: &TeamExecutionContext,
        boundaries: &AgentExecutionBoundaries,
        context_budget_chars: usize,
    ) -> Result<HashMap<String, String>, AppError> {
        debug!(work_item_id = %work_item.id, workflow_run_id = %workflow_run.id, stage_name = %stage_name, "Building agent context");

        let mut context = HashMap::new();

        context.insert("work_item_id".to_string(), work_item.id.clone());
        context.insert("work_item_title".to_string(), work_item.title.clone());
        context.insert(
            "work_item_description".to_string(),
            work_item.description.clone(),
        );
        context.insert(
            "work_item_problem_statement".to_string(),
            work_item.problem_statement.clone(),
        );
        context.insert(
            "work_item_acceptance_criteria".to_string(),
            work_item.acceptance_criteria.clone(),
        );
        context.insert(
            "work_item_constraints".to_string(),
            work_item.constraints.clone(),
        );
        context.insert(
            "expected_stage_role".to_string(),
            execution_context.expected_role.clone(),
        );

        if let Some(team) = &execution_context.team {
            context.insert("assigned_team".to_string(), team.name.clone());
            context.insert("team_department".to_string(), team.department.clone());
            context.insert("team_description".to_string(), team.description.clone());
        }
        if let Some(coordinator) = &execution_context.coordinator {
            context.insert("team_coordinator".to_string(), coordinator.name.clone());
            context.insert(
                "team_coordinator_role".to_string(),
                coordinator.role.clone(),
            );
        }

        let artifacts = artifact_repo::list_work_item_artifacts(&self.db, &work_item.id).await?;
        debug!(work_item_id = %work_item.id, artifact_count = artifacts.len(), "Retrieved artifacts for work item");

        for artifact in &artifacts {
            if artifact.artifact_type.ends_with("_output") {
                context.insert(artifact.artifact_type.clone(), artifact.summary.clone());
            }
        }

        match stage_name {
            "requirement_analysis" => {
                if let Some(product_id) = &work_item.product_id {
                    context.insert("product_id".to_string(), product_id.clone());
                }
                if let Some(product_area_id) = &work_item.product_area_id {
                    context.insert("product_area_id".to_string(), product_area_id.clone());
                }
                if let Some(capability_id) = &work_item.capability_id {
                    context.insert("capability_id".to_string(), capability_id.clone());
                }
            }
            "planning" => {
                if let Some(req_analysis) = artifacts.iter().find(|a| {
                    a.artifact_type == "requirement_analysis_output"
                        || a.artifact_type == "requirement_analysis"
                }) {
                    context.insert(
                        "requirement_analysis".to_string(),
                        req_analysis.summary.clone(),
                    );
                }
            }
            "coding" => {
                if let Some(plan) = artifacts.iter().find(|a| {
                    a.artifact_type == "planning_output"
                        || a.artifact_type == "plan"
                        || a.artifact_type == "planning"
                }) {
                    context.insert("implementation_plan".to_string(), plan.summary.clone());
                }
                if let Some(repo_id) = &work_item.active_repo_id {
                    context.insert("repository_id".to_string(), repo_id.clone());
                    if let Ok(repo) =
                        crate::persistence::repository_repo::get_repository(&self.db, repo_id).await
                    {
                        let repo_context = agent_repository_context::build_repository_context(
                            &repo.local_path,
                            boundaries,
                            context_budget_chars,
                        )?;
                        if !repo_context.is_empty() {
                            context.insert("repository_context".to_string(), repo_context);
                        }
                    }
                }
            }
            _ => {}
        }

        let skills = agent_repo::list_skills(&self.db).await?;
        let skill_name_by_id = skills
            .into_iter()
            .map(|skill| (skill.id, skill.name))
            .collect::<HashMap<_, _>>();

        let agent_skill_links = agent_repo::list_agent_skill_links(&self.db).await?;
        let agent_skills = agent_skill_links
            .into_iter()
            .filter(|link| link.agent_id == execution_context.stage_agent.id)
            .filter_map(|link| {
                skill_name_by_id
                    .get(&link.skill_id)
                    .map(|name| format!("{name} ({})", link.proficiency))
            })
            .collect::<Vec<_>>();
        if !agent_skills.is_empty() {
            context.insert("agent_skills".to_string(), agent_skills.join(", "));
        }

        if let Some(team) = &execution_context.team {
            let team_skill_links = agent_repo::list_team_skill_links(&self.db).await?;
            let team_skills = team_skill_links
                .into_iter()
                .filter(|link| link.team_id == team.id)
                .filter_map(|link| skill_name_by_id.get(&link.skill_id).cloned())
                .collect::<Vec<_>>();
            if !team_skills.is_empty() {
                context.insert("team_skills".to_string(), team_skills.join(", "));
            }
        }

        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run.id, stage_name = %stage_name, context_keys = context.len(), "Successfully built agent context");
        Ok(context)
    }

    pub(super) async fn store_manager_handoff(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
        execution_context: &TeamExecutionContext,
    ) -> Result<(), AppError> {
        let Some(coordinator) = &execution_context.coordinator else {
            return Ok(());
        };

        let handoff_dir = self
            .artifact_base_path
            .join(&workflow_run.id)
            .join("handoffs");
        tokio::fs::create_dir_all(&handoff_dir).await?;
        let handoff_path = handoff_dir.join(format!("{stage_name}.txt"));
        let team_name = execution_context
            .team
            .as_ref()
            .map(|team| team.name.clone())
            .unwrap_or_else(|| "unassigned".to_string());
        let handoff_body = format!(
            "Coordinator: {} ({})\nTeam: {}\nStage: {}\nAssigned Agent: {} ({})\nRule: Coordinator performs the handoff, the specialist executes, and artifacts should be left ready for the next gate.\n",
            coordinator.name,
            coordinator.role,
            team_name,
            stage_name,
            execution_context.stage_agent.name,
            execution_context.stage_agent.role
        );
        tokio::fs::write(&handoff_path, &handoff_body).await?;

        let artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            artifact_repo::CreateArtifactInput {
                id: &artifact_id,
                work_item_id: &work_item.id,
                workflow_run_id: Some(&workflow_run.id),
                agent_run_id: None,
                artifact_type: &format!("manager_handoff_{stage_name}"),
                summary: &format!(
                    "Coordinator {} handed {} to {} for {}",
                    coordinator.name,
                    work_item.title,
                    execution_context.stage_agent.name,
                    stage_name
                ),
                storage_path: &handoff_path.to_string_lossy(),
            },
        )
        .await?;

        Ok(())
    }
}

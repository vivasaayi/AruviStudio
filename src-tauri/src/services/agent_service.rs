use crate::domain::agent::{AgentDefinition, AgentRun, AgentRunStatus, AgentTeam};
use crate::domain::model::ModelDefinition;
use crate::domain::work_item::WorkItem;
use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::persistence::{
    agent_repo, artifact_repo, model_call_repo, model_repo, work_item_repo, workflow_repo,
};
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use crate::services::agent_model_selection;
use crate::services::agent_prompt;
use crate::services::agent_repository_context;
use crate::services::model_service;
use sqlx::SqlitePool;
use std::collections::HashMap;
#[cfg(test)]
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid;

pub struct AgentService {
    db: Arc<SqlitePool>,
    model_service: Arc<model_service::ModelService>,
    artifact_base_path: PathBuf,
    workspace_base_path: PathBuf,
}

pub(crate) struct TeamExecutionContext {
    pub(crate) team: Option<AgentTeam>,
    pub(crate) coordinator: Option<AgentDefinition>,
    pub(crate) stage_agent: AgentDefinition,
    pub(crate) expected_role: String,
}

struct AgentModelOutput {
    content: String,
}

#[cfg(test)]
static TEST_MODEL_OUTPUTS: OnceLock<StdMutex<HashMap<String, VecDeque<String>>>> = OnceLock::new();
#[cfg(test)]
const TEST_ANY_WORKFLOW_KEY: &str = "__any_workflow__";

mod tool_loop;

impl AgentService {
    #[cfg(test)]
    pub(crate) fn set_test_model_outputs_for_workflow(workflow_run_id: &str, outputs: Vec<String>) {
        let map = TEST_MODEL_OUTPUTS.get_or_init(|| StdMutex::new(HashMap::new()));
        let mut guard = map.lock().expect("failed to lock test model output queue");
        if outputs.is_empty() {
            guard.remove(workflow_run_id);
        } else {
            guard.insert(workflow_run_id.to_string(), VecDeque::from(outputs));
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_model_outputs_for_any_workflow(outputs: Vec<String>) {
        Self::set_test_model_outputs_for_workflow(TEST_ANY_WORKFLOW_KEY, outputs);
    }

    pub fn new(
        db: Arc<SqlitePool>,
        model_service: Arc<model_service::ModelService>,
        artifact_base_path: PathBuf,
        workspace_base_path: PathBuf,
    ) -> Self {
        Self {
            db,
            model_service,
            artifact_base_path,
            workspace_base_path,
        }
    }

    fn is_coordinator_role(role: &str) -> bool {
        matches!(
            role.to_ascii_lowercase().as_str(),
            "manager" | "team_lead" | "coordinator"
        )
    }

    /// Run an agent for a specific workflow stage
    pub async fn run_agent_for_stage(
        &self,
        workflow_run_id: &str,
        stage_name: &str,
    ) -> Result<AgentRun, AppError> {
        info!(
            "Running agent for stage {} in workflow {}",
            stage_name, workflow_run_id
        );

        // Get workflow and work item context
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;

        // Resolve team ownership first, then assign the stage to the best agent inside that team.
        let execution_context = self
            .resolve_execution_context(&work_item, &workflow_run, stage_name)
            .await?;
        workflow_repo::set_workflow_assignment(
            &self.db,
            workflow_run_id,
            execution_context.team.as_ref().map(|team| team.id.as_str()),
            execution_context
                .coordinator
                .as_ref()
                .map(|agent| agent.id.as_str()),
        )
        .await?;
        let model_def = agent_model_selection::find_model_for_agent(
            &self.db,
            &execution_context.stage_agent.id,
        )
        .await?;
        let boundaries =
            agent_execution_limits::parse_boundaries(&execution_context.stage_agent.boundaries);
        let context_budget_chars =
            agent_execution_limits::resolve_context_char_budget(&model_def, &boundaries);
        let response_token_budget =
            agent_execution_limits::resolve_response_token_budget(&model_def, &boundaries);

        // Create agent run record
        let agent_run_id = uuid::Uuid::new_v4().to_string();
        let agent_run = agent_repo::create_agent_run(
            &self.db,
            &agent_run_id,
            workflow_run_id,
            &workflow_run.work_item_id,
            &execution_context.stage_agent.id,
            &model_def.id,
            stage_name,
        )
        .await?;

        self.store_manager_handoff(&work_item, &workflow_run, stage_name, &execution_context)
            .await?;

        // Build context and prompt
        let context = self
            .build_agent_context(
                &work_item,
                &workflow_run,
                stage_name,
                &execution_context,
                &boundaries,
                context_budget_chars,
            )
            .await?;
        let prompt = agent_prompt::build_agent_prompt(
            &execution_context.stage_agent,
            &context,
            stage_name,
            &execution_context,
            &boundaries,
        )?;

        // Execute the agent
        let execution_result = if stage_name == "coding" {
            self.execute_coding_with_tools(
                &agent_run,
                &model_def,
                &prompt,
                &execution_context.stage_agent,
                &boundaries,
                response_token_budget,
            )
            .await
        } else {
            match self
                .execute_agent_run(&agent_run, &model_def, &prompt, response_token_budget)
                .await
            {
                Ok(output) => {
                    self.process_agent_output(&agent_run, &output.content, stage_name)
                        .await
                }
                Err(error) => Err(error),
            }
        };

        match execution_result {
            Ok(processed_output) => {
                // Store the output as an artifact
                self.store_agent_output(&agent_run, &prompt, &processed_output)
                    .await?;

                // Update agent run as completed
                agent_repo::update_agent_run_status(
                    &self.db,
                    &agent_run_id,
                    AgentRunStatus::Completed,
                )
                .await?;

                let mut completed_run = agent_run.clone();
                completed_run.status = AgentRunStatus::Completed;
                Ok(completed_run)
            }
            Err(e) => {
                error!("Agent execution failed: {}", e);
                let error_message = e.to_string();
                let failure_output = format!("Agent stage failed:\n{error_message}");
                if let Err(store_error) = self
                    .store_agent_output(&agent_run, &prompt, &failure_output)
                    .await
                {
                    warn!(
                        agent_run_id = %agent_run_id,
                        stage_name = %stage_name,
                        error = %store_error,
                        "Failed to persist prompt/output artifacts for failed agent run"
                    );
                }
                agent_repo::update_agent_run_failure(&self.db, &agent_run_id, &error_message)
                    .await?;
                Err(e)
            }
        }
    }

    /// Find the appropriate agent for a stage
    async fn find_agent_for_stage(
        &self,
        stage_name: &str,
        team: Option<&AgentTeam>,
    ) -> Result<(AgentDefinition, String), AppError> {
        debug!(stage_name = %stage_name, team_id = ?team.as_ref().map(|entry| entry.id.as_str()), "Finding agent for stage");
        let expected_roles = self.stage_role_candidates(stage_name).await;

        if let Some(team) = team {
            let team_agents = agent_repo::list_agents_for_team(&self.db, &team.id).await?;
            if let Some(agent) = self.select_agent_for_roles(team_agents, &expected_roles) {
                let matched_role = agent.role.clone();
                debug!(stage_name = %stage_name, team_id = %team.id, agent_id = %agent.id, matched_role = %matched_role, "Found team agent for stage");
                return Ok((agent, matched_role));
            }
        }

        let all_agents = agent_repo::list_agent_definitions(&self.db).await?;
        if let Some(agent) = self.select_agent_for_roles(all_agents, &expected_roles) {
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

    /// Build context for the agent
    async fn build_agent_context(
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

        // Basic work item information
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

        // Get related artifacts from previous stages
        let artifacts = artifact_repo::list_work_item_artifacts(&self.db, &work_item.id).await?;
        debug!(work_item_id = %work_item.id, artifact_count = artifacts.len(), "Retrieved artifacts for work item");

        for artifact in &artifacts {
            if artifact.artifact_type.ends_with("_output") {
                context.insert(artifact.artifact_type.clone(), artifact.summary.clone());
            }
        }

        // Stage-specific context
        match stage_name {
            "requirement_analysis" => {
                // Include product/product_area/capability hierarchy if available
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
                // Include requirement analysis results
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
                // Include planning results and repository info
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

    async fn resolve_execution_context(
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
            if !Self::is_coordinator_role(&stage_agent.role) {
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

    fn select_agent_for_roles(
        &self,
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

    async fn store_manager_handoff(
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

    /// Execute the agent run against the model
    async fn execute_agent_run(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        prompt: &str,
        max_tokens: i64,
    ) -> Result<AgentModelOutput, AppError> {
        #[cfg(test)]
        if let Some(queue_map) = TEST_MODEL_OUTPUTS.get() {
            let mut guard = queue_map
                .lock()
                .expect("failed to lock test model output queue");
            let mut next: Option<String> = None;
            let mut remove_keys: Vec<String> = Vec::new();

            for key in [agent_run.workflow_run_id.as_str(), TEST_ANY_WORKFLOW_KEY] {
                if next.is_some() {
                    break;
                }
                if let Some(queue) = guard.get_mut(key) {
                    next = queue.pop_front();
                    if queue.is_empty() {
                        remove_keys.push(key.to_string());
                    }
                }
            }

            for key in remove_keys {
                guard.remove(&key);
            }

            if let Some(next) = next {
                debug!(
                    agent_run_id = %agent_run.id,
                    model_id = %model_def.id,
                    model_name = %model_def.name,
                    "Using queued test model response"
                );
                return Ok(AgentModelOutput { content: next });
            }
        }

        debug!(agent_run_id = %agent_run.id, model_id = %model_def.id, model_name = %model_def.name, prompt_length = prompt.len(), max_tokens = max_tokens, "Executing agent run");

        // Get the model provider
        let provider = model_repo::get_provider(&self.db, &model_def.provider_id).await?;
        debug!(agent_run_id = %agent_run.id, provider_id = %provider.id, provider_name = %provider.name, "Retrieved model provider");

        // Create the model gateway
        let gateway = self.model_service.create_gateway(&provider)?;
        debug!(agent_run_id = %agent_run.id, "Created model gateway");

        // Prepare the completion request
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        let request = CompletionRequest {
            model: model_def.name.clone(),
            messages: messages.clone(),
            temperature: Some(0.7),
            max_tokens: Some(max_tokens),
        };

        // Execute the completion
        let source_kind = "workflow_agent";
        let source_label = format!("{} / {}", agent_run.stage, agent_run.agent_id);
        let call_index =
            model_call_repo::next_model_call_index(&self.db, source_kind, Some(&agent_run.id))
                .await?;
        let prompt_chars = agent_execution_limits::char_count_i64(prompt);
        let call_started = Instant::now();
        let response = match gateway.run_completion(request).await {
            Ok(response) => response,
            Err(err) => {
                let duration_ms = agent_execution_limits::elapsed_ms(call_started);
                let error_message = err.to_string();
                let call_id = uuid::Uuid::new_v4().to_string();
                let record_result = async {
                    let request_messages_json = serde_json::to_string_pretty(&messages)?;
                    let snapshots = model_call_repo::write_model_call_snapshots(
                        &self.artifact_base_path,
                        &call_id,
                        Some(&request_messages_json),
                        None,
                    )
                    .await?;
                    model_call_repo::create_model_call(
                        &self.db,
                        model_call_repo::CreateModelCallParams {
                            id: &call_id,
                            source_kind,
                            source_id: Some(&agent_run.id),
                            source_label: &source_label,
                            workflow_run_id: Some(&agent_run.workflow_run_id),
                            agent_run_id: Some(&agent_run.id),
                            work_item_id: None,
                            product_id: None,
                            session_id: None,
                            agent_id: Some(&agent_run.agent_id),
                            stage: Some(&agent_run.stage),
                            provider_id: &provider.id,
                            provider_name: &provider.name,
                            provider_type: provider.provider_type.as_str(),
                            provider_base_url: &provider.base_url,
                            model_id: Some(&model_def.id),
                            model_name: &model_def.name,
                            call_index,
                            request_message_count: 1,
                            prompt_chars,
                            response_chars: 0,
                            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                            max_tokens: Some(max_tokens),
                            temperature: Some(0.7),
                            token_count_input: None,
                            token_count_output: None,
                            duration_ms: Some(duration_ms),
                            status: "failed",
                            error_message: Some(&error_message),
                        },
                    )
                    .await
                }
                .await;
                if let Err(record_err) = record_result {
                    warn!(
                        agent_run_id = %agent_run.id,
                        record_error = %record_err,
                        model_error = %error_message,
                        "Failed to record failed model call telemetry"
                    );
                }
                return Err(err);
            }
        };
        let duration_ms = agent_execution_limits::elapsed_ms(call_started);
        let response_chars = agent_execution_limits::char_count_i64(&response.content);
        let call_id = uuid::Uuid::new_v4().to_string();
        let request_messages_json = serde_json::to_string_pretty(&messages)?;
        let snapshots = model_call_repo::write_model_call_snapshots(
            &self.artifact_base_path,
            &call_id,
            Some(&request_messages_json),
            Some(&response.content),
        )
        .await?;
        model_call_repo::create_model_call(
            &self.db,
            model_call_repo::CreateModelCallParams {
                id: &call_id,
                source_kind,
                source_id: Some(&agent_run.id),
                source_label: &source_label,
                workflow_run_id: Some(&agent_run.workflow_run_id),
                agent_run_id: Some(&agent_run.id),
                work_item_id: None,
                product_id: None,
                session_id: None,
                agent_id: Some(&agent_run.agent_id),
                stage: Some(&agent_run.stage),
                provider_id: &provider.id,
                provider_name: &provider.name,
                provider_type: provider.provider_type.as_str(),
                provider_base_url: &provider.base_url,
                model_id: Some(&model_def.id),
                model_name: &model_def.name,
                call_index,
                request_message_count: 1,
                prompt_chars,
                response_chars,
                request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                max_tokens: Some(max_tokens),
                temperature: Some(0.7),
                token_count_input: response.token_count_input,
                token_count_output: response.token_count_output,
                duration_ms: Some(duration_ms),
                status: "completed",
                error_message: None,
            },
        )
        .await?;
        agent_repo::add_agent_run_token_usage(
            &self.db,
            &agent_run.id,
            response.token_count_input,
            response.token_count_output,
        )
        .await?;
        info!(
            agent_run_id = %agent_run.id,
            response_length = response.content.len(),
            token_count_input = ?response.token_count_input,
            token_count_output = ?response.token_count_output,
            "Successfully executed agent run"
        );

        Ok(AgentModelOutput {
            content: response.content,
        })
    }

    /// Store agent output as artifacts
    async fn store_agent_output(
        &self,
        agent_run: &AgentRun,
        prompt: &str,
        output: &str,
    ) -> Result<(), AppError> {
        debug!(agent_run_id = %agent_run.id, prompt_length = prompt.len(), output_length = output.len(), "Storing agent output as artifacts");

        // Get the workflow run to obtain work_item_id
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item_id = workflow_run.work_item_id;
        debug!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "Retrieved workflow run for work item");

        let artifact_dir = self.artifact_base_path.join(&agent_run.id);
        tokio::fs::create_dir_all(&artifact_dir)
            .await
            .map_err(|e| {
                AppError::Io(std::io::Error::other(format!(
                    "Failed to create artifact directory: {}",
                    e
                )))
            })?;

        // Write prompt to file first so storage path points to an existing file.
        let prompt_path = artifact_dir.join("prompt.txt");
        tokio::fs::write(&prompt_path, prompt).await.map_err(|e| {
            AppError::Io(std::io::Error::other(format!(
                "Failed to write prompt file: {}",
                e
            )))
        })?;
        debug!(agent_run_id = %agent_run.id, prompt_path = %prompt_path.display(), "Wrote prompt to file");

        // Store prompt snapshot
        let prompt_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            artifact_repo::CreateArtifactInput {
                id: &prompt_artifact_id,
                work_item_id: &work_item_id,
                workflow_run_id: Some(&agent_run.workflow_run_id),
                agent_run_id: Some(&agent_run.id),
                artifact_type: &format!("{}_prompt", agent_run.stage),
                summary: "Prompt used for agent execution",
                storage_path: &prompt_path.to_string_lossy(),
            },
        )
        .await?;
        debug!(agent_run_id = %agent_run.id, artifact_id = %prompt_artifact_id, "Created prompt artifact");

        // Write output file before recording artifact metadata.
        let output_path = artifact_dir.join("output.txt");
        tokio::fs::write(&output_path, output).await.map_err(|e| {
            AppError::Io(std::io::Error::other(format!(
                "Failed to write output file: {}",
                e
            )))
        })?;
        debug!(agent_run_id = %agent_run.id, output_path = %output_path.display(), "Wrote output to file");

        // Store output artifact
        let output_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            artifact_repo::CreateArtifactInput {
                id: &output_artifact_id,
                work_item_id: &work_item_id,
                workflow_run_id: Some(&agent_run.workflow_run_id),
                agent_run_id: Some(&agent_run.id),
                artifact_type: &format!("{}_output", agent_run.stage),
                summary: &output.chars().take(200).collect::<String>(),
                storage_path: &output_path.to_string_lossy(),
            },
        )
        .await?;
        debug!(agent_run_id = %agent_run.id, artifact_id = %output_artifact_id, "Created output artifact");

        agent_repo::update_agent_run_snapshot_paths(
            &self.db,
            &agent_run.id,
            &prompt_path.to_string_lossy(),
            &output_path.to_string_lossy(),
        )
        .await?;

        info!(agent_run_id = %agent_run.id, "Successfully stored agent output as artifacts");
        Ok(())
    }

    /// Process agent output based on the agent type
    async fn process_agent_output(
        &self,
        agent_run: &AgentRun,
        output: &str,
        stage_name: &str,
    ) -> Result<String, AppError> {
        debug!(agent_run_id = %agent_run.id, stage_name = %stage_name, output_length = output.len(), "Processing agent output");

        match stage_name {
            "coding" => {
                // For coding agent, we need to apply the code changes
                self.apply_code_changes(agent_run, output).await?;
                info!(agent_run_id = %agent_run.id, stage_name = %stage_name, "Applied code changes from coding agent");
                Ok(output.to_string())
            }
            _ => {
                // For other agents, just return the output as-is
                debug!(agent_run_id = %agent_run.id, stage_name = %stage_name, "Returning output as-is for non-coding agent");
                Ok(output.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests;

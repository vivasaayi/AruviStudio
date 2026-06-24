use crate::domain::agent::{AgentDefinition, AgentRun, AgentRunStatus, AgentTeam};
use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::persistence::{
    agent_repo, artifact_repo, model_call_repo, model_repo, work_item_repo, workflow_repo,
};
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::services::agent_execution_limits;
use crate::services::agent_model_selection;
use crate::services::agent_prompt;
use crate::services::model_service;
use sqlx::SqlitePool;
#[cfg(test)]
use std::collections::{HashMap, VecDeque};
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

mod execution_context;
mod run_context;
mod tool_loop;
mod tool_loop_contract;
mod tool_loop_workspace;

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

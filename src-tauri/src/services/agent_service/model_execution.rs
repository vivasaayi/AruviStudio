use crate::domain::agent::AgentRun;
use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::persistence::{agent_repo, model_call_repo, model_repo};
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::services::agent_execution_limits;
use crate::services::agent_service::{AgentModelOutput, AgentService};
use std::time::Instant;
use tracing::{debug, info, warn};

impl AgentService {
    /// Execute the agent run against the model.
    pub(super) async fn execute_agent_run(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        prompt: &str,
        max_tokens: i64,
    ) -> Result<AgentModelOutput, AppError> {
        #[cfg(test)]
        if let Some(queue_map) = super::TEST_MODEL_OUTPUTS.get() {
            let mut guard = queue_map
                .lock()
                .expect("failed to lock test model output queue");
            let mut next: Option<String> = None;
            let mut remove_keys: Vec<String> = Vec::new();

            for key in [
                agent_run.workflow_run_id.as_str(),
                super::TEST_ANY_WORKFLOW_KEY,
            ] {
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

        let provider = model_repo::get_provider(&self.db, &model_def.provider_id).await?;
        debug!(agent_run_id = %agent_run.id, provider_id = %provider.id, provider_name = %provider.name, "Retrieved model provider");

        let gateway = self.model_service.create_gateway(&provider)?;
        debug!(agent_run_id = %agent_run.id, "Created model gateway");

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
}

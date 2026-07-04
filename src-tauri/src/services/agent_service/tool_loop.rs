use super::tool_loop_contract::{
    build_coding_tool_prompt, parse_tool_loop_response, resolve_allowed_tools,
    write_tool_trace_snapshot, ToolLoopResponse, ToolLoopTraceEntry,
};
use super::AgentService;
use crate::domain::agent::{AgentDefinition, AgentRun};
use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::persistence::{artifact_repo, work_item_repo, workflow_repo};
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use std::collections::HashSet;
use tracing::{info, warn};

impl AgentService {
    pub(super) async fn execute_coding_with_tools(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        base_prompt: &str,
        stage_agent: &AgentDefinition,
        boundaries: &AgentExecutionBoundaries,
        response_token_budget: i64,
    ) -> Result<String, AppError> {
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;
        let Some(repo_id) = work_item.active_repo_id.as_deref() else {
            warn!(agent_run_id = %agent_run.id, work_item_id = %work_item.id, "No active repository found for coding tool loop; returning model output only");
            return self
                .execute_agent_run(agent_run, model_def, base_prompt, response_token_budget)
                .await
                .map(|output| output.content);
        };
        let repo = crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?;
        let workspace = WorkItemWorkspace::create(
            &work_item,
            &agent_run.workflow_run_id,
            &repo,
            &self.workspace_base_path,
        )
        .await?;

        let allowed_tools = resolve_allowed_tools(&stage_agent.allowed_tools);
        let max_steps = agent_execution_limits::max_tool_steps(boundaries);
        let mut tool_observations: Vec<String> = Vec::new();
        let mut trace: Vec<ToolLoopTraceEntry> = Vec::new();
        let mut changed_files: HashSet<String> = HashSet::new();
        let mut final_summary: Option<String> = None;
        let artifact_dir = self.artifact_base_path.join(&agent_run.id);
        tokio::fs::create_dir_all(&artifact_dir).await?;
        let trace_path = artifact_dir.join("tool_trace.json");
        write_tool_trace_snapshot(&trace_path, &trace).await?;
        let trace_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            artifact_repo::CreateArtifactInput {
                id: &trace_artifact_id,
                work_item_id: &work_item.id,
                workflow_run_id: Some(&agent_run.workflow_run_id),
                agent_run_id: Some(&agent_run.id),
                artifact_type: "coding_tool_trace",
                summary: "Tool loop trace for coding stage (live)",
                storage_path: &trace_path.to_string_lossy(),
            },
        )
        .await?;

        for step in 1..=max_steps {
            trace.push(ToolLoopTraceEntry {
                step,
                kind: "model_call_started".to_string(),
                payload: format!("step={} max_steps={}", step, max_steps),
            });
            write_tool_trace_snapshot(&trace_path, &trace).await?;
            let step_prompt = build_coding_tool_prompt(
                base_prompt,
                &allowed_tools,
                &tool_observations,
                step,
                max_steps,
            );
            let model_output = self
                .execute_agent_run(agent_run, model_def, &step_prompt, response_token_budget)
                .await?
                .content;
            trace.push(ToolLoopTraceEntry {
                step,
                kind: "model_output".to_string(),
                payload: agent_execution_limits::compact_trace_payload(&model_output),
            });
            write_tool_trace_snapshot(&trace_path, &trace).await?;

            match parse_tool_loop_response(&model_output) {
                Some(ToolLoopResponse::ToolCall {
                    tool,
                    arguments,
                    reason,
                }) => {
                    let execution = self
                        .execute_tool_call_in_workspace(
                            &workspace,
                            &tool,
                            &arguments,
                            &allowed_tools,
                            boundaries,
                            &mut changed_files,
                        )
                        .await;
                    match execution {
                        Ok(result) => {
                            let rendered = serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "{}".to_string());
                            tool_observations.push(
                                agent_execution_limits::compact_tool_observation(
                                    step,
                                    &tool,
                                    reason,
                                    "tool_result",
                                    &rendered,
                                ),
                            );
                            trace.push(ToolLoopTraceEntry {
                                step,
                                kind: "tool_result".to_string(),
                                payload: agent_execution_limits::compact_trace_payload(&rendered),
                            });
                            write_tool_trace_snapshot(&trace_path, &trace).await?;
                        }
                        Err(error) => {
                            let rendered = error.to_string();
                            tool_observations.push(format!(
                                "tool_error step={} tool={} reason={} error={}",
                                step,
                                tool,
                                reason.unwrap_or_default(),
                                rendered
                            ));
                            trace.push(ToolLoopTraceEntry {
                                step,
                                kind: "tool_error".to_string(),
                                payload: agent_execution_limits::compact_trace_payload(&rendered),
                            });
                            write_tool_trace_snapshot(&trace_path, &trace).await?;
                        }
                    }
                }
                Some(ToolLoopResponse::Final { summary, result }) => {
                    let resolved = summary
                        .or(result)
                        .unwrap_or_else(|| "Coding stage completed by tool loop.".to_string());
                    final_summary = Some(resolved);
                    trace.push(ToolLoopTraceEntry {
                        step,
                        kind: "final".to_string(),
                        payload: final_summary.clone().unwrap_or_default(),
                    });
                    write_tool_trace_snapshot(&trace_path, &trace).await?;
                    break;
                }
                None => {
                    if step == 1 {
                        // Backward-compatible fallback for legacy "File: ..." output.
                        let legacy_changed = self
                            .parse_and_apply_changes(&workspace, &model_output, boundaries)
                            .await?;
                        for path in legacy_changed {
                            changed_files.insert(path);
                        }
                        final_summary =
                            Some("Applied legacy coding response format (File blocks)".to_string());
                        trace.push(ToolLoopTraceEntry {
                            step,
                            kind: "legacy_fallback".to_string(),
                            payload: "Used legacy File-block parser".to_string(),
                        });
                        write_tool_trace_snapshot(&trace_path, &trace).await?;
                        break;
                    }
                    tool_observations.push(format!(
                        "tool_error step={} tool=parser error=Response did not match tool/final JSON contract",
                        step
                    ));
                }
            }
        }

        if final_summary.is_none() {
            if let Some(summary) = self
                .try_force_tool_loop_finalization(
                    agent_run,
                    model_def,
                    &tool_observations,
                    response_token_budget,
                )
                .await?
            {
                trace.push(ToolLoopTraceEntry {
                    step: max_steps + 1,
                    kind: "forced_final".to_string(),
                    payload: summary.clone(),
                });
                write_tool_trace_snapshot(&trace_path, &trace).await?;
                final_summary = Some(summary);
            }
        }

        let changed_files_list = changed_files.into_iter().collect::<Vec<_>>();
        write_tool_trace_snapshot(&trace_path, &trace).await?;

        if final_summary.is_none() {
            if !changed_files_list.is_empty() {
                warn!(
                    agent_run_id = %agent_run.id,
                    changed_files = changed_files_list.len(),
                    max_steps = max_steps,
                    "Coding tool loop reached max steps without final response; continuing with collected edits"
                );
                final_summary = Some(format!(
                    "Applied changes after reaching max steps ({max_steps}) without final response."
                ));
            } else {
                return Err(AppError::Validation(format!(
                    "Coding tool loop did not reach a final response within {} steps",
                    max_steps
                )));
            }
        }

        if !changed_files_list.is_empty() {
            workspace
                .sync_files_back(&repo.local_path, &changed_files_list)
                .await?;

            let applied_files_path = artifact_dir.join("applied_files.txt");
            tokio::fs::write(&applied_files_path, changed_files_list.join("\n")).await?;

            let artifact_id = uuid::Uuid::new_v4().to_string();
            artifact_repo::create_artifact(
                &self.db,
                artifact_repo::CreateArtifactInput {
                    id: &artifact_id,
                    work_item_id: &work_item.id,
                    workflow_run_id: Some(&agent_run.workflow_run_id),
                    agent_run_id: Some(&agent_run.id),
                    artifact_type: "coding_applied_files",
                    summary: &format!("Applied {} files to repository", changed_files_list.len()),
                    storage_path: &applied_files_path.to_string_lossy(),
                },
            )
            .await?;
        } else {
            warn!(agent_run_id = %agent_run.id, "Coding tool loop completed with no changed files");
        }

        if agent_execution_limits::should_keep_workspace(boundaries) {
            info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Retaining workspace for inspection");
        } else {
            workspace.cleanup().await?;
        }

        Ok(format!(
            "{}\nChanged files: {}",
            final_summary.unwrap_or_default(),
            if changed_files_list.is_empty() {
                "none".to_string()
            } else {
                changed_files_list.join(", ")
            }
        ))
    }
}

use super::AgentService;
use crate::domain::agent::{AgentDefinition, AgentRun};
use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::persistence::{agent_repo, artifact_repo, work_item_repo, workflow_repo};
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use crate::services::repo_service;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToolLoopResponse {
    ToolCall {
        tool: String,
        arguments: Value,
        reason: Option<String>,
    },
    Final {
        summary: Option<String>,
        result: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
struct ToolLoopTraceEntry {
    step: usize,
    kind: String,
    payload: String,
}

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

        let allowed_tools = self.resolve_allowed_tools(&stage_agent.allowed_tools);
        let max_steps = agent_execution_limits::max_tool_steps(boundaries);
        let mut tool_observations: Vec<String> = Vec::new();
        let mut trace: Vec<ToolLoopTraceEntry> = Vec::new();
        let mut changed_files: HashSet<String> = HashSet::new();
        let mut final_summary: Option<String> = None;
        let artifact_dir = self.artifact_base_path.join(&agent_run.id);
        tokio::fs::create_dir_all(&artifact_dir).await?;
        let trace_path = artifact_dir.join("tool_trace.json");
        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
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
            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
            let step_prompt = self.build_coding_tool_prompt(
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
            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;

            match Self::parse_tool_loop_response(&model_output) {
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
                            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
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
                            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
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
                    Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
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
                        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
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
                Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                final_summary = Some(summary);
            }
        }

        let changed_files_list = changed_files.into_iter().collect::<Vec<_>>();
        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;

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

    fn resolve_allowed_tools(&self, configured_tools: &[String]) -> HashSet<String> {
        let mut allowed = HashSet::new();
        if configured_tools.is_empty() {
            allowed.insert("repo.list_tree".to_string());
            allowed.insert("repo.read_file".to_string());
            allowed.insert("repo.search".to_string());
            allowed.insert("repo.write_file".to_string());
            allowed.insert("repo.replace_range".to_string());
            allowed.insert("repo.apply_patch".to_string());
            return allowed;
        }
        for tool in configured_tools {
            let lowered = tool.trim().to_ascii_lowercase();
            if lowered.contains("read") {
                allowed.insert("repo.read_file".to_string());
                allowed.insert("repo.list_tree".to_string());
                allowed.insert("repo.search".to_string());
            }
            if lowered.contains("write") || lowered.contains("create") {
                allowed.insert("repo.write_file".to_string());
                allowed.insert("repo.replace_range".to_string());
            }
            if lowered.contains("modify") || lowered.contains("patch") {
                allowed.insert("repo.apply_patch".to_string());
                allowed.insert("repo.replace_range".to_string());
            }
            if lowered.starts_with("repo.") {
                allowed.insert(lowered);
            }
        }
        if allowed.is_empty() {
            allowed.insert("repo.read_file".to_string());
            allowed.insert("repo.search".to_string());
            allowed.insert("repo.write_file".to_string());
            allowed.insert("repo.replace_range".to_string());
        }
        allowed
    }

    fn build_coding_tool_prompt(
        &self,
        base_prompt: &str,
        allowed_tools: &HashSet<String>,
        observations: &[String],
        step: usize,
        max_steps: usize,
    ) -> String {
        let mut prompt = String::new();
        if step == 1 {
            prompt.push_str(base_prompt);
        } else {
            prompt.push_str("Coding task recap:\n");
            prompt.push_str(&Self::condense_tool_loop_base_prompt(base_prompt));
        }
        prompt.push_str("\n\nYou are in a tool execution loop for coding.");
        prompt.push_str("\nReturn exactly one JSON object.");
        prompt.push_str("\nAllowed tools: ");
        prompt.push_str(&allowed_tools.iter().cloned().collect::<Vec<_>>().join(", "));
        prompt.push_str("\nTool argument contract:");
        prompt.push_str(
            "\n- repo.read_file -> {\"path\":\"relative/path.ext\",\"offset_chars\":0,\"length_chars\":12000,\"start_line\":1,\"end_line\":200}",
        );
        prompt.push_str(
            "\n- repo.write_file -> {\"path\":\"relative/path.ext\",\"content\":\"full file content\"}",
        );
        prompt.push_str(
            "\n- repo.replace_range -> {\"path\":\"relative/path.ext\",\"start_line\":10,\"end_line\":22,\"content\":\"replacement text\"}",
        );
        prompt.push_str(
            "\n- repo.apply_patch -> {\"path\":\"relative/path.ext\",\"patch\":\"unified diff hunk text\"}",
        );
        prompt.push_str("\n- repo.search -> {\"query\":\"text\",\"max_results\":20}");
        prompt.push_str("\n- repo.list_tree -> {\"path\":\"optional/subdir\",\"max_depth\":3}");
        prompt.push_str(
            "\nIMPORTANT: prefer repo.read_file with line/chunk arguments for large files.",
        );
        prompt.push_str(
            "\nPrefer repo.replace_range for surgical edits and repo.write_file only for intentional full-file rewrites.",
        );
        prompt.push_str(
            "\nUse repo.apply_patch only when you are certain context lines exactly match current file content.",
        );
        prompt.push_str("\nIf more information or edits are needed, return:");
        prompt.push_str(
            "\n{\"type\":\"tool_call\",\"tool\":\"...\",\"arguments\":{...},\"reason\":\"...\"}",
        );
        prompt.push_str("\nWhen implementation is complete, return:");
        prompt.push_str("\n{\"type\":\"final\",\"summary\":\"...\",\"result\":\"...\"}");
        prompt.push_str(&format!("\nCurrent step: {}/{}", step, max_steps));
        if max_steps.saturating_sub(step) <= 2 {
            prompt.push_str(
                "\nYou are close to the tool-step limit. Prefer finishing and returning type=final now unless a single essential tool call is still required.",
            );
        }
        if !observations.is_empty() {
            prompt.push_str("\n\nTool observations so far:\n");
            for item in observations.iter().rev().take(12).rev() {
                prompt.push_str("- ");
                prompt.push_str(item);
                prompt.push('\n');
            }
        }
        prompt
    }

    async fn try_force_tool_loop_finalization(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        observations: &[String],
        response_token_budget: i64,
    ) -> Result<Option<String>, AppError> {
        if observations.is_empty() {
            return Ok(None);
        }
        let mut prompt =
            String::from("Return exactly one JSON object with type=final. Do not call tools.\n");
        prompt.push_str("Output format:\n");
        prompt.push_str("{\"type\":\"final\",\"summary\":\"...\",\"result\":\"...\"}\n");
        prompt.push_str("Recent tool observations:\n");
        for item in observations.iter().rev().take(16).rev() {
            prompt.push_str("- ");
            prompt.push_str(item);
            prompt.push('\n');
        }
        let model_output = self
            .execute_agent_run(
                agent_run,
                model_def,
                &prompt,
                response_token_budget.min(1024),
            )
            .await?
            .content;
        match Self::parse_tool_loop_response(&model_output) {
            Some(ToolLoopResponse::Final { summary, result }) => {
                Ok(Some(summary.or(result).unwrap_or_else(|| {
                    "Coding stage finalized after tool-loop completion.".to_string()
                })))
            }
            _ => Ok(None),
        }
    }

    fn condense_tool_loop_base_prompt(base_prompt: &str) -> String {
        let mut condensed = String::new();
        for line in base_prompt.lines() {
            if line.starts_with("repository_context:") {
                condensed.push_str("repository_context: [initial repository summary omitted on follow-up turns; use tools for current file state]\n");
                continue;
            }
            condensed.push_str(line);
            condensed.push('\n');
            if condensed.len() >= 3_500 {
                condensed.push_str("...[truncated task recap]...\n");
                break;
            }
        }
        condensed
    }

    fn parse_tool_loop_response(output: &str) -> Option<ToolLoopResponse> {
        let payload = Self::extract_json_payload(output)?;
        serde_json::from_str::<ToolLoopResponse>(&payload).ok()
    }

    async fn write_tool_trace_snapshot(
        trace_path: &Path,
        trace: &[ToolLoopTraceEntry],
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(trace)?;
        tokio::fs::write(trace_path, payload)
            .await
            .map_err(|error| {
                AppError::Io(std::io::Error::other(format!(
                    "Failed to write tool trace snapshot: {}",
                    error
                )))
            })?;
        Ok(())
    }

    fn extract_json_payload(output: &str) -> Option<String> {
        let trimmed = output.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return Some(trimmed.to_string());
        }
        if let Some(start) = trimmed.find("```json") {
            let rest = &trimmed[start + 7..];
            if let Some(end) = rest.find("```") {
                return Some(rest[..end].trim().to_string());
            }
        }
        let start = trimmed.find('{')?;
        let end = trimmed.rfind('}')?;
        if end <= start {
            return None;
        }
        Some(trimmed[start..=end].to_string())
    }

    async fn execute_tool_call_in_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        tool: &str,
        arguments: &Value,
        allowed_tools: &HashSet<String>,
        boundaries: &AgentExecutionBoundaries,
        changed_files: &mut HashSet<String>,
    ) -> Result<Value, AppError> {
        let canonical_tool = tool.trim().to_ascii_lowercase();
        if !allowed_tools.contains(&canonical_tool) {
            return Err(AppError::Validation(format!(
                "Tool '{}' is not allowed for this agent",
                tool
            )));
        }

        match canonical_tool.as_str() {
            "repo.read_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.read_file requires 'path'".to_string())
                    })?;
                let normalized =
                    agent_execution_limits::normalize_relative_path(path).ok_or_else(|| {
                        AppError::Validation("Invalid path for repo.read_file".to_string())
                    })?;
                if !agent_execution_limits::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let content = workspace.read_file(&normalized).await?;
                let total_chars = agent_execution_limits::char_count(&content);
                let max_chars_per_read = agent_execution_limits::max_read_file_chars(boundaries);
                let requested_offset = arguments
                    .get("offset_chars")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let requested_length = arguments
                    .get("length_chars")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(max_chars_per_read);
                let start_line = arguments.get("start_line").and_then(Value::as_u64);
                let end_line = arguments.get("end_line").and_then(Value::as_u64);

                if (start_line.is_some() || end_line.is_some())
                    && (arguments.get("offset_chars").is_some()
                        || arguments.get("length_chars").is_some())
                {
                    return Err(AppError::Validation(
                        "repo.read_file cannot mix line-range arguments with offset/length"
                            .to_string(),
                    ));
                }

                let (selected_content, selected_start_line, selected_end_line, total_lines) =
                    if start_line.is_some() || end_line.is_some() {
                        let lines = content.lines().collect::<Vec<_>>();
                        let total_lines = lines.len();
                        if total_lines == 0 {
                            (String::new(), Some(1usize), Some(0usize), 0usize)
                        } else {
                            let start = start_line.unwrap_or(1) as usize;
                            let mut end = end_line.unwrap_or((start + 199) as u64) as usize;
                            if start == 0 {
                                return Err(AppError::Validation(
                                    "repo.read_file start_line must be >= 1".to_string(),
                                ));
                            }
                            if end < start {
                                return Err(AppError::Validation(
                                    "repo.read_file end_line must be >= start_line".to_string(),
                                ));
                            }
                            if start > total_lines {
                                return Err(AppError::Validation(format!(
                                    "repo.read_file start_line {} is beyond total lines {}",
                                    start, total_lines
                                )));
                            }
                            end = end.min(total_lines);
                            let range_content = lines[start - 1..end].join("\n");
                            (range_content, Some(start), Some(end), total_lines)
                        }
                    } else {
                        (content.clone(), None, None, content.lines().count())
                    };

                let selected_total_chars = agent_execution_limits::char_count(&selected_content);
                if requested_offset > selected_total_chars {
                    return Err(AppError::Validation(format!(
                        "repo.read_file offset_chars {} is beyond content length {}",
                        requested_offset, selected_total_chars
                    )));
                }
                let effective_length = requested_length.clamp(1, max_chars_per_read);
                let clipped = agent_execution_limits::substring_by_char_range(
                    &selected_content,
                    requested_offset,
                    effective_length,
                );
                let returned_chars = agent_execution_limits::char_count(&clipped);
                let next_offset = requested_offset + returned_chars;
                let truncated = next_offset < selected_total_chars;
                Ok(serde_json::json!({
                    "path": normalized,
                    "content": clipped,
                    "truncated": truncated,
                    "total_chars": selected_total_chars,
                    "returned_chars": returned_chars,
                    "offset_chars": requested_offset,
                    "next_offset_chars": if truncated { Some(next_offset) } else { None::<usize> },
                    "max_chars_per_read": max_chars_per_read,
                    "selection_total_chars": selected_total_chars,
                    "file_total_chars": total_chars,
                    "selection_start_line": selected_start_line,
                    "selection_end_line": selected_end_line,
                    "file_total_lines": total_lines,
                }))
            }
            "repo.list_tree" => {
                let base_path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                let max_depth = arguments
                    .get("max_depth")
                    .and_then(Value::as_u64)
                    .unwrap_or(3)
                    .clamp(1, 6) as usize;
                self.list_tree_from_workspace(workspace, base_path, max_depth, boundaries)
            }
            "repo.search" => {
                let query = arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.search requires 'query'".to_string())
                    })?;
                let max_results = arguments
                    .get("max_results")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .clamp(1, 80) as usize;
                self.search_workspace(workspace, query, max_results, boundaries)
            }
            "repo.write_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.write_file requires 'path'".to_string())
                    })?;
                let content = arguments
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.write_file requires 'content'".to_string())
                    })?;
                let normalized =
                    agent_execution_limits::normalize_relative_path(path).ok_or_else(|| {
                        AppError::Validation("Invalid path for repo.write_file".to_string())
                    })?;
                if !agent_execution_limits::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                agent_execution_limits::ensure_write_limit(content, boundaries)?;
                workspace.write_file(&normalized, content).await?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "bytes_written": content.len(),
                    "chars_written": agent_execution_limits::char_count(content),
                }))
            }
            "repo.replace_range" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'path'".to_string())
                    })?;
                let start_line = arguments
                    .get("start_line")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'start_line'".to_string())
                    })? as usize;
                let end_line = arguments
                    .get("end_line")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'end_line'".to_string())
                    })? as usize;
                let replacement = arguments
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'content'".to_string())
                    })?;
                if start_line == 0 {
                    return Err(AppError::Validation(
                        "repo.replace_range start_line must be >= 1".to_string(),
                    ));
                }
                if end_line < start_line {
                    return Err(AppError::Validation(
                        "repo.replace_range end_line must be >= start_line".to_string(),
                    ));
                }
                let normalized =
                    agent_execution_limits::normalize_relative_path(path).ok_or_else(|| {
                        AppError::Validation("Invalid path for repo.replace_range".to_string())
                    })?;
                if !agent_execution_limits::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let existing = workspace.read_file(&normalized).await?;
                let (lines, had_trailing_newline) =
                    agent_execution_limits::split_lines_preserve_trailing(&existing);
                if lines.is_empty() {
                    return Err(AppError::Validation(format!(
                        "repo.replace_range requires a non-empty file; use repo.write_file for {}",
                        normalized
                    )));
                }
                if end_line > lines.len() {
                    return Err(AppError::Validation(format!(
                        "repo.replace_range end_line {} is beyond total lines {}",
                        end_line,
                        lines.len()
                    )));
                }
                let replacement_lines = replacement
                    .split('\n')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                let mut merged = Vec::with_capacity(
                    lines.len() - (end_line - start_line + 1) + replacement_lines.len(),
                );
                merged.extend_from_slice(&lines[..start_line - 1]);
                merged.extend(replacement_lines);
                merged.extend_from_slice(&lines[end_line..]);
                let mut updated = merged.join("\n");
                if had_trailing_newline {
                    updated.push('\n');
                }
                agent_execution_limits::ensure_write_limit(&updated, boundaries)?;
                workspace.write_file(&normalized, &updated).await?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "start_line": start_line,
                    "end_line": end_line,
                    "replaced_lines": end_line.saturating_sub(start_line) + 1,
                }))
            }
            "repo.apply_patch" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.apply_patch requires 'path'".to_string())
                    })?;
                let patch = arguments
                    .get("patch")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.apply_patch requires 'patch'".to_string())
                    })?;
                let base_sha256 = arguments.get("base_sha256").and_then(Value::as_str);
                let normalized =
                    agent_execution_limits::normalize_relative_path(path).ok_or_else(|| {
                        AppError::Validation("Invalid path for repo.apply_patch".to_string())
                    })?;
                if !agent_execution_limits::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let new_hash = repo_service::apply_repository_patch(
                    &workspace.repo_path.to_string_lossy(),
                    &normalized,
                    patch,
                    base_sha256,
                )?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "sha256": new_hash,
                }))
            }
            _ => Err(AppError::Validation(format!(
                "Unsupported tool call: {}",
                tool
            ))),
        }
    }

    fn list_tree_from_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        base_path: &str,
        max_depth: usize,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Value, AppError> {
        let normalized = if base_path.is_empty() {
            None
        } else {
            Some(
                agent_execution_limits::normalize_relative_path(base_path).ok_or_else(|| {
                    AppError::Validation("Invalid base path for repo.list_tree".to_string())
                })?,
            )
        };
        if let Some(path) = normalized.as_deref() {
            if !agent_execution_limits::is_repo_relative_path_allowed(path, boundaries) {
                return Err(AppError::Validation(format!(
                    "Path is outside boundaries: {}",
                    path
                )));
            }
        }

        let root = if let Some(path) = normalized.as_deref() {
            workspace.repo_path.join(path)
        } else {
            workspace.repo_path.clone()
        };
        if !root.exists() || !root.is_dir() {
            return Err(AppError::Validation(
                "repo.list_tree target is not a directory".to_string(),
            ));
        }

        let mut entries: Vec<Value> = Vec::new();
        for entry in WalkDir::new(&root)
            .max_depth(max_depth + 1)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0
                    || !entry
                        .file_name()
                        .to_str()
                        .is_some_and(agent_execution_limits::ignored_repository_dir)
            })
            .filter_map(Result::ok)
        {
            if entries.len() >= 350 {
                break;
            }
            if entry.path() == root {
                continue;
            }
            let rel = match entry.path().strip_prefix(&workspace.repo_path) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if agent_execution_limits::path_has_ignored_repository_component(&rel) {
                continue;
            }
            if !agent_execution_limits::is_repo_relative_path_allowed(&rel, boundaries) {
                continue;
            }
            entries.push(serde_json::json!({
                "path": rel,
                "kind": if entry.file_type().is_dir() { "directory" } else { "file" },
            }));
        }
        Ok(serde_json::json!({ "entries": entries }))
    }

    fn search_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        query: &str,
        max_results: usize,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Value, AppError> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err(AppError::Validation(
                "repo.search query cannot be empty".to_string(),
            ));
        }
        let mut results: Vec<Value> = Vec::new();
        for entry in WalkDir::new(&workspace.repo_path)
            .max_depth(8)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0
                    || !entry
                        .file_name()
                        .to_str()
                        .is_some_and(agent_execution_limits::ignored_repository_dir)
            })
            .filter_map(Result::ok)
        {
            if results.len() >= max_results {
                break;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = match entry.path().strip_prefix(&workspace.repo_path) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if agent_execution_limits::path_has_ignored_repository_component(&rel) {
                continue;
            }
            if !agent_execution_limits::is_repo_relative_path_allowed(&rel, boundaries)
                || !agent_execution_limits::is_text_source_file(&rel)
            {
                continue;
            }
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(text) => text,
                Err(_) => continue,
            };
            for (index, line) in content.lines().enumerate() {
                if line.to_ascii_lowercase().contains(&needle) {
                    results.push(serde_json::json!({
                        "path": rel,
                        "line": index + 1,
                        "text": line.trim(),
                    }));
                    if results.len() >= max_results {
                        break;
                    }
                }
            }
        }
        Ok(serde_json::json!({ "query": query, "results": results }))
    }

    /// Apply code changes from the coding agent output
    pub(super) async fn apply_code_changes(
        &self,
        agent_run: &AgentRun,
        output: &str,
    ) -> Result<(), AppError> {
        debug!(agent_run_id = %agent_run.id, output_length = output.len(), "Applying code changes from agent output");

        // Get the workflow run to obtain work_item_id
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item_id = workflow_run.work_item_id;
        debug!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "Retrieved workflow run for work item");

        // Get the work item and repository info
        let work_item = work_item_repo::get_work_item(&self.db, &work_item_id).await?;
        let agent_def = agent_repo::get_agent_definition(&self.db, &agent_run.agent_id).await?;
        let boundaries = agent_execution_limits::parse_boundaries(&agent_def.boundaries);
        let repo = if let Some(repo_id) = &work_item.active_repo_id {
            Some(crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?)
        } else {
            None
        };

        if let Some(repo) = repo {
            debug!(agent_run_id = %agent_run.id, repo_id = %repo.id, repo_path = %repo.local_path, "Found active repository for work item");

            // Create isolated workspace
            let workspace = WorkItemWorkspace::create(
                &work_item,
                &agent_run.workflow_run_id,
                &repo,
                &self.workspace_base_path,
            )
            .await?;
            info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Created isolated workspace");

            // Parse and apply code changes
            let changed_files = self
                .parse_and_apply_changes(&workspace, output, &boundaries)
                .await?;
            info!(agent_run_id = %agent_run.id, changed_files = changed_files.len(), "Successfully parsed and applied code changes");

            if changed_files.is_empty() {
                warn!(agent_run_id = %agent_run.id, "Coding stage produced no file edits");
            } else {
                workspace
                    .sync_files_back(&repo.local_path, &changed_files)
                    .await?;
                info!(agent_run_id = %agent_run.id, changed_files = changed_files.len(), repo_path = %repo.local_path, "Synced changed files back to repository");

                let artifact_dir = self.artifact_base_path.join(&agent_run.id);
                tokio::fs::create_dir_all(&artifact_dir).await?;
                let applied_files_path = artifact_dir.join("applied_files.txt");
                let applied_summary = changed_files.join("\n");
                tokio::fs::write(&applied_files_path, &applied_summary).await?;

                let artifact_id = uuid::Uuid::new_v4().to_string();
                artifact_repo::create_artifact(
                    &self.db,
                    artifact_repo::CreateArtifactInput {
                        id: &artifact_id,
                        work_item_id: &work_item_id,
                        workflow_run_id: Some(&agent_run.workflow_run_id),
                        agent_run_id: Some(&agent_run.id),
                        artifact_type: "coding_applied_files",
                        summary: &format!("Applied {} files to repository", changed_files.len()),
                        storage_path: &applied_files_path.to_string_lossy(),
                    },
                )
                .await?;
            }

            // TODO: Generate diff and store as artifact

            if agent_execution_limits::should_keep_workspace(&boundaries) {
                info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Retaining workspace for inspection");
            } else {
                workspace.cleanup().await?;
                debug!(agent_run_id = %agent_run.id, "Cleaned up workspace after processing");
            }
        } else {
            warn!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "No active repository found for work item, skipping code changes");
        }

        info!(agent_run_id = %agent_run.id, "Successfully applied code changes");
        Ok(())
    }

    /// Parse coding agent output and apply file changes
    async fn parse_and_apply_changes(
        &self,
        workspace: &WorkItemWorkspace,
        output: &str,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Vec<String>, AppError> {
        debug!(workspace_path = %workspace.base_path.display(), output_length = output.len(), "Parsing and applying code changes");

        // Simple parsing - look for file markers like "File: path/to/file"
        // This is a basic implementation - in production, you'd want more robust parsing

        let lines: Vec<&str> = output.lines().collect();
        let mut current_file: Option<String> = None;
        let mut file_content = String::new();
        let mut files_processed = 0usize;
        let mut changed_files: Vec<String> = Vec::new();
        let max_files = agent_execution_limits::max_files_per_run(boundaries);

        for line in lines {
            if line.starts_with("File: ") || line.starts_with("### File: ") {
                // Save previous file if any
                if let Some(file_path) = current_file.take() {
                    if !file_content.trim().is_empty() {
                        if files_processed >= max_files {
                            warn!(workspace_path = %workspace.base_path.display(), max_files = max_files, "Reached max files per run boundary; skipping additional file updates");
                            break;
                        }
                        let Some(normalized_path) =
                            agent_execution_limits::normalize_relative_path(&file_path)
                        else {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %file_path, "Skipping invalid relative file path from agent output");
                            file_content.clear();
                            continue;
                        };
                        if !agent_execution_limits::is_repo_relative_path_allowed(
                            &normalized_path,
                            boundaries,
                        ) {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, "Skipping file outside boundaries");
                            file_content.clear();
                            continue;
                        }
                        agent_execution_limits::ensure_write_limit(&file_content, boundaries)?;
                        workspace
                            .write_file(&normalized_path, &file_content)
                            .await?;
                        files_processed += 1;
                        changed_files.push(normalized_path.clone());
                        debug!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, content_length = file_content.len(), "Wrote file content");
                    }
                }

                // Start new file
                let raw = line
                    .trim_start_matches("File: ")
                    .trim_start_matches("### File: ")
                    .trim();
                current_file = Some(raw.to_string());
                file_content.clear();
                debug!(workspace_path = %workspace.base_path.display(), file_path = current_file.as_ref().unwrap(), "Started processing new file");
            } else if let Some(ref _file_path) = current_file {
                if line.trim_start().starts_with("```") {
                    continue;
                }
                file_content.push_str(line);
                file_content.push('\n');
            }
        }

        // Save the last file
        if let Some(file_path) = current_file {
            if !file_content.trim().is_empty() {
                if files_processed < max_files {
                    if let Some(normalized_path) =
                        agent_execution_limits::normalize_relative_path(&file_path)
                    {
                        if agent_execution_limits::is_repo_relative_path_allowed(
                            &normalized_path,
                            boundaries,
                        ) {
                            agent_execution_limits::ensure_write_limit(&file_content, boundaries)?;
                            workspace
                                .write_file(&normalized_path, &file_content)
                                .await?;
                            files_processed += 1;
                            changed_files.push(normalized_path.clone());
                            debug!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, content_length = file_content.len(), "Wrote final file content");
                        } else {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, "Skipping final file outside boundaries");
                        }
                    }
                } else {
                    warn!(workspace_path = %workspace.base_path.display(), max_files = max_files, "Skipped final file due to max files boundary");
                }
            }
        }

        info!(workspace_path = %workspace.base_path.display(), files_processed = files_processed, "Successfully parsed and applied code changes");
        Ok(changed_files)
    }
}

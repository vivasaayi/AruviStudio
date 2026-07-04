use super::AgentService;
use crate::domain::agent::AgentRun;
use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum ToolLoopResponse {
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
pub(super) struct ToolLoopTraceEntry {
    pub(super) step: usize,
    pub(super) kind: String,
    pub(super) payload: String,
}

pub(super) fn resolve_allowed_tools(configured_tools: &[String]) -> HashSet<String> {
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

pub(super) fn build_coding_tool_prompt(
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
        prompt.push_str(&condense_tool_loop_base_prompt(base_prompt));
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
    prompt
        .push_str("\nIMPORTANT: prefer repo.read_file with line/chunk arguments for large files.");
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

pub(super) fn parse_tool_loop_response(output: &str) -> Option<ToolLoopResponse> {
    let payload = extract_json_payload(output)?;
    serde_json::from_str::<ToolLoopResponse>(&payload).ok()
}

pub(super) async fn write_tool_trace_snapshot(
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

impl AgentService {
    pub(super) async fn try_force_tool_loop_finalization(
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
        match parse_tool_loop_response(&model_output) {
            Some(ToolLoopResponse::Final { summary, result }) => {
                Ok(Some(summary.or(result).unwrap_or_else(|| {
                    "Coding stage finalized after tool-loop completion.".to_string()
                })))
            }
            _ => Ok(None),
        }
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

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
        #[serde(default)]
        no_changes_needed: bool,
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
        match lowered.as_str() {
            "repo.list_tree" | "repo.read_file" | "repo.search" | "repo.write_file"
            | "repo.replace_range" | "repo.apply_patch" => {
                allowed.insert(lowered);
            }
            "read" | "read_file" | "read_files" | "read_code" | "read_context" | "read_tests" => {
                allowed.extend(
                    ["repo.read_file", "repo.list_tree", "repo.search"]
                        .into_iter()
                        .map(str::to_string),
                );
            }
            "write" | "write_file" | "write_files" | "write_tests" | "create" | "create_file"
            | "create_files" => {
                allowed.extend(
                    ["repo.write_file", "repo.replace_range"]
                        .into_iter()
                        .map(str::to_string),
                );
            }
            "modify" | "modify_code" | "patch" | "apply_patch" => {
                allowed.extend(
                    ["repo.apply_patch", "repo.replace_range"]
                        .into_iter()
                        .map(str::to_string),
                );
            }
            _ => {}
        }
    }
    // An unrecognized nonempty configuration grants no tools. An empty configuration
    // retains the historical defaults above for existing agents.
    allowed
}

pub(super) fn build_coding_tool_prompt(
    base_prompt: &str,
    allowed_tools: &HashSet<String>,
    observations: &[String],
    step: usize,
    max_steps: usize,
    has_changes: bool,
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
    let mut ordered_tools = allowed_tools.iter().map(String::as_str).collect::<Vec<_>>();
    ordered_tools.sort_unstable();
    prompt.push_str(&ordered_tools.join(", "));
    prompt.push_str("\nTool argument contract:");
    prompt.push_str(
        "\n- repo.read_file -> {\"path\":\"relative/path.ext\",\"offset_chars\":0,\"length_chars\":12000} OR {\"path\":\"relative/path.ext\",\"start_line\":1,\"end_line\":200}. Do not combine offset/length with line ranges.",
    );
    prompt.push_str(
        "\n- repo.write_file -> {\"path\":\"relative/path.ext\",\"content\":\"full file content\"}",
    );
    prompt.push_str("\n  A missing file cannot be read. When the task requires a new file and writing is allowed, use repo.write_file with its complete content.");
    prompt.push_str("\n  For new files, repo.write_file creates missing parent directories automatically; do not stop because a directory is absent. Its content must include the complete implementation, including definitions for exported functions, not just export statements.");
    prompt.push_str(
        "\n- repo.replace_range -> {\"path\":\"relative/path.ext\",\"start_line\":10,\"end_line\":22,\"content\":\"replacement text\"}",
    );
    prompt.push_str("\n  Include only replacement lines for the selected range, not unchanged neighboring lines. If an adjacent duplicate is intentional, set allow_adjacent_duplicate=true.");
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
    prompt.push_str("\nIf no files need to change, return type=final with no_changes_needed=true and explain why in summary. Otherwise write the files with a tool before finishing.");
    if allowed_tools.contains("repo.read_file") {
        prompt.push_str("\nAfter a successful write, inspect the current file if needed. Do not repeat an edit that already succeeded; finish when the requested implementation is complete.");
    } else {
        prompt.push_str("\nDo not repeat an edit that already succeeded; finish when the requested implementation is complete.");
    }
    prompt.push_str(&format!("\nCurrent step: {}/{}", step, max_steps));
    if max_steps.saturating_sub(step) <= 2 {
        if has_changes {
            prompt.push_str("\nYou are close to the tool-step limit. If the requested implementation is complete, return type=final; otherwise perform an essential remaining tool call.");
        } else {
            prompt.push_str("\nYou are close to the tool-step limit and no files have been changed. Do not claim an edit happened. If the task requires an edit and an allowed write tool can perform it, make that edit now.");
        }
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

pub(super) fn parser_error_observation(
    step: usize,
    output: &str,
    ambiguous: bool,
    allowed_tools: &HashSet<String>,
) -> String {
    use crate::services::agent_execution_limits::truncate_for_model_context;
    if ambiguous {
        return format!("tool_error step={} tool=parser error=Response contained multiple actions; no action was executed. Return exactly one tool_call OR one final object, not both.", step);
    }
    let mut message = format!("tool_error step={} tool=parser error=Response did not match tool/final JSON contract; no action was executed.", step);
    // A common local-model mistake places the tool name in `type` and its
    // arguments at top level. Offer a bounded corrected envelope as data only;
    // the model must submit a new action and normal tool authorization applies.
    if let Some(payload) = extract_json_payload(output) {
        if let Ok(Value::Object(mut object)) = serde_json::from_str::<Value>(&payload) {
            let tool = object.remove("type").and_then(|value| value.as_str().map(str::to_owned));
            if let Some(tool) = tool.filter(|name| allowed_tools.contains(name)) {
                let reason = object.remove("reason").filter(Value::is_string);
                let corrected = serde_json::json!({"type":"tool_call", "tool":tool, "arguments":object, "reason":reason});
                let serialized = serde_json::to_string(&corrected).unwrap_or_default();
                if serialized.len() <= 1600 {
                    message.push_str(" To retry this attempted action, return exactly this single object (no action has run): ");
                    message.push_str(&serialized);
                    return message;
                }
            }
        }
    }
    if allowed_tools.contains("repo.write_file") {
        message.push_str(" If the previous response contains intended file content, submit it as one repo.write_file tool_call with path and complete content; a final response does not write files.");
    }
    let excerpt = truncate_for_model_context(output, 1600);
    message.push_str(" Previous invalid response (quoted data): ");
    message.push_str(&serde_json::to_string(&excerpt).unwrap_or_default());
    message
}

pub(super) fn failed_tool_observation(
    step: usize,
    tool: &str,
    reason: Option<&str>,
    error: &AppError,
    allowed_tools: &HashSet<String>,
) -> String {
    use crate::services::agent_execution_limits::truncate_for_model_context;
    let mut observation = format!(
        "tool_error step={} tool={} reason={} error={}",
        step,
        truncate_for_model_context(tool, 200),
        truncate_for_model_context(reason.unwrap_or_default(), 300),
        truncate_for_model_context(&error.to_string(), 2_000)
    );
    if tool.trim().eq_ignore_ascii_case("repo.read_file") && matches!(error, AppError::NotFound(_))
    {
        observation
            .push_str("; the file is absent; do not repeat this read unless the file changes.");
        if allowed_tools.contains("repo.write_file") {
            observation.push_str(
                " If the task requires a new file, use repo.write_file with complete content.",
            );
        }
    }
    observation
}

/// Detect distinct action-shaped objects without counting nested objects or
/// braces inside JSON strings. Ambiguous output must not execute an action.
pub(super) fn has_multiple_actions(output: &str) -> bool {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut objects = 0usize;
    for (index, ch) in output.char_indices() {
        if depth == 0 {
            if ch == '{' {
                depth = 1;
                start = index;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else {
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0
                        && serde_json::from_str::<Value>(&output[start..=index])
                            .ok()
                            .and_then(|value| {
                                value.get("type").and_then(Value::as_str).map(str::to_owned)
                            })
                            .is_some_and(|kind| matches!(kind.as_str(), "tool_call" | "final" | "repo.read_file" | "repo.list_tree" | "repo.search" | "repo.write_file" | "repo.replace_range" | "repo.apply_patch"))
                    {
                        objects += 1;
                        if objects > 1 {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    false
}

pub(super) fn parse_tool_loop_response(output: &str) -> Option<ToolLoopResponse> {
    if has_multiple_actions(output) {
        return None;
    }
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
        json_object: bool,
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
                json_object,
            )
            .await?
            .content;
        match parse_tool_loop_response(&model_output) {
            Some(ToolLoopResponse::Final {
                summary, result, ..
            }) => Ok(Some(summary.or(result).unwrap_or_else(|| {
                "Coding stage finalized after tool-loop completion.".to_string()
            }))),
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

#[cfg(test)]
mod configured_tool_tests {
    use super::*;

    #[test]
    fn rejects_multiple_actions_without_confusing_nested_or_quoted_braces() {
        let single = r#"Here is the call:
```json
{"type":"tool_call","tool":"repo.write_file","arguments":{"path":"a.js","content":"const x = {value: \"}\"};"}}
```"#;
        assert!(matches!(
            parse_tool_loop_response(single),
            Some(ToolLoopResponse::ToolCall { .. })
        ));
        assert!(!has_multiple_actions(single));
        let two_fenced = "```json\n{\"type\":\"tool_call\",\"tool\":\"repo.read_file\",\"arguments\":{\"path\":\"a\"}}\n```\n```json\n{\"type\":\"final\",\"summary\":\"done\"}\n```";
        assert!(has_multiple_actions(two_fenced));
        assert!(parse_tool_loop_response(two_fenced).is_none());
        let two_plain = r#"Tool call: {"type":"tool_call","tool":"repo.read_file","arguments":{"path":"a"}} Final answer: {"type":"final","summary":"done"}"#;
        assert!(has_multiple_actions(two_plain));
        assert!(parse_tool_loop_response(two_plain).is_none());
        let incomplete_action = r#"```json
{"type":"tool_call","tool":"repo.read_file","arguments":{"path":"a"}}
```
{"type":"tool_call"}"#;
        assert!(has_multiple_actions(incomplete_action));
        assert!(parse_tool_loop_response(incomplete_action).is_none());
        let legacy = "File: src/a.js\n```js\nconst a = {\"x\":1}; const b = {\"y\":2};\n```";
        assert!(!has_multiple_actions(legacy));
    }

    #[test]
    fn flat_tool_calls_are_rejected_but_receive_allowed_targeted_guidance() {
        let flat = r#"Tool call: {"type":"repo.replace_range","path":"a.js","start_line":2,"end_line":2,"content":"new","reason":"fix"}"#;
        assert!(parse_tool_loop_response(flat).is_none());
        let guidance = parser_error_observation(3, flat, false, &resolve_allowed_tools(&[]));
        assert!(guidance.contains("no action was executed"));
        assert!(guidance.contains(r#""type":"tool_call""#));
        assert!(guidance.contains(r#""tool":"repo.replace_range""#));
        assert!(guidance.contains(r#""start_line":2"#));
        assert!(!guidance.contains("complete content"));
        let restricted = parser_error_observation(3, flat, false, &resolve_allowed_tools(&["read".into()]));
        assert!(!restricted.contains("To retry this attempted action"));
        let multiple = format!(r#"{{"type":"final","summary":"done"}} {flat}"#);
        assert!(has_multiple_actions(&multiple));
        assert!(parse_tool_loop_response(&multiple).is_none());
        assert!(!parser_error_observation(3, &multiple, true, &resolve_allowed_tools(&[])).contains("To retry"));
    }

    #[test]
    fn prompt_orders_tools_and_distinguishes_unfinished_edits() {
        let tools = resolve_allowed_tools(&[]);
        let no_changes = build_coding_tool_prompt("task", &tools, &[], 8, 8, false);
        let changed = build_coding_tool_prompt("task", &tools, &[], 8, 8, true);
        assert!(no_changes.contains("no files have been changed"));
        assert!(!changed.contains("no files have been changed"));
        assert!(changed.contains("If the requested implementation is complete"));
        assert!(no_changes.contains("Allowed tools: repo.apply_patch, repo.list_tree, repo.read_file, repo.replace_range, repo.search, repo.write_file"));
    }

    #[test]
    fn missing_read_guidance_respects_write_permissions_and_is_bounded() {
        let error = AppError::NotFound("Repository file does not exist: src/new.js".into());
        let writer = resolve_allowed_tools(&[]);
        let reader = resolve_allowed_tools(&["repo.read_file".into()]);
        let reason = "x".repeat(50_000);
        let allowed = failed_tool_observation(1, "repo.read_file", Some(&reason), &error, &writer);
        assert!(allowed.contains("do not repeat this read"));
        assert!(allowed.contains("repo.write_file"));
        assert!(allowed.len() < 4_000);
        let restricted = failed_tool_observation(1, "repo.read_file", None, &error, &reader);
        assert!(restricted.contains("do not repeat this read"));
        assert!(!restricted.contains("repo.write_file"));
    }

    #[test]
    fn read_examples_use_alternative_ranges() {
        let prompt =
            build_coding_tool_prompt("task", &resolve_allowed_tools(&[]), &[], 1, 8, false);
        assert!(prompt.contains("Do not combine offset/length with line ranges"));
        assert!(prompt.contains("A missing file cannot be read"));
        assert!(!prompt.contains("\"length_chars\":12000,\"start_line\""));
    }

    #[test]
    fn unknown_nonempty_configuration_does_not_grant_write_access() {
        assert!(resolve_allowed_tools(&["unrecognized_tool".into()]).is_empty());
        assert!(!resolve_allowed_tools(&["read_files".into()]).contains("repo.write_file"));
        assert!(resolve_allowed_tools(&["do_not_write".into()]).is_empty());
        assert_eq!(
            resolve_allowed_tools(&["repo.read_file".into()]),
            ["repo.read_file".to_string()].into_iter().collect()
        );
        assert!(resolve_allowed_tools(&["write_files".into()]).contains("repo.write_file"));
        assert!(resolve_allowed_tools(&[]).contains("repo.write_file"));
    }
    #[test]
    fn parser_feedback_preserves_bounded_output_and_respects_permissions() {
        let writer = resolve_allowed_tools(&["repo.write_file".into()]);
        let reader = resolve_allowed_tools(&["repo.read_file".into()]);
        let output = format!("{{\"result\":\"candidate\"}}{}", "x".repeat(2000));
        let feedback = parser_error_observation(2, &output, false, &writer);
        assert!(feedback.contains("candidate"));
        assert!(feedback.contains("repo.write_file"));
        assert!(feedback.contains("truncated for model context"));
        assert!(!parser_error_observation(2, &output, false, &reader).contains("repo.write_file"));
        let ambiguous = parser_error_observation(2, &output, true, &writer);
        assert!(ambiguous.contains("multiple actions"));
        assert!(!ambiguous.contains("candidate"));
    }
}

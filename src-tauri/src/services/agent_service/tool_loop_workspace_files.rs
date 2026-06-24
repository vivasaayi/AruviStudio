use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use serde_json::Value;
use std::collections::HashSet;

pub(super) async fn read_workspace_file(
    workspace: &WorkItemWorkspace,
    arguments: &Value,
    boundaries: &AgentExecutionBoundaries,
) -> Result<Value, AppError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("repo.read_file requires 'path'".to_string()))?;
    let normalized = agent_execution_limits::normalize_relative_path(path)
        .ok_or_else(|| AppError::Validation("Invalid path for repo.read_file".to_string()))?;
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
        && (arguments.get("offset_chars").is_some() || arguments.get("length_chars").is_some())
    {
        return Err(AppError::Validation(
            "repo.read_file cannot mix line-range arguments with offset/length".to_string(),
        ));
    }

    let (selected_content, selected_start_line, selected_end_line, total_lines) =
        if start_line.is_some() || end_line.is_some() {
            select_line_range(&content, start_line, end_line)?
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

pub(super) async fn write_workspace_file(
    workspace: &WorkItemWorkspace,
    arguments: &Value,
    boundaries: &AgentExecutionBoundaries,
    changed_files: &mut HashSet<String>,
) -> Result<Value, AppError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("repo.write_file requires 'path'".to_string()))?;
    let content = arguments
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("repo.write_file requires 'content'".to_string()))?;
    let normalized = agent_execution_limits::normalize_relative_path(path)
        .ok_or_else(|| AppError::Validation("Invalid path for repo.write_file".to_string()))?;
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

pub(super) async fn replace_workspace_range(
    workspace: &WorkItemWorkspace,
    arguments: &Value,
    boundaries: &AgentExecutionBoundaries,
    changed_files: &mut HashSet<String>,
) -> Result<Value, AppError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("repo.replace_range requires 'path'".to_string()))?;
    let start_line = arguments
        .get("start_line")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            AppError::Validation("repo.replace_range requires 'start_line'".to_string())
        })? as usize;
    let end_line = arguments
        .get("end_line")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::Validation("repo.replace_range requires 'end_line'".to_string()))?
        as usize;
    let replacement = arguments
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("repo.replace_range requires 'content'".to_string()))?;
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

    let normalized = agent_execution_limits::normalize_relative_path(path)
        .ok_or_else(|| AppError::Validation("Invalid path for repo.replace_range".to_string()))?;
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
    let mut merged =
        Vec::with_capacity(lines.len() - (end_line - start_line + 1) + replacement_lines.len());
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

fn select_line_range(
    content: &str,
    start_line: Option<u64>,
    end_line: Option<u64>,
) -> Result<(String, Option<usize>, Option<usize>, usize), AppError> {
    let lines = content.lines().collect::<Vec<_>>();
    let total_lines = lines.len();
    if total_lines == 0 {
        return Ok((String::new(), Some(1), Some(0), 0));
    }

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
    Ok((
        lines[start - 1..end].join("\n"),
        Some(start),
        Some(end),
        total_lines,
    ))
}

use super::AgentService;
use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use crate::services::agent_service::tool_loop_workspace_files::{
    read_workspace_file, replace_workspace_range, write_workspace_file,
};
use crate::services::repo_service;
use serde_json::Value;
use std::collections::HashSet;
use walkdir::WalkDir;

impl AgentService {
    pub(super) async fn execute_tool_call_in_workspace(
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
            "repo.read_file" => read_workspace_file(workspace, arguments, boundaries).await,
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
                write_workspace_file(workspace, arguments, boundaries, changed_files).await
            }
            "repo.replace_range" => {
                replace_workspace_range(workspace, arguments, boundaries, changed_files).await
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
}

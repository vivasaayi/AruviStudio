use crate::error::AppError;
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use std::path::Path;

pub(crate) fn build_repository_context(
    repo_path: &str,
    boundaries: &AgentExecutionBoundaries,
    context_budget_chars: usize,
) -> Result<String, AppError> {
    let repo_root = Path::new(repo_path);
    if !repo_root.exists() || !repo_root.is_dir() {
        return Ok(String::new());
    }

    let max_repo_files_scanned = boundaries
        .max_repo_files_scanned
        .unwrap_or(600)
        .clamp(50, 5_000);
    let mut stack = vec![repo_root.to_path_buf()];
    let mut files: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if agent_execution_limits::ignored_repository_dir(file_name.as_str()) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            if scanned >= max_repo_files_scanned {
                break;
            }
            scanned += 1;
            let rel = match path.strip_prefix(repo_root) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if !agent_execution_limits::is_repo_relative_path_allowed(&rel, boundaries) {
                continue;
            }
            if !agent_execution_limits::is_text_source_file(&rel) {
                continue;
            }
            files.push(rel);
        }
        if scanned >= max_repo_files_scanned {
            break;
        }
    }

    files.sort();
    if files.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();
    output.push_str("Repository file manifest:\n");
    for file in files.iter().take(80) {
        output.push_str("- ");
        output.push_str(file);
        output.push('\n');
    }

    let max_files = agent_execution_limits::max_files_per_run(boundaries);
    let max_file_chars = agent_execution_limits::max_repo_snippet_chars(boundaries);
    let mut used_chars = output.len();
    let mut snippets_added = 0usize;
    let snippet_budget = context_budget_chars
        .saturating_sub(used_chars)
        .min((context_budget_chars / 4).max(2_500))
        .min(6_000);

    let prioritized_files = agent_execution_limits::prioritize_repository_files(&files);
    for rel in &prioritized_files {
        if snippets_added >= max_files || used_chars >= snippet_budget {
            break;
        }
        let file_path = repo_root.join(rel);
        let metadata = match std::fs::metadata(&file_path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if metadata.len() > 300_000 {
            continue;
        }
        let content = match std::fs::read_to_string(&file_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if content.trim().is_empty() {
            continue;
        }
        let remaining = snippet_budget.saturating_sub(used_chars);
        if remaining < 100 {
            break;
        }
        let take_chars = remaining.min(max_file_chars);
        let snippet = content.chars().take(take_chars).collect::<String>();
        output.push_str("\nFile Snippet:\nPath: ");
        output.push_str(rel);
        output.push('\n');
        output.push_str(&snippet);
        if snippet.len() < content.len() {
            output.push_str("\n...[truncated]...\n");
        } else {
            output.push('\n');
        }
        used_chars = output.len();
        snippets_added += 1;
    }

    Ok(output)
}

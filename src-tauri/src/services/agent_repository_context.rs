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
    let canonical_root = std::fs::canonicalize(repo_root)?;

    let max_repo_files_scanned = boundaries
        .max_repo_files_scanned
        .unwrap_or(600)
        .clamp(50, 5_000);
    let mut stack = vec![repo_root.to_path_buf()];
    let mut files: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    while let Some(dir) = stack.pop() {
        if !std::fs::canonicalize(&dir).is_ok_and(|path| path.starts_with(&canonical_root)) {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = match entry.file_type() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if file_type.is_symlink() { continue; }
            if file_type.is_dir() {
                if agent_execution_limits::ignored_repository_dir(file_name.as_str()) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
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
        let metadata = match std::fs::symlink_metadata(&file_path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if !metadata.file_type().is_file() || metadata.len() > 300_000 {
            continue;
        }
        if !std::fs::canonicalize(&file_path).is_ok_and(|path| path.starts_with(&canonical_root)) {
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn repository_context_skips_external_symlink_files_and_directories() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&repo).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(repo.join("main.rs"), "fn safe() {}\n").unwrap();
        std::fs::write(outside.join("private.rs"), "EXTERNAL_PRIVATE_MARKER\n").unwrap();
        std::os::unix::fs::symlink(outside.join("private.rs"), repo.join("linked.rs")).unwrap();
        std::os::unix::fs::symlink(&outside, repo.join("linked-dir")).unwrap();
        let context = build_repository_context(repo.to_str().unwrap(), &AgentExecutionBoundaries::default(), 12000).unwrap();
        assert!(context.contains("main.rs"));
        assert!(!context.contains("linked.rs"));
        assert!(!context.contains("linked-dir"));
        assert!(!context.contains("EXTERNAL_PRIVATE_MARKER"));
    }
}

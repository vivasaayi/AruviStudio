use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use serde::Deserialize;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct AgentExecutionBoundaries {
    pub(crate) instructions: Option<String>,
    pub(crate) max_tokens: Option<i64>,
    pub(crate) max_context_chars: Option<usize>,
    pub(crate) context_window_ratio: Option<f32>,
    pub(crate) max_files_per_run: Option<usize>,
    pub(crate) max_read_file_chars: Option<usize>,
    pub(crate) max_write_file_chars: Option<usize>,
    pub(crate) max_file_chars: Option<usize>,
    pub(crate) max_repo_files_scanned: Option<usize>,
    pub(crate) allowed_paths: Option<Vec<String>>,
    pub(crate) blocked_paths: Option<Vec<String>>,
    pub(crate) keep_workspace: Option<bool>,
    pub(crate) max_tool_steps: Option<usize>,
}

pub(crate) fn parse_boundaries(raw: &serde_json::Value) -> AgentExecutionBoundaries {
    serde_json::from_value::<AgentExecutionBoundaries>(raw.clone()).unwrap_or_default()
}

pub(crate) fn resolve_context_char_budget(
    model_def: &ModelDefinition,
    boundaries: &AgentExecutionBoundaries,
) -> usize {
    if let Some(explicit) = boundaries.max_context_chars {
        return explicit.clamp(4_000, 120_000);
    }
    let ratio = boundaries
        .context_window_ratio
        .unwrap_or(0.25)
        .clamp(0.1, 0.9) as f64;
    if let Some(context_window) = model_def.context_window {
        let estimated_chars = ((context_window as f64) * ratio * 4.0) as usize;
        return estimated_chars.clamp(4_000, 120_000);
    }
    12_000
}

pub(crate) fn resolve_response_token_budget(
    model_def: &ModelDefinition,
    boundaries: &AgentExecutionBoundaries,
) -> i64 {
    let requested = boundaries.max_tokens.unwrap_or(4_096).clamp(256, 16_384);
    if let Some(context_window) = model_def.context_window {
        let ceiling = ((context_window as f64) * 0.4) as i64;
        return requested.min(ceiling.max(512));
    }
    requested
}

pub(crate) fn max_files_per_run(boundaries: &AgentExecutionBoundaries) -> usize {
    boundaries.max_files_per_run.unwrap_or(3).clamp(1, 200)
}

pub(crate) fn max_read_file_chars(boundaries: &AgentExecutionBoundaries) -> usize {
    boundaries
        .max_read_file_chars
        .unwrap_or(16_000)
        .clamp(400, 500_000)
}

pub(crate) fn max_write_file_chars(boundaries: &AgentExecutionBoundaries) -> usize {
    boundaries
        .max_write_file_chars
        .unwrap_or(200_000)
        .clamp(400, 2_000_000)
}

pub(crate) fn max_repo_snippet_chars(boundaries: &AgentExecutionBoundaries) -> usize {
    boundaries
        .max_file_chars
        .unwrap_or(6_000)
        .clamp(200, 80_000)
}

pub(crate) fn max_tool_steps(boundaries: &AgentExecutionBoundaries) -> usize {
    boundaries.max_tool_steps.unwrap_or(8).clamp(2, 24)
}

pub(crate) fn ignored_repository_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_product_areas"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".turbo"
            | ".idea"
            | ".vscode"
            | ".vite"
            | "coverage"
            | "test-results"
    )
}

pub(crate) fn path_has_ignored_repository_component(path: &str) -> bool {
    path.split('/').any(ignored_repository_dir)
}

pub(crate) fn truncate_for_model_context(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("\n...[truncated for model context]...");
    }
    output
}

pub(crate) fn compact_tool_observation(
    step: usize,
    tool: &str,
    reason: Option<String>,
    kind: &str,
    rendered: &str,
) -> String {
    format!(
        "{kind} step={} tool={} reason={} result={}",
        step,
        tool,
        reason.unwrap_or_default(),
        truncate_for_model_context(rendered, 4_000)
    )
}

pub(crate) fn compact_trace_payload(rendered: &str) -> String {
    truncate_for_model_context(rendered, 20_000)
}

pub(crate) fn should_keep_workspace(boundaries: &AgentExecutionBoundaries) -> bool {
    boundaries.keep_workspace.unwrap_or(true)
}

pub(crate) fn char_count(content: &str) -> usize {
    content.chars().count()
}

pub(crate) fn char_count_i64(content: &str) -> i64 {
    i64::try_from(char_count(content)).unwrap_or(i64::MAX)
}

pub(crate) fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

pub(crate) fn substring_by_char_range(
    content: &str,
    offset_chars: usize,
    length_chars: usize,
) -> String {
    content
        .chars()
        .skip(offset_chars)
        .take(length_chars)
        .collect::<String>()
}

pub(crate) fn split_lines_preserve_trailing(content: &str) -> (Vec<String>, bool) {
    let has_trailing_newline = content.ends_with('\n');
    let mut lines = content
        .split('\n')
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    if has_trailing_newline && lines.last().is_some_and(|value| value.is_empty()) {
        lines.pop();
    }
    (lines, has_trailing_newline)
}

pub(crate) fn ensure_write_limit(
    content: &str,
    boundaries: &AgentExecutionBoundaries,
) -> Result<(), AppError> {
    let chars = char_count(content);
    let limit = max_write_file_chars(boundaries);
    if chars > limit {
        return Err(AppError::Validation(format!(
            "File content exceeds max_write_file_chars ({} > {})",
            chars, limit
        )));
    }
    Ok(())
}

pub(crate) fn normalize_relative_path(path: &str) -> Option<String> {
    let normalized = path
        .replace('\\', "/")
        .trim()
        .trim_start_matches("./")
        .to_string();
    if normalized.is_empty() {
        return None;
    }
    let candidate = Path::new(&normalized);
    if candidate.is_absolute() {
        return None;
    }
    let mut cleaned: Vec<String> = Vec::new();
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(part) => cleaned.push(part.to_string_lossy().to_string()),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if cleaned.is_empty() {
        return None;
    }
    Some(cleaned.join("/"))
}

pub(crate) fn is_repo_relative_path_allowed(
    path: &str,
    boundaries: &AgentExecutionBoundaries,
) -> bool {
    let Some(normalized) = normalize_relative_path(path) else {
        return false;
    };
    if path_has_ignored_repository_component(&normalized) {
        return false;
    }
    if let Some(blocked_paths) = &boundaries.blocked_paths {
        if blocked_paths
            .iter()
            .any(|blocked| normalized.starts_with(blocked.trim_start_matches("./")))
        {
            return false;
        }
    }
    if let Some(allowed_paths) = &boundaries.allowed_paths {
        if allowed_paths.is_empty() {
            return true;
        }
        return allowed_paths
            .iter()
            .any(|allowed| normalized.starts_with(allowed.trim_start_matches("./")));
    }
    true
}

pub(crate) fn prioritize_repository_files(files: &[String]) -> Vec<String> {
    let mut scored = files
        .iter()
        .cloned()
        .map(|path| {
            let lower = path.to_ascii_lowercase();
            let score = if lower == "package.json" {
                0
            } else if lower == "readme.md" {
                1
            } else if lower == "src/app.tsx"
                || lower == "src/app.jsx"
                || lower == "src/app.ts"
                || lower == "src/app.js"
            {
                2
            } else if lower == "src/main.tsx"
                || lower == "src/main.jsx"
                || lower == "src/main.ts"
                || lower == "src/main.js"
                || lower == "src/index.tsx"
                || lower == "src/index.jsx"
                || lower == "src/index.ts"
                || lower == "src/index.js"
            {
                3
            } else if lower.starts_with("src/") {
                4
            } else if lower.ends_with(".json") || lower.ends_with(".toml") {
                5
            } else {
                6
            };
            (score, path)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, path)| path).collect()
}

pub(crate) fn is_text_source_file(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    let allowed = [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".json", ".md", ".toml", ".yaml", ".yml", ".sql",
        ".sh", ".py", ".go", ".java", ".kt", ".swift", ".c", ".h", ".cpp", ".hpp", ".css", ".scss",
        ".html", ".xml",
    ];
    allowed.iter().any(|ext| lowered.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::{
        compact_tool_observation, ensure_write_limit, is_repo_relative_path_allowed,
        substring_by_char_range, AgentExecutionBoundaries,
    };

    #[test]
    fn substring_by_char_range_respects_offset_and_length() {
        let value = "alpha-beta-gamma";
        let sliced = substring_by_char_range(value, 6, 4);
        assert_eq!(sliced, "beta");
    }

    #[test]
    fn ensure_write_limit_rejects_content_above_boundary() {
        let boundaries = AgentExecutionBoundaries {
            max_write_file_chars: Some(400),
            ..Default::default()
        };
        let oversized = "x".repeat(401);
        let result = ensure_write_limit(&oversized, &boundaries);
        assert!(result.is_err(), "expected write limit validation to fail");
    }

    #[test]
    fn generated_repository_paths_are_not_allowed() {
        let boundaries = AgentExecutionBoundaries::default();
        assert!(!is_repo_relative_path_allowed(
            "node_product_areas/react/index.js",
            &boundaries
        ));
        assert!(!is_repo_relative_path_allowed(
            "dist/assets/app.js",
            &boundaries
        ));
        assert!(is_repo_relative_path_allowed(
            "src/features/chat/pages/ChatPage.tsx",
            &boundaries
        ));
    }

    #[test]
    fn tool_observations_are_capped_for_model_context() {
        let rendered = "x".repeat(10_000);
        let observation = compact_tool_observation(
            3,
            "repo.search",
            Some("find usages".to_string()),
            "tool_result",
            &rendered,
        );
        assert!(
            observation.len() < 4_300,
            "observation should be capped before it is reused in prompts"
        );
        assert!(observation.contains("truncated for model context"));
    }
}

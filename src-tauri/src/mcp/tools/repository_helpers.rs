use crate::error::AppError;
use serde_json::{json, Value};

pub(super) fn normalize_repository_scope_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "product" => Ok("product".to_string()),
        "product_area" => Ok("product_area".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported repository scope type '{other}'. Use product or product_area."
        ))),
    }
}

fn git_status_labels(status: git2::Status) -> Vec<String> {
    let mut labels = Vec::new();
    if status.is_index_new() {
        labels.push("index_new");
    }
    if status.is_index_modified() {
        labels.push("index_modified");
    }
    if status.is_index_deleted() {
        labels.push("index_deleted");
    }
    if status.is_index_renamed() {
        labels.push("index_renamed");
    }
    if status.is_wt_new() {
        labels.push("worktree_new");
    }
    if status.is_wt_modified() {
        labels.push("worktree_modified");
    }
    if status.is_wt_deleted() {
        labels.push("worktree_deleted");
    }
    if status.is_wt_renamed() {
        labels.push("worktree_renamed");
    }
    if status.is_ignored() {
        labels.push("ignored");
    }
    labels.into_iter().map(ToString::to_string).collect()
}

pub(super) fn repository_git_status(
    local_path: &str,
    include_ignored: bool,
) -> Result<Value, AppError> {
    let repo = git2::Repository::open(local_path)?;
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|head| head.shorthand())
        .map(ToString::to_string);
    let head_sha = head
        .as_ref()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string());
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    if include_ignored {
        options.include_ignored(true);
    }
    let statuses = repo.statuses(Some(&mut options))?;
    let changed_files = statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            Some(json!({
                "path": path,
                "status": git_status_labels(entry.status())
            }))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "branch": branch,
        "headSha": head_sha,
        "dirty": !changed_files.is_empty(),
        "changedFiles": changed_files
    }))
}

pub(super) fn repository_git_diff(local_path: &str, max_bytes: i64) -> Result<Value, AppError> {
    let repo = git2::Repository::open(local_path)?;
    let mut options = git2::DiffOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let diff = repo.diff_index_to_workdir(None, Some(&mut options))?;
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' | '-' | ' ' => diff_text.push(line.origin()),
            _ => {}
        }
        diff_text.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;
    let max_bytes = max_bytes.clamp(1_024, 2_000_000) as usize;
    let truncated = diff_text.len() > max_bytes;
    if truncated {
        diff_text.truncate(max_bytes);
    }
    Ok(json!({
        "diff": diff_text,
        "truncated": truncated
    }))
}

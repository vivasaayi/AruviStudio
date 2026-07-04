use crate::services::external_cli_task_packet::ExternalCliWorkspaceSnapshot;
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExternalCliDiffSnapshot {
    captured: bool,
    status_short: String,
    unstaged_diff: String,
    staged_diff: String,
    untracked_files: Vec<String>,
    error_message: Option<String>,
}

pub(crate) async fn capture_workspace_snapshot(repo_path: &Path) -> ExternalCliWorkspaceSnapshot {
    let current_branch = run_git_capture(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .ok()
        .and_then(non_empty_string);
    let head_commit = run_git_capture(repo_path, &["rev-parse", "HEAD"])
        .await
        .ok()
        .and_then(non_empty_string);
    let status_result = run_git_capture(repo_path, &["status", "--short"]).await;
    let (status_short, status_error) = match status_result {
        Ok(output) => (output, None),
        Err(error) => (
            String::new(),
            Some(format!("Unable to capture git status: {error}")),
        ),
    };

    ExternalCliWorkspaceSnapshot {
        current_branch,
        head_commit,
        status_short,
        status_error,
    }
}

pub(crate) async fn capture_git_diff_snapshot(repo_path: &Path) -> ExternalCliDiffSnapshot {
    if let Err(error) = run_git_capture(repo_path, &["rev-parse", "--is-inside-work-tree"]).await {
        return ExternalCliDiffSnapshot {
            captured: false,
            status_short: String::new(),
            unstaged_diff: String::new(),
            staged_diff: String::new(),
            untracked_files: Vec::new(),
            error_message: Some(format!("Unable to capture git diff: {error}")),
        };
    }

    let status_short = run_git_capture(repo_path, &["status", "--short"])
        .await
        .unwrap_or_else(|error| format!("Unable to capture git status: {error}"));
    let unstaged_diff = run_git_capture(repo_path, &["diff", "--binary", "--no-ext-diff"])
        .await
        .unwrap_or_else(|error| format!("Unable to capture unstaged diff: {error}"));
    let staged_diff = run_git_capture(
        repo_path,
        &["diff", "--binary", "--no-ext-diff", "--cached"],
    )
    .await
    .unwrap_or_else(|error| format!("Unable to capture staged diff: {error}"));
    let untracked_files =
        run_git_capture(repo_path, &["ls-files", "--others", "--exclude-standard"])
            .await
            .map(|output| {
                output
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

    ExternalCliDiffSnapshot {
        captured: true,
        status_short,
        unstaged_diff,
        staged_diff,
        untracked_files,
        error_message: None,
    }
}

async fn run_git_capture(repo_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = output
            .status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "terminated".to_string());
        return Err(if stderr.is_empty() {
            format!("git exited with status {code}")
        } else {
            format!("git exited with status {code}: {stderr}")
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

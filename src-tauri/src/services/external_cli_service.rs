use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{artifact_repo, external_cli_repo, repository_repo, work_item_repo};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const EXTERNAL_CLI_TIMEOUT_SECS: u64 = 15 * 60;

#[derive(Debug, Clone, Copy)]
enum ExternalCliProvider {
    Codex,
    Claude,
    Cursor,
    Copilot,
}

impl ExternalCliProvider {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "copilot" => Ok(Self::Copilot),
            _ => Err(AppError::Validation(format!(
                "Unsupported external CLI provider: {value}"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex CLI",
            Self::Claude => "Claude Code CLI",
            Self::Cursor => "Cursor Agent CLI",
            Self::Copilot => "GitHub Copilot CLI",
        }
    }

    fn command_spec(self, prompt: &str) -> (&'static str, Vec<String>) {
        match self {
            Self::Codex => ("codex", vec!["exec".to_string(), prompt.to_string()]),
            Self::Claude => ("claude", vec!["-p".to_string(), prompt.to_string()]),
            Self::Cursor => ("cursor-agent", vec!["-p".to_string(), prompt.to_string()]),
            Self::Copilot => (
                "gh",
                vec!["copilot".to_string(), "-p".to_string(), prompt.to_string()],
            ),
        }
    }
}

#[derive(Debug, Serialize)]
struct ExternalCliRunArtifact<'a> {
    run_id: &'a str,
    provider: &'a str,
    label: &'a str,
    command: &'a str,
    args: &'a [String],
    cwd: &'a str,
    prompt: &'a str,
    status: &'a str,
    exit_code: Option<i64>,
    duration_ms: i64,
    stdout: &'a str,
    stderr: &'a str,
    error_message: Option<&'a str>,
}

pub async fn run_external_cli_for_work_item(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    provider: &str,
) -> Result<ExternalCliRun, AppError> {
    let provider = ExternalCliProvider::parse(provider)?;
    let work_item = work_item_repo::get_work_item(pool, work_item_id).await?;
    let repository = repository_repo::resolve_repository_for_work_item(pool, work_item_id)
        .await?
        .ok_or_else(|| {
            AppError::Validation("Attach a workspace before invoking an external CLI.".to_string())
        })?;
    let prompt = build_external_cli_prompt(&work_item);
    let (command, args) = provider.command_spec(&prompt);
    let invocation = ExternalCliInvocation {
        work_item_id: work_item_id.to_string(),
        provider: provider.as_str().to_string(),
        label: provider.label().to_string(),
        command: command.to_string(),
        args,
        prompt,
        cwd: repository.local_path,
    };
    let run_id = Uuid::new_v4().to_string();
    external_cli_repo::create_external_cli_run(pool, &run_id, &invocation).await?;

    let started = Instant::now();
    let execution = timeout(
        Duration::from_secs(EXTERNAL_CLI_TIMEOUT_SECS),
        Command::new(&invocation.command)
            .args(&invocation.args)
            .current_dir(&invocation.cwd)
            .output(),
    )
    .await;
    let duration_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);

    let (status, exit_code, stdout, stderr, error_message) = match execution {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().map(i64::from);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let status = if output.status.success() {
                "completed"
            } else {
                "failed"
            };
            let error_message = if output.status.success() {
                None
            } else {
                Some(format!(
                    "{} exited with status {}",
                    invocation.label,
                    exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "terminated".to_string())
                ))
            };
            (status.to_string(), exit_code, stdout, stderr, error_message)
        }
        Ok(Err(error)) => (
            "failed".to_string(),
            None,
            String::new(),
            String::new(),
            Some(format!("Failed to launch {}: {}", invocation.label, error)),
        ),
        Err(_) => (
            "failed".to_string(),
            None,
            String::new(),
            String::new(),
            Some(format!(
                "{} timed out after {} seconds",
                invocation.label, EXTERNAL_CLI_TIMEOUT_SECS
            )),
        ),
    };

    let artifact_id = store_external_cli_artifact(
        pool,
        artifact_base_path,
        work_item_id,
        &run_id,
        &invocation,
        &status,
        exit_code,
        duration_ms,
        &stdout,
        &stderr,
        error_message.as_deref(),
    )
    .await?;

    external_cli_repo::complete_external_cli_run(
        pool,
        &run_id,
        &status,
        exit_code,
        duration_ms,
        i64::try_from(stdout.chars().count()).unwrap_or(i64::MAX),
        i64::try_from(stderr.chars().count()).unwrap_or(i64::MAX),
        Some(&artifact_id),
        error_message.as_deref(),
    )
    .await
}

fn build_external_cli_prompt(work_item: &WorkItem) -> String {
    format!(
        "You are assisting AruviStudio with this approved work item.\n\nWork item ID: {}\nTitle: {}\nType: {}\nPriority: {}\nComplexity: {}\n\nProblem statement:\n{}\n\nDescription:\n{}\n\nAcceptance criteria:\n{}\n\nConstraints:\n{}\n\nWork in the current repository. Keep the change scoped to this work item. Report what you changed, commands you ran, and any blockers.",
        work_item.id,
        work_item.title,
        work_item.work_item_type,
        work_item.priority,
        work_item.complexity,
        empty_as_not_provided(&work_item.problem_statement),
        empty_as_not_provided(&work_item.description),
        empty_as_not_provided(&work_item.acceptance_criteria),
        empty_as_not_provided(&work_item.constraints),
    )
}

fn empty_as_not_provided(value: &str) -> &str {
    if value.trim().is_empty() {
        "Not provided."
    } else {
        value
    }
}

async fn store_external_cli_artifact(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    run_id: &str,
    invocation: &ExternalCliInvocation,
    status: &str,
    exit_code: Option<i64>,
    duration_ms: i64,
    stdout: &str,
    stderr: &str,
    error_message: Option<&str>,
) -> Result<String, AppError> {
    let run_dir = artifact_base_path.join("external_cli_runs").join(run_id);
    tokio::fs::create_dir_all(&run_dir).await?;
    let artifact_path = run_dir.join("run.json");
    let payload = ExternalCliRunArtifact {
        run_id,
        provider: &invocation.provider,
        label: &invocation.label,
        command: &invocation.command,
        args: &invocation.args,
        cwd: &invocation.cwd,
        prompt: &invocation.prompt,
        status,
        exit_code,
        duration_ms,
        stdout,
        stderr,
        error_message,
    };
    let content = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(&artifact_path, content).await?;
    let artifact_id = Uuid::new_v4().to_string();
    artifact_repo::create_artifact(
        pool,
        &artifact_id,
        work_item_id,
        None,
        None,
        "external_cli_run",
        &format!("{} {}", invocation.label, status),
        artifact_path.to_string_lossy().as_ref(),
    )
    .await?;
    Ok(artifact_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::product::{HierarchyNodeType, Priority};
    use crate::domain::work_item::{Complexity, WorkItemStatus, WorkItemType};

    fn sample_work_item() -> WorkItem {
        WorkItem {
            id: "wi-1".to_string(),
            product_id: Some("product-1".to_string()),
            module_id: Some("module-1".to_string()),
            capability_id: None,
            source_node_id: Some("module-1".to_string()),
            source_node_type: Some(HierarchyNodeType::Module),
            parent_work_item_id: None,
            title: "Add external CLI launch buttons".to_string(),
            problem_statement: "A user already pays for another coding assistant.".to_string(),
            description: "Let the user invoke that assistant from AruviStudio.".to_string(),
            acceptance_criteria: "The run is tracked with output and status.".to_string(),
            constraints: "".to_string(),
            work_item_type: WorkItemType::CapabilityDelivery,
            priority: Priority::High,
            complexity: Complexity::Medium,
            status: WorkItemStatus::Approved,
            repo_override_id: None,
            active_repo_id: None,
            branch_name: Some("work/external-cli".to_string()),
            sort_order: 0,
            created_at: "2026-06-10 00:00:00".to_string(),
            updated_at: "2026-06-10 00:00:00".to_string(),
        }
    }

    #[test]
    fn builds_work_item_prompt_with_missing_fields_marked() {
        let prompt = build_external_cli_prompt(&sample_work_item());

        assert!(prompt.contains("Work item ID: wi-1"));
        assert!(prompt.contains("Title: Add external CLI launch buttons"));
        assert!(prompt.contains("A user already pays for another coding assistant."));
        assert!(prompt.contains("The run is tracked with output and status."));
        assert!(prompt.contains("Constraints:\nNot provided."));
        assert!(prompt.contains("Report what you changed, commands you ran, and any blockers."));
    }

    #[test]
    fn resolves_supported_cli_command_specs() {
        let prompt = "Implement the work item";

        let (codex_command, codex_args) = ExternalCliProvider::Codex.command_spec(prompt);
        assert_eq!(codex_command, "codex");
        assert_eq!(codex_args, vec!["exec", prompt]);

        let (claude_command, claude_args) = ExternalCliProvider::Claude.command_spec(prompt);
        assert_eq!(claude_command, "claude");
        assert_eq!(claude_args, vec!["-p", prompt]);

        let (cursor_command, cursor_args) = ExternalCliProvider::Cursor.command_spec(prompt);
        assert_eq!(cursor_command, "cursor-agent");
        assert_eq!(cursor_args, vec!["-p", prompt]);

        let (copilot_command, copilot_args) = ExternalCliProvider::Copilot.command_spec(prompt);
        assert_eq!(copilot_command, "gh");
        assert_eq!(copilot_args, vec!["copilot", "-p", prompt]);
    }

    #[test]
    fn rejects_unknown_cli_provider() {
        let error = ExternalCliProvider::parse("unknown").expect_err("provider should fail");
        assert!(error
            .to_string()
            .contains("Unsupported external CLI provider"));
    }
}

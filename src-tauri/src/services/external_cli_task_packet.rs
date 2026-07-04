use crate::domain::repository::Repository;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::services::external_cli_provider::ExternalCliProvider;
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExternalCliTaskPacket {
    packet_version: String,
    source: String,
    provider: String,
    generated_at: String,
    repository: ExternalCliRepositoryPacket,
    work_item: ExternalCliWorkItemPacket,
    task_instructions: Vec<String>,
    review_checkpoint: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExternalCliRepositoryPacket {
    id: String,
    name: String,
    path: String,
    remote_url: String,
    default_branch: String,
    requested_branch: String,
    current_branch: Option<String>,
    head_commit: Option<String>,
    workspace_status: String,
    workspace_status_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ExternalCliWorkItemPacket {
    id: String,
    title: String,
    work_item_type: String,
    priority: String,
    complexity: String,
    status: String,
    problem_statement: String,
    description: String,
    acceptance_criteria: String,
    constraints: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExternalCliWorkspaceSnapshot {
    pub(crate) current_branch: Option<String>,
    pub(crate) head_commit: Option<String>,
    pub(crate) status_short: String,
    pub(crate) status_error: Option<String>,
}

pub(crate) fn build_external_cli_task_packet(
    provider: ExternalCliProvider,
    work_item: &WorkItem,
    repository: &Repository,
    workspace_snapshot: ExternalCliWorkspaceSnapshot,
) -> ExternalCliTaskPacket {
    let requested_branch = work_item
        .branch_name
        .clone()
        .unwrap_or_else(|| repository.default_branch.clone());

    ExternalCliTaskPacket {
        packet_version: "external-cli-task/v1".to_string(),
        source: "Aruvi Studio".to_string(),
        provider: provider.as_str().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        repository: ExternalCliRepositoryPacket {
            id: repository.id.clone(),
            name: repository.name.clone(),
            path: repository.local_path.clone(),
            remote_url: empty_as_not_provided(&repository.remote_url),
            default_branch: repository.default_branch.clone(),
            requested_branch,
            current_branch: workspace_snapshot.current_branch,
            head_commit: workspace_snapshot.head_commit,
            workspace_status: workspace_snapshot.status_short,
            workspace_status_error: workspace_snapshot.status_error,
        },
        work_item: ExternalCliWorkItemPacket {
            id: work_item.id.clone(),
            title: work_item.title.clone(),
            work_item_type: work_item.work_item_type.to_string(),
            priority: work_item.priority.to_string(),
            complexity: work_item.complexity.to_string(),
            status: work_item.status.to_string(),
            problem_statement: empty_as_not_provided(&work_item.problem_statement),
            description: empty_as_not_provided(&work_item.description),
            acceptance_criteria: empty_as_not_provided(&work_item.acceptance_criteria),
            constraints: empty_as_not_provided(&work_item.constraints),
        },
        task_instructions: vec![
            "Implement the approved work item in the current repository workspace.".to_string(),
            "Keep changes scoped to the work item and its acceptance criteria.".to_string(),
            "Respect all listed constraints; if a constraint cannot be met, report it as a blocker."
                .to_string(),
            "Run relevant validation commands when practical and report every command run."
                .to_string(),
            "Do not mark the work item complete, push commits, or bypass Aruvi review checkpoints."
                .to_string(),
        ],
        review_checkpoint:
            "Return implementation output to Aruvi. Aruvi records the run and routes successful output to pending_test_review/waiting_human_review before completion."
                .to_string(),
    }
}

pub(crate) fn build_external_cli_prompt(
    task_packet: &ExternalCliTaskPacket,
) -> Result<String, AppError> {
    let packet_json = serde_json::to_string_pretty(task_packet)?;
    Ok(format!(
        "You are assisting AruviStudio with an approved implementation work item.\n\nAruvi is the planner, context source, workflow coordinator, and review system. Use the task packet below as the source of truth, implement in the current repository, and report what changed, commands run, validation results, and blockers.\n\nTask packet:\n```json\n{}\n```",
        packet_json
    ))
}

fn empty_as_not_provided(value: &str) -> String {
    if value.trim().is_empty() {
        "Not provided.".to_string()
    } else {
        value.to_string()
    }
}

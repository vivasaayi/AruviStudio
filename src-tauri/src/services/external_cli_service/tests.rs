use super::*;
use crate::domain::product::{HierarchyNodeType, Priority};
use crate::domain::work_item::{Complexity, WorkItemStatus, WorkItemType};
use crate::services::external_cli_events::{
    append_external_cli_session_log, summarize_external_cli_failure,
};
use crate::services::external_cli_task_packet::ExternalCliWorkspaceSnapshot;
use uuid::Uuid;

fn sample_work_item() -> WorkItem {
    WorkItem {
        id: "wi-1".to_string(),
        product_id: Some("product-1".to_string()),
        product_area_id: Some("product_area-1".to_string()),
        capability_id: None,
        source_node_id: Some("product_area-1".to_string()),
        source_node_type: Some(HierarchyNodeType::ProductArea),
        parent_work_item_id: None,
        title: "Add external CLI launch buttons".to_string(),
        problem_statement: "A user already pays for another coding assistant.".to_string(),
        description: "Let the user invoke that assistant from AruviStudio.".to_string(),
        acceptance_criteria: "The run is tracked with output and status.".to_string(),
        constraints: "".to_string(),
        work_item_type: WorkItemType::Story,
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

fn sample_repository() -> Repository {
    Repository {
        id: "repo-1".to_string(),
        name: "Aruvi Studio".to_string(),
        local_path: "/tmp/aruvi".to_string(),
        remote_url: String::new(),
        default_branch: "main".to_string(),
        auth_profile: None,
        created_at: "2026-06-10 00:00:00".to_string(),
        updated_at: "2026-06-10 00:00:00".to_string(),
    }
}

#[test]
fn builds_work_item_prompt_with_missing_fields_marked() {
    let task_packet = build_external_cli_task_packet(
        ExternalCliProvider::Codex,
        &sample_work_item(),
        &sample_repository(),
        ExternalCliWorkspaceSnapshot {
            current_branch: Some("main".to_string()),
            head_commit: Some("abc123".to_string()),
            status_short: String::new(),
            status_error: None,
        },
    );
    let prompt = build_external_cli_prompt(&task_packet).expect("prompt should serialize");

    assert!(prompt.contains("\"id\": \"wi-1\""));
    assert!(prompt.contains("\"title\": \"Add external CLI launch buttons\""));
    assert!(prompt.contains("A user already pays for another coding assistant."));
    assert!(prompt.contains("The run is tracked with output and status."));
    assert!(prompt.contains("\"constraints\": \"Not provided.\""));
    assert!(prompt.contains("report what changed, commands run, validation results, and blockers"));
}

#[test]
fn resolves_supported_cli_command_specs() {
    let prompt = "Implement the work item";
    let cwd = "/tmp/aruvi";

    let (codex_command, codex_args) = ExternalCliProvider::Codex.command_spec(prompt, cwd);
    assert_eq!(codex_command, "codex");
    assert_eq!(
        codex_args,
        vec![
            "exec",
            "--ignore-user-config",
            "--sandbox",
            "workspace-write",
            "--cd",
            cwd,
            prompt,
        ]
    );

    let (claude_command, claude_args) = ExternalCliProvider::Claude.command_spec(prompt, cwd);
    assert_eq!(claude_command, "claude");
    assert_eq!(claude_args, vec!["-p", prompt]);

    let (cursor_command, cursor_args) = ExternalCliProvider::Cursor.command_spec(prompt, cwd);
    assert_eq!(cursor_command, "cursor-agent");
    assert_eq!(cursor_args, vec!["-p", prompt]);

    let (copilot_command, copilot_args) = ExternalCliProvider::Copilot.command_spec(prompt, cwd);
    assert_eq!(copilot_command, "copilot");
    assert!(copilot_args.contains(&"--autopilot".to_string()));
    assert!(copilot_args.contains(&"--no-ask-user".to_string()));
    assert_eq!(copilot_args.last(), Some(&prompt.to_string()));
}

#[test]
fn rejects_unknown_cli_provider() {
    let error = ExternalCliProvider::parse("unknown").expect_err("provider should fail");
    assert!(error
        .to_string()
        .contains("Unsupported external CLI provider"));
}

#[test]
fn summarizes_external_cli_failure_from_process_error() {
    let summary = summarize_external_cli_failure(
        "GitHub Copilot CLI",
        None,
        Some("Failed to launch GitHub Copilot CLI: No such file or directory"),
        "",
        "",
    );

    assert_eq!(
        summary,
        "Failed to launch GitHub Copilot CLI: No such file or directory"
    );
}

#[tokio::test]
async fn appends_external_cli_session_log_file() {
    let path =
        std::env::temp_dir().join(format!("aruvi-external-cli-session-{}.log", Uuid::new_v4()));
    let path_string = path.to_string_lossy().to_string();

    append_external_cli_session_log(&path_string, 1, "lifecycle", "Starting Codex CLI.")
        .await
        .expect("first session log append should succeed");
    append_external_cli_session_log(&path_string, 2, "stderr", "network disconnected")
        .await
        .expect("second session log append should succeed");

    let content = tokio::fs::read_to_string(&path)
        .await
        .expect("session log should be readable");
    assert!(content.contains("#1 INFO: Starting Codex CLI."));
    assert!(content.contains("#2 STDERR: network disconnected"));

    let _ = tokio::fs::remove_file(path).await;
}

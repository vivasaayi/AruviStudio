use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun};
use crate::error::AppError;
use crate::persistence::{artifact_repo, external_cli_repo, workflow_repo};
use crate::services::external_cli_events::append_external_cli_event;
use crate::services::external_cli_task_packet::ExternalCliTaskPacket;
use crate::services::external_cli_workspace::{capture_git_diff_snapshot, ExternalCliDiffSnapshot};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::{atomic::AtomicI64, Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
struct ExternalCliReviewCheckpoint {
    work_item_status: String,
    workflow_run_id: String,
    workflow_stage: String,
    notes: String,
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
    task_packet: &'a ExternalCliTaskPacket,
    status: &'a str,
    exit_code: Option<i64>,
    started_at: &'a str,
    ended_at: &'a str,
    duration_ms: i64,
    stdout: &'a str,
    stderr: &'a str,
    session_log_path: &'a str,
    diff_snapshot: &'a ExternalCliDiffSnapshot,
    review_checkpoint: Option<&'a ExternalCliReviewCheckpoint>,
    error_message: Option<&'a str>,
}

pub(crate) struct ExternalCliRunFinalization<'a> {
    pub(crate) artifact_base_path: &'a Path,
    pub(crate) run_id: &'a str,
    pub(crate) invocation: &'a ExternalCliInvocation,
    pub(crate) task_packet: &'a ExternalCliTaskPacket,
    pub(crate) status: &'a str,
    pub(crate) exit_code: Option<i64>,
    pub(crate) started_at: &'a str,
    pub(crate) ended_at: String,
    pub(crate) duration_ms: i64,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) error_message: Option<String>,
    pub(crate) sequence: &'a Arc<AtomicI64>,
}

pub(crate) async fn finalize_external_cli_run(
    pool: &SqlitePool,
    input: ExternalCliRunFinalization<'_>,
) -> Result<ExternalCliRun, AppError> {
    let diff_snapshot = capture_git_diff_snapshot(Path::new(&input.invocation.cwd)).await;
    append_external_cli_event(
        pool,
        input.run_id,
        &input.invocation.work_item_id,
        &input.invocation.session_log_path,
        "lifecycle",
        "Captured git diff snapshot.".to_string(),
        input.sequence,
    )
    .await?;
    let mut completion_error_message = input.error_message;
    let review_checkpoint = if input.status == "completed" {
        match record_successful_external_cli_review_checkpoint(
            pool,
            &input.invocation.work_item_id,
            input.run_id,
            &input.invocation.label,
        )
        .await
        {
            Ok(checkpoint) => Some(checkpoint),
            Err(error) => {
                let message = format!("Aruvi review checkpoint failed: {error}");
                append_external_cli_event(
                    pool,
                    input.run_id,
                    &input.invocation.work_item_id,
                    &input.invocation.session_log_path,
                    "error",
                    message.clone(),
                    input.sequence,
                )
                .await?;
                completion_error_message = Some(match completion_error_message {
                    Some(existing) => format!("{existing}; {message}"),
                    None => message,
                });
                None
            }
        }
    } else {
        None
    };
    if review_checkpoint.is_some() {
        append_external_cli_event(
            pool,
            input.run_id,
            &input.invocation.work_item_id,
            &input.invocation.session_log_path,
            "lifecycle",
            "Moved output to the Aruvi review checkpoint.".to_string(),
            input.sequence,
        )
        .await?;
    }

    let artifact_id = store_external_cli_artifact(
        pool,
        StoreExternalCliArtifactInput {
            artifact_base_path: input.artifact_base_path,
            work_item_id: &input.invocation.work_item_id,
            run_id: input.run_id,
            invocation: input.invocation,
            task_packet: input.task_packet,
            status: input.status,
            exit_code: input.exit_code,
            started_at: input.started_at,
            ended_at: &input.ended_at,
            duration_ms: input.duration_ms,
            stdout: &input.stdout,
            stderr: &input.stderr,
            diff_snapshot: &diff_snapshot,
            review_checkpoint: review_checkpoint.as_ref(),
            error_message: completion_error_message.as_deref(),
        },
    )
    .await?;
    append_external_cli_event(
        pool,
        input.run_id,
        &input.invocation.work_item_id,
        &input.invocation.session_log_path,
        "lifecycle",
        format!("Captured output artifact: {artifact_id}."),
        input.sequence,
    )
    .await?;

    external_cli_repo::complete_external_cli_run(
        pool,
        external_cli_repo::CompleteExternalCliRunInput {
            id: input.run_id,
            status: input.status,
            exit_code: input.exit_code,
            duration_ms: input.duration_ms,
            stdout_chars: i64::try_from(input.stdout.chars().count()).unwrap_or(i64::MAX),
            stderr_chars: i64::try_from(input.stderr.chars().count()).unwrap_or(i64::MAX),
            output_artifact_id: Some(&artifact_id),
            error_message: completion_error_message.as_deref(),
        },
    )
    .await
}

async fn record_successful_external_cli_review_checkpoint(
    pool: &SqlitePool,
    work_item_id: &str,
    run_id: &str,
    label: &str,
) -> Result<ExternalCliReviewCheckpoint, AppError> {
    sqlx::query("UPDATE work_items SET status='waiting_human_review', updated_at=datetime('now') WHERE id=? AND status <> 'done'")
        .bind(work_item_id)
        .execute(pool)
        .await?;

    let workflow_run =
        match workflow_repo::find_active_workflow_for_work_item(pool, work_item_id).await? {
            Some(run) => run,
            None => {
                let workflow_run_id = Uuid::new_v4().to_string();
                workflow_repo::create_workflow_run(pool, &workflow_run_id, work_item_id).await?
            }
        };

    if workflow_run.current_stage != "pending_test_review" {
        workflow_repo::update_workflow_stage(pool, &workflow_run.id, "pending_test_review").await?;
        workflow_repo::update_workflow_lifecycle(pool, &workflow_run.id, "running", None, false)
            .await?;
        workflow_repo::record_stage_transition(
            pool,
            &Uuid::new_v4().to_string(),
            &workflow_run.id,
            &workflow_run.current_stage,
            "pending_test_review",
            "external_cli_completion",
            &format!("{label} run {run_id} completed; awaiting Aruvi review checkpoint"),
        )
        .await?;
    }

    Ok(ExternalCliReviewCheckpoint {
        work_item_status: "waiting_human_review".to_string(),
        workflow_run_id: workflow_run.id,
        workflow_stage: "pending_test_review".to_string(),
        notes: format!("{label} output is ready for Aruvi review before completion."),
    })
}

struct StoreExternalCliArtifactInput<'a> {
    artifact_base_path: &'a Path,
    work_item_id: &'a str,
    run_id: &'a str,
    invocation: &'a ExternalCliInvocation,
    task_packet: &'a ExternalCliTaskPacket,
    status: &'a str,
    exit_code: Option<i64>,
    started_at: &'a str,
    ended_at: &'a str,
    duration_ms: i64,
    stdout: &'a str,
    stderr: &'a str,
    diff_snapshot: &'a ExternalCliDiffSnapshot,
    review_checkpoint: Option<&'a ExternalCliReviewCheckpoint>,
    error_message: Option<&'a str>,
}

async fn store_external_cli_artifact(
    pool: &SqlitePool,
    input: StoreExternalCliArtifactInput<'_>,
) -> Result<String, AppError> {
    let run_dir = input
        .artifact_base_path
        .join("external_cli_runs")
        .join(input.run_id);
    tokio::fs::create_dir_all(&run_dir).await?;
    let artifact_path = run_dir.join("run.json");
    let payload = ExternalCliRunArtifact {
        run_id: input.run_id,
        provider: &input.invocation.provider,
        label: &input.invocation.label,
        command: &input.invocation.command,
        args: &input.invocation.args,
        cwd: &input.invocation.cwd,
        prompt: &input.invocation.prompt,
        task_packet: input.task_packet,
        status: input.status,
        exit_code: input.exit_code,
        started_at: input.started_at,
        ended_at: input.ended_at,
        duration_ms: input.duration_ms,
        stdout: input.stdout,
        stderr: input.stderr,
        session_log_path: &input.invocation.session_log_path,
        diff_snapshot: input.diff_snapshot,
        review_checkpoint: input.review_checkpoint,
        error_message: input.error_message,
    };
    let content = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(&artifact_path, content).await?;
    let artifact_id = Uuid::new_v4().to_string();
    artifact_repo::create_artifact(
        pool,
        artifact_repo::CreateArtifactInput {
            id: &artifact_id,
            work_item_id: input.work_item_id,
            workflow_run_id: None,
            agent_run_id: None,
            artifact_type: "external_cli_run",
            summary: &format!("{} {}", input.invocation.label, input.status),
            storage_path: artifact_path.to_string_lossy().as_ref(),
        },
    )
    .await?;
    Ok(artifact_id)
}

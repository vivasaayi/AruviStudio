use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun};
use crate::domain::repository::Repository;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{
    artifact_repo, external_cli_repo, repository_repo, work_item_repo, workflow_repo,
};
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const EXTERNAL_CLI_TIMEOUT_SECS: u64 = 15 * 60;
const EXTERNAL_CLI_EVENT_MESSAGE_MAX_CHARS: usize = 16_000;

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

    fn command_spec(self, prompt: &str, cwd: &str) -> (&'static str, Vec<String>) {
        match self {
            Self::Codex => (
                "codex",
                vec![
                    "exec".to_string(),
                    "--ignore-user-config".to_string(),
                    "--sandbox".to_string(),
                    "workspace-write".to_string(),
                    "--cd".to_string(),
                    cwd.to_string(),
                    prompt.to_string(),
                ],
            ),
            Self::Claude => ("claude", vec!["-p".to_string(), prompt.to_string()]),
            Self::Cursor => ("cursor-agent", vec!["-p".to_string(), prompt.to_string()]),
            Self::Copilot => (
                "copilot",
                vec![
                    "--autopilot".to_string(),
                    "--no-ask-user".to_string(),
                    "--max-autopilot-continues".to_string(),
                    "10".to_string(),
                    "-s".to_string(),
                    "--allow-tool".to_string(),
                    "shell(git:*),shell(npm:*),shell(cargo:*),shell(rg:*),shell(ls:*),shell(cat:*),shell(sed:*),write".to_string(),
                    "-p".to_string(),
                    prompt.to_string(),
                ],
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ExternalCliTaskPacket {
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
struct ExternalCliWorkspaceSnapshot {
    current_branch: Option<String>,
    head_commit: Option<String>,
    status_short: String,
    status_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ExternalCliDiffSnapshot {
    captured: bool,
    status_short: String,
    unstaged_diff: String,
    staged_diff: String,
    untracked_files: Vec<String>,
    error_message: Option<String>,
}

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

pub async fn run_external_cli_for_work_item(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    provider: &str,
) -> Result<ExternalCliRun, AppError> {
    let provider = ExternalCliProvider::parse(provider)?;
    let work_item = work_item_repo::get_work_item(pool, work_item_id).await?;
    let repository = ensure_repository_ready_for_external_cli(pool, &work_item).await?;
    let workspace_snapshot = capture_workspace_snapshot(Path::new(&repository.local_path)).await;
    let task_packet =
        build_external_cli_task_packet(provider, &work_item, &repository, workspace_snapshot);
    let prompt = build_external_cli_prompt(&task_packet)?;
    let run_id = Uuid::new_v4().to_string();
    let run_dir = artifact_base_path.join("external_cli_runs").join(&run_id);
    tokio::fs::create_dir_all(&run_dir).await?;
    let session_log_path = run_dir.join("session.log").to_string_lossy().to_string();
    let (command, args) = provider.command_spec(&prompt, &repository.local_path);
    let invocation = ExternalCliInvocation {
        work_item_id: work_item_id.to_string(),
        provider: provider.as_str().to_string(),
        label: provider.label().to_string(),
        command: command.to_string(),
        args,
        prompt,
        cwd: repository.local_path,
        session_log_path,
    };
    let run = external_cli_repo::create_external_cli_run(pool, &run_id, &invocation).await?;

    let execution_pool = pool.clone();
    let execution_artifact_base_path = artifact_base_path.to_path_buf();
    let execution_run_id = run_id.clone();
    tokio::spawn(async move {
        let fallback_pool = execution_pool.clone();
        let fallback_run_id = execution_run_id.clone();
        if let Err(error) = execute_external_cli_run(
            execution_pool,
            execution_artifact_base_path,
            execution_run_id,
            invocation,
            task_packet,
        )
        .await
        {
            let failure_message = format!("External CLI tracking failed: {error}");
            let _ = external_cli_repo::complete_external_cli_run(
                &fallback_pool,
                &fallback_run_id,
                "failed",
                None,
                0,
                0,
                0,
                None,
                Some(&failure_message),
            )
            .await;
            eprintln!("external CLI execution failed: {error}");
        }
    });

    Ok(run)
}

async fn execute_external_cli_run(
    pool: SqlitePool,
    artifact_base_path: PathBuf,
    run_id: String,
    invocation: ExternalCliInvocation,
    task_packet: ExternalCliTaskPacket,
) -> Result<(), AppError> {
    let started_at = Utc::now().to_rfc3339();
    let started = Instant::now();
    let sequence = Arc::new(AtomicI64::new(0));

    append_external_cli_event(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        "lifecycle",
        format!("Starting {}.", invocation.label),
        &sequence,
    )
    .await?;
    append_external_cli_event(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        "lifecycle",
        format!("Command: {}", format_invocation_for_event(&invocation)),
        &sequence,
    )
    .await?;
    append_external_cli_event(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        "lifecycle",
        format!("Working directory: {}", invocation.cwd),
        &sequence,
    )
    .await?;

    let mut child = match Command::new(&invocation.command)
        .args(&invocation.args)
        .current_dir(&invocation.cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            append_external_cli_event(
                &pool,
                &run_id,
                &invocation.work_item_id,
                &invocation.session_log_path,
                "lifecycle",
                "Process spawned.".to_string(),
                &sequence,
            )
            .await?;
            child
        }
        Err(error) => {
            let error_message = format!("Failed to launch {}: {}", invocation.label, error);
            append_external_cli_event(
                &pool,
                &run_id,
                &invocation.work_item_id,
                &invocation.session_log_path,
                "error",
                error_message.clone(),
                &sequence,
            )
            .await?;
            finalize_external_cli_run(
                &pool,
                &artifact_base_path,
                &run_id,
                &invocation,
                &task_packet,
                "failed",
                None,
                &started_at,
                Utc::now().to_rfc3339(),
                i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
                String::new(),
                String::new(),
                Some(error_message),
                &sequence,
            )
            .await?;
            return Ok(());
        }
    };

    let stdout = Arc::new(Mutex::new(String::new()));
    let stderr = Arc::new(Mutex::new(String::new()));
    let stdout_task = child.stdout.take().map(|reader| {
        spawn_output_reader(
            pool.clone(),
            run_id.clone(),
            invocation.work_item_id.clone(),
            invocation.session_log_path.clone(),
            "stdout",
            reader,
            stdout.clone(),
            sequence.clone(),
        )
    });
    let stderr_task = child.stderr.take().map(|reader| {
        spawn_output_reader(
            pool.clone(),
            run_id.clone(),
            invocation.work_item_id.clone(),
            invocation.session_log_path.clone(),
            "stderr",
            reader,
            stderr.clone(),
            sequence.clone(),
        )
    });

    let (status, exit_code, mut error_message) =
        match timeout(Duration::from_secs(EXTERNAL_CLI_TIMEOUT_SECS), child.wait()).await {
            Ok(Ok(exit_status)) => {
                let exit_code = exit_status.code().map(i64::from);
                if exit_status.success() {
                    append_external_cli_event(
                        &pool,
                        &run_id,
                        &invocation.work_item_id,
                        &invocation.session_log_path,
                        "lifecycle",
                        "Process completed successfully.".to_string(),
                        &sequence,
                    )
                    .await?;
                    ("completed".to_string(), exit_code, None)
                } else {
                    let error_message = format!(
                        "{} exited with status {}",
                        invocation.label,
                        exit_code
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "terminated".to_string())
                    );
                    append_external_cli_event(
                        &pool,
                        &run_id,
                        &invocation.work_item_id,
                        &invocation.session_log_path,
                        "error",
                        error_message.clone(),
                        &sequence,
                    )
                    .await?;
                    ("failed".to_string(), exit_code, Some(error_message))
                }
            }
            Ok(Err(error)) => {
                let error_message = format!("Failed to wait for {}: {}", invocation.label, error);
                append_external_cli_event(
                    &pool,
                    &run_id,
                    &invocation.work_item_id,
                    &invocation.session_log_path,
                    "error",
                    error_message.clone(),
                    &sequence,
                )
                .await?;
                ("failed".to_string(), None, Some(error_message))
            }
            Err(_) => {
                let _ = child.kill().await;
                let error_message = format!(
                    "{} timed out after {} seconds",
                    invocation.label, EXTERNAL_CLI_TIMEOUT_SECS
                );
                append_external_cli_event(
                    &pool,
                    &run_id,
                    &invocation.work_item_id,
                    &invocation.session_log_path,
                    "error",
                    error_message.clone(),
                    &sequence,
                )
                .await?;
                ("failed".to_string(), None, Some(error_message))
            }
        };

    join_output_reader(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        stdout_task,
        &sequence,
    )
    .await?;
    join_output_reader(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        stderr_task,
        &sequence,
    )
    .await?;

    let ended_at = Utc::now().to_rfc3339();
    let duration_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let stdout = stdout.lock().await.clone();
    let stderr = stderr.lock().await.clone();
    if status != "completed" {
        let summary = summarize_external_cli_failure(
            &invocation.label,
            exit_code,
            error_message.as_deref(),
            &stdout,
            &stderr,
        );
        append_external_cli_event(
            &pool,
            &run_id,
            &invocation.work_item_id,
            &invocation.session_log_path,
            "error",
            format!("Failure summary: {summary}"),
            &sequence,
        )
        .await?;
        error_message = Some(summary);
    }
    finalize_external_cli_run(
        &pool,
        &artifact_base_path,
        &run_id,
        &invocation,
        &task_packet,
        &status,
        exit_code,
        &started_at,
        ended_at,
        duration_ms,
        stdout,
        stderr,
        error_message,
        &sequence,
    )
    .await?;

    Ok(())
}

async fn finalize_external_cli_run(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    run_id: &str,
    invocation: &ExternalCliInvocation,
    task_packet: &ExternalCliTaskPacket,
    status: &str,
    exit_code: Option<i64>,
    started_at: &str,
    ended_at: String,
    duration_ms: i64,
    stdout: String,
    stderr: String,
    error_message: Option<String>,
    sequence: &Arc<AtomicI64>,
) -> Result<ExternalCliRun, AppError> {
    let diff_snapshot = capture_git_diff_snapshot(Path::new(&invocation.cwd)).await;
    append_external_cli_event(
        pool,
        run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        "lifecycle",
        "Captured git diff snapshot.".to_string(),
        sequence,
    )
    .await?;
    let mut completion_error_message = error_message;
    let review_checkpoint = if status == "completed" {
        match record_successful_external_cli_review_checkpoint(
            pool,
            &invocation.work_item_id,
            run_id,
            &invocation.label,
        )
        .await
        {
            Ok(checkpoint) => Some(checkpoint),
            Err(error) => {
                let message = format!("Aruvi review checkpoint failed: {error}");
                append_external_cli_event(
                    pool,
                    run_id,
                    &invocation.work_item_id,
                    &invocation.session_log_path,
                    "error",
                    message.clone(),
                    sequence,
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
            run_id,
            &invocation.work_item_id,
            &invocation.session_log_path,
            "lifecycle",
            "Moved output to the Aruvi review checkpoint.".to_string(),
            sequence,
        )
        .await?;
    }

    let artifact_id = store_external_cli_artifact(
        pool,
        artifact_base_path,
        &invocation.work_item_id,
        run_id,
        invocation,
        task_packet,
        status,
        exit_code,
        started_at,
        &ended_at,
        duration_ms,
        &stdout,
        &stderr,
        &diff_snapshot,
        review_checkpoint.as_ref(),
        completion_error_message.as_deref(),
    )
    .await?;
    append_external_cli_event(
        pool,
        run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        "lifecycle",
        format!("Captured output artifact: {artifact_id}."),
        sequence,
    )
    .await?;

    external_cli_repo::complete_external_cli_run(
        pool,
        run_id,
        status,
        exit_code,
        duration_ms,
        i64::try_from(stdout.chars().count()).unwrap_or(i64::MAX),
        i64::try_from(stderr.chars().count()).unwrap_or(i64::MAX),
        Some(&artifact_id),
        completion_error_message.as_deref(),
    )
    .await
}

fn spawn_output_reader<R>(
    pool: SqlitePool,
    run_id: String,
    work_item_id: String,
    session_log_path: String,
    stream: &'static str,
    reader: R,
    capture: Arc<Mutex<String>>,
    sequence: Arc<AtomicI64>,
) -> JoinHandle<Result<(), AppError>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let bytes_read = reader.read_until(b'\n', &mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            let chunk = String::from_utf8_lossy(&buffer).to_string();
            {
                let mut output = capture.lock().await;
                output.push_str(&chunk);
            }
            let message = chunk
                .trim_end_matches(|c| c == '\r' || c == '\n')
                .to_string();
            append_external_cli_event(
                &pool,
                &run_id,
                &work_item_id,
                &session_log_path,
                stream,
                message,
                &sequence,
            )
            .await?;
        }
        Ok(())
    })
}

async fn join_output_reader(
    pool: &SqlitePool,
    run_id: &str,
    work_item_id: &str,
    session_log_path: &str,
    task: Option<JoinHandle<Result<(), AppError>>>,
    sequence: &Arc<AtomicI64>,
) -> Result<(), AppError> {
    let Some(task) = task else {
        return Ok(());
    };
    match task.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            append_external_cli_event(
                pool,
                run_id,
                work_item_id,
                session_log_path,
                "error",
                format!("Failed while reading process output: {error}"),
                sequence,
            )
            .await?;
            Ok(())
        }
        Err(error) => {
            append_external_cli_event(
                pool,
                run_id,
                work_item_id,
                session_log_path,
                "error",
                format!("Output reader task failed: {error}"),
                sequence,
            )
            .await?;
            Ok(())
        }
    }
}

async fn append_external_cli_event(
    pool: &SqlitePool,
    run_id: &str,
    work_item_id: &str,
    session_log_path: &str,
    stream: &str,
    message: String,
    sequence: &Arc<AtomicI64>,
) -> Result<(), AppError> {
    let sequence_number = sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let event_id = Uuid::new_v4().to_string();
    let message = truncate_event_message(&message);
    external_cli_repo::append_external_cli_run_event(
        pool,
        &event_id,
        run_id,
        work_item_id,
        stream,
        &message,
        sequence_number,
    )
    .await?;
    append_external_cli_session_log(session_log_path, sequence_number, stream, &message).await?;
    Ok(())
}

async fn append_external_cli_session_log(
    session_log_path: &str,
    sequence: i64,
    stream: &str,
    message: &str,
) -> Result<(), AppError> {
    if session_log_path.trim().is_empty() {
        return Ok(());
    }
    let path = Path::new(session_log_path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(format_session_log_entry(sequence, stream, message).as_bytes())
        .await?;
    Ok(())
}

fn format_session_log_entry(sequence: i64, stream: &str, message: &str) -> String {
    let label = match stream {
        "stdout" => "STDOUT",
        "stderr" => "STDERR",
        "error" => "ERROR",
        "lifecycle" => "INFO",
        other => other,
    };
    let timestamp = Utc::now().to_rfc3339();
    let content = if message.is_empty() { "" } else { message };
    format!("[{timestamp}] #{sequence} {label}: {content}\n")
}

fn truncate_event_message(message: &str) -> String {
    let mut chars = message.chars();
    let mut truncated = String::new();
    for _ in 0..EXTERNAL_CLI_EVENT_MESSAGE_MAX_CHARS {
        match chars.next() {
            Some(value) => truncated.push(value),
            None => return truncated,
        }
    }
    if chars.next().is_some() {
        truncated.push_str("\n[truncated]");
    }
    truncated
}

fn format_invocation_for_event(invocation: &ExternalCliInvocation) -> String {
    let args = invocation.args.iter().map(|arg| {
        if arg == &invocation.prompt {
            "<work item prompt>".to_string()
        } else if arg.contains(' ') {
            format!("\"{}\"", arg)
        } else {
            arg.to_string()
        }
    });
    std::iter::once(invocation.command.clone())
        .chain(args)
        .collect::<Vec<_>>()
        .join(" ")
}

fn summarize_external_cli_failure(
    label: &str,
    exit_code: Option<i64>,
    process_error: Option<&str>,
    stdout: &str,
    stderr: &str,
) -> String {
    let combined = format!("{stdout}\n{stderr}");
    let mut parts = Vec::new();

    if combined.contains("stream disconnected before completion")
        && combined.contains("chatgpt.com/backend-api/codex/responses")
    {
        parts.push(format!(
            "{label} disconnected from the ChatGPT Codex backend before completion; verify Codex login, network/proxy access, and retry."
        ));
    }

    if combined.contains("MCP API authorization failed") {
        parts.push(
            "Codex also tried to initialize the Aruvi MCP bridge without the required bearer authorization; Aruvi now launches Codex with an isolated config to avoid inheriting that broken global MCP entry.".to_string(),
        );
    }

    if combined.contains("https://chatgpt.com/backend-api/wham/apps") {
        parts.push(
            "Codex could not reach the ChatGPT apps endpoint while shutting down MCP clients."
                .to_string(),
        );
    }

    if parts.is_empty() {
        parts.push(process_error.map(ToOwned::to_owned).unwrap_or_else(|| {
            format!(
                "{label} exited with status {}.",
                exit_code
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "terminated".to_string())
            )
        }));
    }

    parts.join(" ")
}

async fn ensure_repository_ready_for_external_cli(
    pool: &SqlitePool,
    work_item: &WorkItem,
) -> Result<Repository, AppError> {
    let repository = repository_repo::resolve_repository_for_work_item(pool, &work_item.id)
        .await?
        .ok_or_else(|| {
            AppError::Validation(
                "Repository readiness failed for external CLI invocation: no repository is attached to the work item, module, or product scope."
                    .to_string(),
            )
        })?;

    let repo_path = Path::new(&repository.local_path);
    if !repo_path.exists() {
        return Err(AppError::Validation(format!(
            "Repository readiness failed for external CLI invocation: attached repository path does not exist: {}",
            repository.local_path
        )));
    }
    if !repo_path.is_dir() {
        return Err(AppError::Validation(format!(
            "Repository readiness failed for external CLI invocation: attached repository path is not a directory: {}",
            repository.local_path
        )));
    }

    if work_item.active_repo_id.as_deref() != Some(repository.id.as_str()) {
        sqlx::query(
            "UPDATE work_items SET active_repo_id=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&repository.id)
        .bind(&work_item.id)
        .execute(pool)
        .await?;
    }

    Ok(repository)
}

fn build_external_cli_task_packet(
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

fn build_external_cli_prompt(task_packet: &ExternalCliTaskPacket) -> Result<String, AppError> {
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

async fn capture_workspace_snapshot(repo_path: &Path) -> ExternalCliWorkspaceSnapshot {
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

async fn capture_git_diff_snapshot(repo_path: &Path) -> ExternalCliDiffSnapshot {
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

async fn store_external_cli_artifact(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    run_id: &str,
    invocation: &ExternalCliInvocation,
    task_packet: &ExternalCliTaskPacket,
    status: &str,
    exit_code: Option<i64>,
    started_at: &str,
    ended_at: &str,
    duration_ms: i64,
    stdout: &str,
    stderr: &str,
    diff_snapshot: &ExternalCliDiffSnapshot,
    review_checkpoint: Option<&ExternalCliReviewCheckpoint>,
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
        task_packet,
        status,
        exit_code,
        started_at,
        ended_at,
        duration_ms,
        stdout,
        stderr,
        session_log_path: &invocation.session_log_path,
        diff_snapshot,
        review_checkpoint,
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
        assert!(
            prompt.contains("report what changed, commands run, validation results, and blockers")
        );
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

        let (copilot_command, copilot_args) =
            ExternalCliProvider::Copilot.command_spec(prompt, cwd);
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
}

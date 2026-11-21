use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun};
use crate::domain::repository::Repository;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{external_cli_repo, repository_repo, work_item_repo};
use crate::services::external_cli_events::{
    append_external_cli_event, format_invocation_for_event, join_output_reader,
    spawn_output_reader, summarize_external_cli_failure, ExternalCliOutputReaderContext,
};
use crate::services::external_cli_finalization::{
    finalize_external_cli_run, ExternalCliRunFinalization,
};
use crate::services::external_cli_provider::ExternalCliProvider;
use crate::services::external_cli_task_packet::{
    build_external_cli_prompt, build_external_cli_task_packet, ExternalCliTaskPacket,
};
use crate::services::external_cli_workspace::capture_workspace_snapshot;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{atomic::AtomicI64, Arc};
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const EXTERNAL_CLI_TIMEOUT_SECS: u64 = 15 * 60;

pub async fn run_external_cli_for_work_item(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    provider: &str,
) -> Result<ExternalCliRun, AppError> {
    run_external_cli(pool, artifact_base_path, work_item_id, provider, None).await
}

/// Remote launches are explicitly enabled by the server operator and use a
/// separate CLI home. This is not a filesystem or process sandbox.
pub async fn run_remote_external_cli_for_work_item(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    provider: &str,
) -> Result<ExternalCliRun, AppError> {
    let home = remote_cli_home(
        std::env::var_os("ARUVI_EXTERNAL_CLI_ENABLED"),
        std::env::var_os("ARUVI_EXTERNAL_CLI_HOME"),
    )?;
    run_external_cli(pool, artifact_base_path, work_item_id, provider, Some(home)).await
}

fn remote_cli_home(enabled: Option<std::ffi::OsString>, home: Option<std::ffi::OsString>) -> Result<PathBuf, AppError> {
    if enabled.as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Err(AppError::Validation("Remote external CLI execution is not enabled".into()));
    }
    let home = home.map(PathBuf::from).filter(|path| path.is_absolute() && path.is_dir())
        .ok_or_else(|| AppError::Validation("Remote external CLI home must be an existing absolute directory".into()))?;
    Ok(home)
}

fn configure_remote_cli_command(command: &mut Command, home: &Path) {
    command.env_clear().env("HOME", home)
        .env("PATH", std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()))
        .env("TMPDIR", "/tmp");
}

async fn run_external_cli(
    pool: &SqlitePool,
    artifact_base_path: &Path,
    work_item_id: &str,
    provider: &str,
    remote_home: Option<PathBuf>,
) -> Result<ExternalCliRun, AppError> {
    let provider = ExternalCliProvider::parse(provider)?;
    let work_item = work_item_repo::get_work_item(pool, work_item_id).await?;
    let repository = ensure_repository_ready_for_external_cli(pool, &work_item, remote_home.is_none()).await?;
    let workspace_snapshot = capture_workspace_snapshot(Path::new(&repository.local_path)).await;
    let task_packet =
        build_external_cli_task_packet(provider, &work_item, &repository, workspace_snapshot);
    let prompt = build_external_cli_prompt(&task_packet)?;
    let run_id = Uuid::new_v4().to_string();
    let run_dir = artifact_base_path.join("external_cli_runs").join(&run_id);
    if remote_home.is_some() {
        tokio::fs::create_dir_all(artifact_base_path.join("external_cli_runs")).await?;
        tokio::fs::create_dir(&run_dir).await?;
    } else {
        tokio::fs::create_dir_all(&run_dir).await?;
    }
    let session_log_path = run_dir.join("session.log").to_string_lossy().to_string();
    let (command, args) = provider.command_spec(&prompt, &repository.local_path);
    let invocation = ExternalCliInvocation {
        work_item_id: work_item_id.to_string(),
        provider: provider.as_str().to_string(),
        label: provider.label().to_string(),
        command: command.to_string(),
        args,
        prompt,
        cwd: repository.local_path.clone(),
        session_log_path,
    };
    let run = if remote_home.is_some() {
        match external_cli_repo::create_remote_external_cli_run(pool, &run_id, &invocation, &repository.id).await {
            Ok(run) => run,
            Err(error) => { let _ = tokio::fs::remove_dir(&run_dir).await; return Err(error); }
        }
    } else {
        external_cli_repo::create_external_cli_run(pool, &run_id, &invocation).await?
    };

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
            remote_home,
        )
        .await
        {
            let failure_message = format!("External CLI tracking failed: {error}");
            let _ = external_cli_repo::complete_external_cli_run(
                &fallback_pool,
                external_cli_repo::CompleteExternalCliRunInput {
                    id: &fallback_run_id,
                    status: "failed",
                    exit_code: None,
                    duration_ms: 0,
                    stdout_chars: 0,
                    stderr_chars: 0,
                    output_artifact_id: None,
                    error_message: Some(&failure_message),
                },
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
    remote_home: Option<PathBuf>,
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

    let mut command = Command::new(&invocation.command);
    command.args(&invocation.args)
        .current_dir(&invocation.cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let remote = remote_home.is_some();
    if let Some(home) = remote_home {
        configure_remote_cli_command(&mut command, &home);
        command.kill_on_drop(true);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }
    }
    let mut child = match command.spawn() {
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
                ExternalCliRunFinalization {
                    artifact_base_path: &artifact_base_path,
                    run_id: &run_id,
                    invocation: &invocation,
                    task_packet: &task_packet,
                    status: "failed",
                    exit_code: None,
                    started_at: &started_at,
                    ended_at: Utc::now().to_rfc3339(),
                    duration_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
                    stdout: String::new(),
                    stderr: String::new(),
                    error_message: Some(error_message),
                    sequence: &sequence,
                },
            )
            .await?;
            return Ok(());
        }
    };

    #[cfg(unix)]
    let process_group = if remote { child.id() } else { None };
    let stdout = Arc::new(Mutex::new(String::new()));
    let stderr = Arc::new(Mutex::new(String::new()));
    let stdout_task = child.stdout.take().map(|reader| {
        spawn_output_reader(
            ExternalCliOutputReaderContext {
                pool: pool.clone(),
                run_id: run_id.clone(),
                work_item_id: invocation.work_item_id.clone(),
                session_log_path: invocation.session_log_path.clone(),
                stream: "stdout",
                capture: stdout.clone(),
                sequence: sequence.clone(),
                bounded: remote,
            },
            reader,
        )
    });
    let stderr_task = child.stderr.take().map(|reader| {
        spawn_output_reader(
            ExternalCliOutputReaderContext {
                pool: pool.clone(),
                run_id: run_id.clone(),
                work_item_id: invocation.work_item_id.clone(),
                session_log_path: invocation.session_log_path.clone(),
                stream: "stderr",
                capture: stderr.clone(),
                sequence: sequence.clone(),
                bounded: remote,
            },
            reader,
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
                #[cfg(unix)]
                if let Some(id) = process_group { unsafe { libc::kill(-(id as i32), libc::SIGKILL); } }
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

    #[cfg(unix)]
    if let Some(id) = process_group { unsafe { libc::kill(-(id as i32), libc::SIGKILL); } }
    join_output_reader(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        stdout_task,
        &sequence,
        remote,
    )
    .await?;
    join_output_reader(
        &pool,
        &run_id,
        &invocation.work_item_id,
        &invocation.session_log_path,
        stderr_task,
        &sequence,
        remote,
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
        ExternalCliRunFinalization {
            artifact_base_path: &artifact_base_path,
            run_id: &run_id,
            invocation: &invocation,
            task_packet: &task_packet,
            status: &status,
            exit_code,
            started_at: &started_at,
            ended_at,
            duration_ms,
            stdout,
            stderr,
            error_message,
            sequence: &sequence,
        },
    )
    .await?;

    Ok(())
}

async fn ensure_repository_ready_for_external_cli(
    pool: &SqlitePool,
    work_item: &WorkItem,
    update_active: bool,
) -> Result<Repository, AppError> {
    let repository = repository_repo::resolve_repository_for_work_item(pool, &work_item.id)
        .await?
        .ok_or_else(|| {
            AppError::Validation(
                "Repository readiness failed for external CLI invocation: no repository is attached to the work item, product area, or product scope."
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

    if update_active && work_item.active_repo_id.as_deref() != Some(repository.id.as_str()) {
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

#[cfg(test)]
mod tests;

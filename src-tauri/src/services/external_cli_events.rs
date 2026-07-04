use crate::domain::external_cli::ExternalCliInvocation;
use crate::error::AppError;
use crate::persistence::external_cli_repo;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

const EXTERNAL_CLI_EVENT_MESSAGE_MAX_CHARS: usize = 16_000;

pub(crate) struct ExternalCliOutputReaderContext {
    pub(crate) pool: SqlitePool,
    pub(crate) run_id: String,
    pub(crate) work_item_id: String,
    pub(crate) session_log_path: String,
    pub(crate) stream: &'static str,
    pub(crate) capture: Arc<Mutex<String>>,
    pub(crate) sequence: Arc<AtomicI64>,
}

pub(crate) fn spawn_output_reader<R>(
    context: ExternalCliOutputReaderContext,
    reader: R,
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
                let mut output = context.capture.lock().await;
                output.push_str(&chunk);
            }
            let message = chunk.trim_end_matches(['\r', '\n']).to_string();
            append_external_cli_event(
                &context.pool,
                &context.run_id,
                &context.work_item_id,
                &context.session_log_path,
                context.stream,
                message,
                &context.sequence,
            )
            .await?;
        }
        Ok(())
    })
}

pub(crate) async fn join_output_reader(
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

pub(crate) async fn append_external_cli_event(
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

pub(crate) async fn append_external_cli_session_log(
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

pub(crate) fn format_invocation_for_event(invocation: &ExternalCliInvocation) -> String {
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

pub(crate) fn summarize_external_cli_failure(
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

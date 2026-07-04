use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun, ExternalCliRunEvent};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};

fn row_to_external_cli_run(row: sqlx::sqlite::SqliteRow) -> ExternalCliRun {
    let args_json: String = row.get("args_json");
    ExternalCliRun {
        id: row.get("id"),
        work_item_id: row.get("work_item_id"),
        provider: row.get("provider"),
        label: row.get("label"),
        command: row.get("command"),
        args: serde_json::from_str::<Vec<String>>(&args_json).unwrap_or_default(),
        prompt: row.get("prompt"),
        cwd: row.get("cwd"),
        status: row.get("status"),
        exit_code: row.get("exit_code"),
        duration_ms: row.get("duration_ms"),
        stdout_chars: row.get("stdout_chars"),
        stderr_chars: row.get("stderr_chars"),
        session_log_path: row.get("session_log_path"),
        output_artifact_id: row.get("output_artifact_id"),
        error_message: row.get("error_message"),
        started_at: row.get("started_at"),
        ended_at: row.get("ended_at"),
        created_at: row.get("created_at"),
    }
}

fn row_to_external_cli_run_event(row: sqlx::sqlite::SqliteRow) -> ExternalCliRunEvent {
    ExternalCliRunEvent {
        id: row.get("id"),
        run_id: row.get("run_id"),
        work_item_id: row.get("work_item_id"),
        stream: row.get("stream"),
        message: row.get("message"),
        sequence: row.get("sequence"),
        created_at: row.get("created_at"),
    }
}

pub async fn create_external_cli_run(
    pool: &SqlitePool,
    id: &str,
    invocation: &ExternalCliInvocation,
) -> Result<ExternalCliRun, AppError> {
    let args_json = serde_json::to_string(&invocation.args)?;
    sqlx::query(
        "INSERT INTO external_cli_runs (
            id,work_item_id,provider,label,command,args_json,prompt,cwd,status,session_log_path,started_at
         ) VALUES (?,?,?,?,?,?,?,?,?,?,datetime('now'))",
    )
    .bind(id)
    .bind(&invocation.work_item_id)
    .bind(&invocation.provider)
    .bind(&invocation.label)
    .bind(&invocation.command)
    .bind(args_json)
    .bind(&invocation.prompt)
    .bind(&invocation.cwd)
    .bind("running")
    .bind(&invocation.session_log_path)
    .execute(pool)
    .await?;
    get_external_cli_run(pool, id).await
}

pub struct CompleteExternalCliRunInput<'a> {
    pub id: &'a str,
    pub status: &'a str,
    pub exit_code: Option<i64>,
    pub duration_ms: i64,
    pub stdout_chars: i64,
    pub stderr_chars: i64,
    pub output_artifact_id: Option<&'a str>,
    pub error_message: Option<&'a str>,
}

pub async fn complete_external_cli_run(
    pool: &SqlitePool,
    input: CompleteExternalCliRunInput<'_>,
) -> Result<ExternalCliRun, AppError> {
    sqlx::query(
        "UPDATE external_cli_runs
         SET status=?,
             exit_code=?,
             duration_ms=?,
             stdout_chars=?,
             stderr_chars=?,
             output_artifact_id=?,
             error_message=?,
             ended_at=datetime('now')
         WHERE id=?",
    )
    .bind(input.status)
    .bind(input.exit_code)
    .bind(input.duration_ms)
    .bind(input.stdout_chars)
    .bind(input.stderr_chars)
    .bind(input.output_artifact_id)
    .bind(input.error_message)
    .bind(input.id)
    .execute(pool)
    .await?;
    get_external_cli_run(pool, input.id).await
}

pub async fn append_external_cli_run_event(
    pool: &SqlitePool,
    id: &str,
    run_id: &str,
    work_item_id: &str,
    stream: &str,
    message: &str,
    sequence: i64,
) -> Result<ExternalCliRunEvent, AppError> {
    sqlx::query(
        "INSERT INTO external_cli_run_events (
            id,run_id,work_item_id,stream,message,sequence
         ) VALUES (?,?,?,?,?,?)",
    )
    .bind(id)
    .bind(run_id)
    .bind(work_item_id)
    .bind(stream)
    .bind(message)
    .bind(sequence)
    .execute(pool)
    .await?;
    get_external_cli_run_event(pool, id).await
}

pub async fn get_external_cli_run(pool: &SqlitePool, id: &str) -> Result<ExternalCliRun, AppError> {
    sqlx::query(
        "SELECT id,work_item_id,provider,label,command,args_json,prompt,cwd,status,exit_code,duration_ms,stdout_chars,stderr_chars,session_log_path,output_artifact_id,error_message,started_at,ended_at,created_at
         FROM external_cli_runs WHERE id=?",
    )
    .bind(id)
    .map(row_to_external_cli_run)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("External CLI run {id} not found")))
}

async fn get_external_cli_run_event(
    pool: &SqlitePool,
    id: &str,
) -> Result<ExternalCliRunEvent, AppError> {
    sqlx::query(
        "SELECT id,run_id,work_item_id,stream,message,sequence,created_at
         FROM external_cli_run_events WHERE id=?",
    )
    .bind(id)
    .map(row_to_external_cli_run_event)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("External CLI run event {id} not found")))
}

pub async fn list_external_cli_run_events(
    pool: &SqlitePool,
    run_id: &str,
    limit: i64,
) -> Result<Vec<ExternalCliRunEvent>, AppError> {
    let bounded_limit = limit.clamp(1, 2_000);
    let mut events = sqlx::query(
        "SELECT id,run_id,work_item_id,stream,message,sequence,created_at
         FROM external_cli_run_events
         WHERE run_id=?
         ORDER BY sequence DESC, created_at DESC
         LIMIT ?",
    )
    .bind(run_id)
    .bind(bounded_limit)
    .map(row_to_external_cli_run_event)
    .fetch_all(pool)
    .await?;
    events.sort_by(|a, b| {
        a.sequence
            .cmp(&b.sequence)
            .then_with(|| a.created_at.cmp(&b.created_at))
    });
    Ok(events)
}

pub async fn mark_interrupted_external_cli_runs(pool: &SqlitePool) -> Result<u64, AppError> {
    let message =
        "Run marked failed because Aruvi Studio restarted before the external CLI process reported completion.";
    sqlx::query(
        "INSERT INTO external_cli_run_events (
            id,run_id,work_item_id,stream,message,sequence
         )
         SELECT
            'startup-recovery-' || r.id || '-' || strftime('%s','now'),
            r.id,
            r.work_item_id,
            'error',
            ?,
            COALESCE((
                SELECT MAX(e.sequence)
                FROM external_cli_run_events e
                WHERE e.run_id = r.id
            ), 0) + 1
         FROM external_cli_runs r
         WHERE r.status = 'running'",
    )
    .bind(message)
    .execute(pool)
    .await?;

    let result = sqlx::query(
        "UPDATE external_cli_runs
         SET status='failed',
             exit_code=NULL,
             duration_ms=COALESCE(
                duration_ms,
                CAST((julianday('now') - julianday(started_at)) * 86400000 AS INTEGER)
             ),
             error_message=?,
             ended_at=datetime('now')
         WHERE status='running'",
    )
    .bind(message)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn list_external_cli_runs_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Vec<ExternalCliRun>, AppError> {
    sqlx::query(
        "SELECT id,work_item_id,provider,label,command,args_json,prompt,cwd,status,exit_code,duration_ms,stdout_chars,stderr_chars,session_log_path,output_artifact_id,error_message,started_at,ended_at,created_at
         FROM external_cli_runs WHERE work_item_id=? ORDER BY started_at DESC",
    )
    .bind(work_item_id)
    .map(row_to_external_cli_run)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

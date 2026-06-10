use crate::domain::external_cli::{ExternalCliInvocation, ExternalCliRun};
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
        output_artifact_id: row.get("output_artifact_id"),
        error_message: row.get("error_message"),
        started_at: row.get("started_at"),
        ended_at: row.get("ended_at"),
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
            id,work_item_id,provider,label,command,args_json,prompt,cwd,status,started_at
         ) VALUES (?,?,?,?,?,?,?,?,?,datetime('now'))",
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
    .execute(pool)
    .await?;
    get_external_cli_run(pool, id).await
}

pub async fn complete_external_cli_run(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    exit_code: Option<i64>,
    duration_ms: i64,
    stdout_chars: i64,
    stderr_chars: i64,
    output_artifact_id: Option<&str>,
    error_message: Option<&str>,
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
    .bind(status)
    .bind(exit_code)
    .bind(duration_ms)
    .bind(stdout_chars)
    .bind(stderr_chars)
    .bind(output_artifact_id)
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;
    get_external_cli_run(pool, id).await
}

pub async fn get_external_cli_run(pool: &SqlitePool, id: &str) -> Result<ExternalCliRun, AppError> {
    sqlx::query(
        "SELECT id,work_item_id,provider,label,command,args_json,prompt,cwd,status,exit_code,duration_ms,stdout_chars,stderr_chars,output_artifact_id,error_message,started_at,ended_at,created_at
         FROM external_cli_runs WHERE id=?",
    )
    .bind(id)
    .map(row_to_external_cli_run)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("External CLI run {id} not found")))
}

pub async fn list_external_cli_runs_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Vec<ExternalCliRun>, AppError> {
    sqlx::query(
        "SELECT id,work_item_id,provider,label,command,args_json,prompt,cwd,status,exit_code,duration_ms,stdout_chars,stderr_chars,output_artifact_id,error_message,started_at,ended_at,created_at
         FROM external_cli_runs WHERE work_item_id=? ORDER BY started_at DESC",
    )
    .bind(work_item_id)
    .map(row_to_external_cli_run)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

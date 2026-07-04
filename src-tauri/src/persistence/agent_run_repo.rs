use crate::domain::agent::{AgentRun, AgentRunStatus};
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_agent_run(
    pool: &SqlitePool,
    id: &str,
    workflow_run_id: &str,
    work_item_id: &str,
    agent_id: &str,
    model_id: &str,
    stage: &str,
) -> Result<AgentRun, AppError> {
    sqlx::query_as::<_, AgentRun>(
        "INSERT INTO agent_runs (id,workflow_run_id,work_item_id,agent_id,model_id,stage,status,started_at)
         VALUES (?,?,?,?,?,?,?,datetime('now'))
         RETURNING id,workflow_run_id,agent_id,stage,status,prompt_snapshot_path,output_snapshot_path,token_count_input,token_count_output,duration_ms,error_message,started_at,ended_at,created_at"
    )
    .bind(id)
    .bind(workflow_run_id)
    .bind(work_item_id)
    .bind(agent_id)
    .bind(model_id)
    .bind(stage)
    .bind("running")
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn update_agent_run_status(
    pool: &SqlitePool,
    id: &str,
    status: AgentRunStatus,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE agent_runs
         SET status=?,
             ended_at=datetime('now'),
             duration_ms=CAST((julianday(datetime('now')) - julianday(started_at)) * 86400000 AS INTEGER)
         WHERE id=?",
    )
    .bind(status.as_str())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_agent_run_failure(
    pool: &SqlitePool,
    id: &str,
    error_message: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE agent_runs
         SET status='failed',
             error_message=?,
             ended_at=datetime('now'),
             duration_ms=CAST((julianday(datetime('now')) - julianday(started_at)) * 86400000 AS INTEGER)
         WHERE id=?",
    )
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_agent_run_token_usage(
    pool: &SqlitePool,
    id: &str,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
) -> Result<(), AppError> {
    if token_count_input.is_none() && token_count_output.is_none() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE agent_runs
         SET token_count_input = COALESCE(token_count_input, 0) + ?,
             token_count_output = COALESCE(token_count_output, 0) + ?
         WHERE id=?",
    )
    .bind(token_count_input.unwrap_or(0))
    .bind(token_count_output.unwrap_or(0))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_agent_run_snapshot_paths(
    pool: &SqlitePool,
    id: &str,
    prompt_snapshot_path: &str,
    output_snapshot_path: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE agent_runs
         SET prompt_snapshot_path=?,
             output_snapshot_path=?
         WHERE id=?",
    )
    .bind(prompt_snapshot_path)
    .bind(output_snapshot_path)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_agent_runs_for_workflow(
    pool: &SqlitePool,
    workflow_run_id: &str,
) -> Result<Vec<AgentRun>, AppError> {
    sqlx::query_as::<_, AgentRun>(
        "SELECT id,workflow_run_id,agent_id,stage,status,prompt_snapshot_path,output_snapshot_path,token_count_input,token_count_output,duration_ms,error_message,started_at,ended_at,created_at
         FROM agent_runs WHERE workflow_run_id=? ORDER BY started_at ASC",
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

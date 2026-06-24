use crate::domain::agent_work::AgentWorkBatch;
use crate::error::AppError;
use crate::persistence::agent_work_event_repo::{append_event, AppendAgentWorkEventInput};
use crate::persistence::agent_work_status::normalize_batch_status;
use sqlx::SqlitePool;

pub async fn complete_batch(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: &str,
    status: &str,
    agent: Option<&str>,
    commit_sha: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkBatch, AppError> {
    let status = normalize_batch_status(status)?;
    sqlx::query(
        "INSERT INTO agent_work_batches (id, run_id, status, agent, commit_sha, completed_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            status=excluded.status,
            agent=COALESCE(excluded.agent, agent_work_batches.agent),
            commit_sha=COALESCE(excluded.commit_sha, agent_work_batches.commit_sha),
            completed_at=CASE
                WHEN excluded.status IN ('committed','blocked','skipped','cancelled') THEN datetime('now')
                ELSE agent_work_batches.completed_at
            END,
            updated_at=datetime('now')",
    )
    .bind(batch_id)
    .bind(run_id)
    .bind(status)
    .bind(agent)
    .bind(commit_sha)
    .execute(pool)
    .await?;
    append_event(
        pool,
        AppendAgentWorkEventInput {
            run_id,
            event_type: "batch_status",
            batch_id: Some(batch_id),
            feature_id: None,
            work_item_id: None,
            agent,
            command: None,
            status: Some(status),
            details,
            metadata: None,
        },
    )
    .await?;
    get_batch(pool, batch_id).await
}

pub async fn get_batch(pool: &SqlitePool, batch_id: &str) -> Result<AgentWorkBatch, AppError> {
    sqlx::query_as::<_, AgentWorkBatch>(
        "SELECT id,run_id,status,selection_rule,agent,commit_sha,metadata_json,started_at,completed_at,updated_at
         FROM agent_work_batches WHERE id=?",
    )
    .bind(batch_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work batch {batch_id} not found")))
}

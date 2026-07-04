use crate::domain::agent_work::{
    AgentWorkAgentActivity, AgentWorkRun, AgentWorkRunHealth, AgentWorkRunSummary,
    AgentWorkStatusCount,
};
use crate::error::AppError;
use crate::persistence::agent_work_dependency_repo::ready_item_count;
use crate::persistence::agent_work_event_repo::{list_events, list_evidence};
use serde_json::Value;
use sqlx::SqlitePool;

fn json_object_string(value: Option<Value>) -> Result<String, AppError> {
    match value {
        Some(value @ Value::Object(_)) => serde_json::to_string(&value).map_err(AppError::from),
        Some(Value::Null) | None => Ok("{}".to_string()),
        Some(_) => Err(AppError::Validation(
            "metadata must be a JSON object".to_string(),
        )),
    }
}

fn normalize_run_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" => Ok("active"),
        "paused" => Ok("paused"),
        "completed" => Ok("completed"),
        "blocked" => Ok("blocked"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work run status '{other}'."
        ))),
    }
}

pub struct UpsertAgentWorkRunInput<'a> {
    pub id: &'a str,
    pub product_id: Option<&'a str>,
    pub repository_id: Option<&'a str>,
    pub roadmap_hash: &'a str,
    pub status: Option<&'a str>,
    pub last_commit_sha: Option<&'a str>,
    pub current_batch_id: Option<&'a str>,
    pub next_action: Option<&'a str>,
    pub metadata: Option<Value>,
}

pub async fn upsert_run(
    pool: &SqlitePool,
    input: UpsertAgentWorkRunInput<'_>,
) -> Result<AgentWorkRun, AppError> {
    let status = normalize_run_status(input.status.unwrap_or("active"))?;
    let metadata_json = json_object_string(input.metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_runs (
            id, product_id, repository_id, roadmap_hash, status, last_commit_sha,
            current_batch_id, next_action, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            product_id=excluded.product_id,
            repository_id=excluded.repository_id,
            roadmap_hash=excluded.roadmap_hash,
            status=excluded.status,
            last_commit_sha=COALESCE(excluded.last_commit_sha, agent_work_runs.last_commit_sha),
            current_batch_id=excluded.current_batch_id,
            next_action=excluded.next_action,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(input.id)
    .bind(input.product_id)
    .bind(input.repository_id)
    .bind(input.roadmap_hash)
    .bind(status)
    .bind(input.last_commit_sha)
    .bind(input.current_batch_id)
    .bind(input.next_action.unwrap_or(""))
    .bind(metadata_json)
    .execute(pool)
    .await?;
    get_run(pool, input.id).await
}

pub async fn get_run(pool: &SqlitePool, id: &str) -> Result<AgentWorkRun, AppError> {
    sqlx::query_as::<_, AgentWorkRun>(
        "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
         FROM agent_work_runs WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work run {id} not found")))
}

pub async fn list_runs(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkRun>, AppError> {
    let limit = limit.clamp(1, 500);
    if let Some(status) = status {
        let status = normalize_run_status(status)?;
        sqlx::query_as::<_, AgentWorkRun>(
            "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
             FROM agent_work_runs WHERE status=? ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(status)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, AgentWorkRun>(
            "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
             FROM agent_work_runs ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

pub async fn get_run_summary(
    pool: &SqlitePool,
    run_id: &str,
    event_limit: i64,
) -> Result<AgentWorkRunSummary, AppError> {
    let run = get_run(pool, run_id).await?;
    let status_counts = sqlx::query_as::<_, AgentWorkStatusCount>(
        "SELECT status, COUNT(*) AS count FROM agent_work_items WHERE run_id=? GROUP BY status ORDER BY status",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    let active_locks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_locks WHERE run_id=? AND released_at IS NULL",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let latest_events = list_events(pool, run_id, None, None, event_limit).await?;
    Ok(AgentWorkRunSummary {
        run,
        status_counts,
        active_locks,
        latest_events,
    })
}

pub async fn get_run_health(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<AgentWorkRunHealth, AppError> {
    let summary = get_run_summary(pool, run_id, 20).await?;
    let ready_items = ready_item_count(pool, run_id).await?;
    let expired_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_items
         WHERE run_id=?
           AND status IN ('claimed','in_progress')
           AND lease_expires_at IS NOT NULL
           AND lease_expires_at <= datetime('now')",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let blocked_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_items WHERE run_id=? AND status='blocked'",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let active_agents: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT agent) FROM agent_work_items
         WHERE run_id=? AND agent IS NOT NULL AND status IN ('claimed','in_progress','implemented')",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let active_conflict_zones: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT zone_key) FROM agent_work_locks WHERE run_id=? AND released_at IS NULL",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let latest_evidence = list_evidence(pool, run_id, None, None, None, 20).await?;
    Ok(AgentWorkRunHealth {
        summary,
        ready_items,
        expired_claims,
        blocked_items,
        active_agents,
        active_conflict_zones,
        latest_evidence,
    })
}

pub async fn list_agent_activity(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkAgentActivity>, AppError> {
    sqlx::query_as::<_, AgentWorkAgentActivity>(
        "SELECT
            item.agent AS agent,
            SUM(CASE WHEN item.status IN ('claimed','in_progress','implemented') THEN 1 ELSE 0 END) AS active_items,
            SUM(CASE WHEN item.status='claimed' THEN 1 ELSE 0 END) AS claimed_items,
            SUM(CASE WHEN item.status='in_progress' THEN 1 ELSE 0 END) AS in_progress_items,
            SUM(CASE WHEN item.status='implemented' THEN 1 ELSE 0 END) AS implemented_items,
            MAX(item.heartbeat_at) AS latest_heartbeat_at,
            (
                SELECT MAX(event.ts)
                FROM agent_work_events event
                WHERE event.run_id=item.run_id AND event.agent=item.agent
            ) AS latest_event_at
         FROM agent_work_items item
         WHERE item.run_id=? AND item.agent IS NOT NULL
         GROUP BY item.agent
         ORDER BY latest_heartbeat_at DESC, latest_event_at DESC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

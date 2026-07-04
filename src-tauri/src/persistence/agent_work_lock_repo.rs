use crate::domain::agent_work::{AgentWorkConflictZoneSummary, AgentWorkLock};
use crate::error::AppError;
use sqlx::SqlitePool;

const DEFAULT_LEASE_SECONDS: i64 = 900;

fn lease_modifier(lease_seconds: i64) -> String {
    format!("+{} seconds", lease_seconds.clamp(30, 86_400))
}

pub async fn list_active_locks(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkLock>, AppError> {
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks
         WHERE run_id=? AND released_at IS NULL
         ORDER BY zone_key",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_conflict_zones(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkConflictZoneSummary>, AppError> {
    sqlx::query_as::<_, AgentWorkConflictZoneSummary>(
        "SELECT
            zone_key,
            COUNT(*) AS active_locks,
            COALESCE(GROUP_CONCAT(DISTINCT agent), '') AS agents,
            COALESCE(GROUP_CONCAT(DISTINCT feature_id), '') AS feature_ids,
            MIN(lease_expires_at) AS earliest_expiry
         FROM agent_work_locks
         WHERE run_id=? AND released_at IS NULL
         GROUP BY zone_key
         ORDER BY zone_key",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn inspect_conflict_zone(
    pool: &SqlitePool,
    run_id: &str,
    zone_key: &str,
) -> Result<Vec<AgentWorkLock>, AppError> {
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks
         WHERE run_id=? AND zone_key=? AND released_at IS NULL
         ORDER BY lease_expires_at",
    )
    .bind(run_id)
    .bind(zone_key)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub struct ReserveConflictZoneInput<'a> {
    pub run_id: &'a str,
    pub zone_key: &'a str,
    pub agent: &'a str,
    pub batch_id: Option<&'a str>,
    pub feature_id: Option<&'a str>,
    pub claim_token: Option<&'a str>,
    pub lease_seconds: Option<i64>,
}

pub async fn reserve_conflict_zone(
    pool: &SqlitePool,
    input: ReserveConflictZoneInput<'_>,
) -> Result<AgentWorkLock, AppError> {
    let existing = inspect_conflict_zone(pool, input.run_id, input.zone_key).await?;
    if !existing.is_empty() {
        return Err(AppError::Validation(format!(
            "Conflict zone '{}' is already reserved.",
            input.zone_key
        )));
    }
    let claim_token = input
        .claim_token
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
        .bind(lease_modifier(
            input.lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS),
        ))
        .fetch_one(pool)
        .await?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agent_work_locks (
            id, run_id, zone_key, batch_id, feature_id, agent, claim_token, lease_expires_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.run_id)
    .bind(input.zone_key)
    .bind(input.batch_id)
    .bind(input.feature_id)
    .bind(input.agent)
    .bind(&claim_token)
    .bind(&lease_expires_at)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn release_conflict_zone(
    pool: &SqlitePool,
    run_id: &str,
    zone_key: &str,
    claim_token: Option<&str>,
) -> Result<(), AppError> {
    if let Some(claim_token) = claim_token {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND zone_key=? AND claim_token=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(zone_key)
        .bind(claim_token)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND zone_key=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(zone_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}

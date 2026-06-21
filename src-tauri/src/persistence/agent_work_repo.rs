use crate::domain::agent_work::{
    AgentWorkBatch, AgentWorkCatalogLinkResult, AgentWorkEvent, AgentWorkEvidence, AgentWorkItem,
    AgentWorkMaterializationResult,
};
use crate::error::AppError;
pub use crate::persistence::agent_work_claim_repo::claim_next_item;
pub use crate::persistence::agent_work_dependency_repo::{
    delete_dependency, list_dependencies, list_ready_items, upsert_dependency,
};
pub use crate::persistence::agent_work_item_repo::{get_item, list_items, upsert_item};
pub use crate::persistence::agent_work_lock_repo::{
    inspect_conflict_zone, list_active_locks, list_conflict_zones, release_conflict_zone,
    reserve_conflict_zone, ReserveConflictZoneInput,
};
pub use crate::persistence::agent_work_run_repo::{
    get_run, get_run_health, get_run_summary, list_agent_activity, list_runs, upsert_run,
    UpsertAgentWorkRunInput,
};
pub(crate) use crate::persistence::agent_work_status::normalize_batch_status;
use crate::persistence::agent_work_status::normalize_status;
use serde_json::Value;
use sqlx::SqlitePool;

const DEFAULT_LEASE_SECONDS: i64 = 900;

fn lease_modifier(lease_seconds: i64) -> String {
    format!("+{} seconds", lease_seconds.clamp(30, 86_400))
}

pub async fn materialize_catalog(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    create_work_items: bool,
) -> Result<AgentWorkMaterializationResult, AppError> {
    crate::persistence::agent_work_catalog_repo::materialize_catalog(
        pool,
        run_id,
        product_id,
        create_work_items,
    )
    .await
}

pub async fn link_catalog_work_items(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    sync_statuses: bool,
) -> Result<AgentWorkCatalogLinkResult, AppError> {
    crate::persistence::agent_work_catalog_repo::link_catalog_work_items(
        pool,
        run_id,
        product_id,
        sync_statuses,
    )
    .await
}

pub async fn heartbeat_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    claim_token: &str,
    lease_seconds: Option<i64>,
) -> Result<AgentWorkItem, AppError> {
    let lease_modifier = lease_modifier(lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS));
    let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
        .bind(&lease_modifier)
        .fetch_one(pool)
        .await?;
    let result = sqlx::query(
        "UPDATE agent_work_items
         SET lease_expires_at=?, heartbeat_at=datetime('now'), updated_at=datetime('now')
         WHERE run_id=? AND feature_id=? AND claim_token=? AND status IN ('claimed','in_progress','implemented')",
    )
    .bind(&lease_expires_at)
    .bind(run_id)
    .bind(feature_id)
    .bind(claim_token)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Validation(
            "No active claim matched run_id, feature_id, and claim_token.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE agent_work_locks
         SET lease_expires_at=?, updated_at=datetime('now')
         WHERE run_id=? AND feature_id=? AND claim_token=? AND released_at IS NULL",
    )
    .bind(&lease_expires_at)
    .bind(run_id)
    .bind(feature_id)
    .bind(claim_token)
    .execute(pool)
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub struct UpdateAgentWorkItemStatusInput<'a> {
    pub run_id: &'a str,
    pub feature_id: &'a str,
    pub status: &'a str,
    pub agent: Option<&'a str>,
    pub batch_id: Option<&'a str>,
    pub claim_token: Option<&'a str>,
    pub commit_sha: Option<&'a str>,
    pub details: Option<&'a str>,
}

pub async fn update_item_status(
    pool: &SqlitePool,
    input: UpdateAgentWorkItemStatusInput<'_>,
) -> Result<AgentWorkItem, AppError> {
    let status = normalize_status(input.status)?;
    if let Some(claim_token) = input.claim_token {
        let current: Option<String> = sqlx::query_scalar(
            "SELECT claim_token FROM agent_work_items WHERE run_id=? AND feature_id=?",
        )
        .bind(input.run_id)
        .bind(input.feature_id)
        .fetch_optional(pool)
        .await?;
        if current.as_deref() != Some(claim_token) {
            return Err(AppError::Validation(
                "claim_token does not match the current claim.".to_string(),
            ));
        }
    }

    sqlx::query(
        "UPDATE agent_work_items
         SET status=?,
             agent=COALESCE(?, agent),
             batch_id=COALESCE(?, batch_id),
             commit_sha=COALESCE(?, commit_sha),
             updated_at=datetime('now')
         WHERE run_id=? AND feature_id=?",
    )
    .bind(status)
    .bind(input.agent)
    .bind(input.batch_id)
    .bind(input.commit_sha)
    .bind(input.run_id)
    .bind(input.feature_id)
    .execute(pool)
    .await?;

    if matches!(status, "committed" | "blocked" | "skipped" | "cancelled") {
        release_item_locks(pool, input.run_id, input.feature_id, input.claim_token).await?;
    }

    append_event(
        pool,
        input.run_id,
        "status",
        input.batch_id,
        Some(input.feature_id),
        None,
        input.agent,
        None,
        Some(status),
        input.details,
        None,
    )
    .await?;

    get_item(pool, input.run_id, input.feature_id).await
}

pub async fn release_item_locks(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    claim_token: Option<&str>,
) -> Result<(), AppError> {
    if let Some(claim_token) = claim_token {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND feature_id=? AND claim_token=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(claim_token)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND feature_id=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(feature_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn requeue_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkItem, AppError> {
    let current = get_item(pool, run_id, feature_id).await?;
    release_item_locks(pool, run_id, feature_id, current.claim_token.as_deref()).await?;
    sqlx::query(
        "UPDATE agent_work_items
         SET status='pending',
             batch_id=NULL,
             agent=NULL,
             claim_token=NULL,
             lease_expires_at=NULL,
             heartbeat_at=NULL,
             updated_at=datetime('now')
         WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .execute(pool)
    .await?;
    append_event(
        pool,
        run_id,
        "requeue",
        current.batch_id.as_deref(),
        Some(feature_id),
        current.work_item_id.as_deref(),
        agent,
        None,
        Some("pending"),
        details,
        None,
    )
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub async fn requeue_expired_items(
    pool: &SqlitePool,
    run_id: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let expired = sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items
         WHERE run_id=?
           AND status IN ('claimed','in_progress')
           AND lease_expires_at IS NOT NULL
           AND lease_expires_at <= datetime('now')
         ORDER BY lease_expires_at",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    let mut requeued = Vec::new();
    for item in expired {
        requeued.push(requeue_item(pool, run_id, &item.feature_id, agent, details).await?);
    }
    Ok(requeued)
}

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
        run_id,
        "batch_status",
        Some(batch_id),
        None,
        None,
        agent,
        None,
        Some(status),
        details,
        None,
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

pub async fn link_commit(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: &str,
    feature_ids: &[String],
    commit_sha: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkBatch, AppError> {
    for feature_id in feature_ids {
        sqlx::query(
            "UPDATE agent_work_items
             SET status='committed',
                 batch_id=?,
                 agent=COALESCE(?, agent),
                 commit_sha=?,
                 updated_at=datetime('now')
             WHERE run_id=? AND feature_id=?",
        )
        .bind(batch_id)
        .bind(agent)
        .bind(commit_sha)
        .bind(run_id)
        .bind(feature_id)
        .execute(pool)
        .await?;
        release_item_locks(pool, run_id, feature_id, None).await?;
        append_event(
            pool,
            run_id,
            "commit",
            Some(batch_id),
            Some(feature_id),
            None,
            agent,
            Some("git commit"),
            Some("committed"),
            details,
            None,
        )
        .await?;
    }
    sqlx::query(
        "UPDATE agent_work_runs
         SET last_commit_sha=?, current_batch_id=NULL, updated_at=datetime('now')
         WHERE id=?",
    )
    .bind(commit_sha)
    .bind(run_id)
    .execute(pool)
    .await?;
    complete_batch(
        pool,
        run_id,
        batch_id,
        "committed",
        agent,
        Some(commit_sha),
        details,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn append_event(
    pool: &SqlitePool,
    run_id: &str,
    event_type: &str,
    batch_id: Option<&str>,
    feature_id: Option<&str>,
    work_item_id: Option<&str>,
    agent: Option<&str>,
    command: Option<&str>,
    status: Option<&str>,
    details: Option<&str>,
    metadata: Option<Value>,
) -> Result<AgentWorkEvent, AppError> {
    crate::persistence::agent_work_event_repo::append_event(
        pool,
        run_id,
        event_type,
        batch_id,
        feature_id,
        work_item_id,
        agent,
        command,
        status,
        details,
        metadata,
    )
    .await
}

pub async fn list_events(
    pool: &SqlitePool,
    run_id: &str,
    after_id: Option<i64>,
    feature_id: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvent>, AppError> {
    crate::persistence::agent_work_event_repo::list_events(
        pool, run_id, after_id, feature_id, limit,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn append_evidence(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: Option<&str>,
    feature_id: Option<&str>,
    work_item_id: Option<&str>,
    agent: Option<&str>,
    evidence_type: &str,
    command: Option<&str>,
    exit_code: Option<i64>,
    status: Option<&str>,
    summary: &str,
    details: &str,
    changed_files: Option<Value>,
    artifact_refs: Option<Value>,
    metadata: Option<Value>,
) -> Result<AgentWorkEvidence, AppError> {
    crate::persistence::agent_work_event_repo::append_evidence(
        pool,
        run_id,
        batch_id,
        feature_id,
        work_item_id,
        agent,
        evidence_type,
        command,
        exit_code,
        status,
        summary,
        details,
        changed_files,
        artifact_refs,
        metadata,
    )
    .await
}

#[allow(dead_code)]
pub async fn get_evidence(pool: &SqlitePool, id: &str) -> Result<AgentWorkEvidence, AppError> {
    crate::persistence::agent_work_event_repo::get_evidence(pool, id).await
}

pub async fn list_evidence(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: Option<&str>,
    batch_id: Option<&str>,
    agent: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvidence>, AppError> {
    crate::persistence::agent_work_event_repo::list_evidence(
        pool, run_id, feature_id, batch_id, agent, limit,
    )
    .await
}

pub async fn import_legacy_checkpoint(
    pool: &SqlitePool,
    checkpoint_path: &str,
    run_id: Option<&str>,
    source_label: Option<&str>,
) -> Result<Value, AppError> {
    crate::persistence::agent_work_import_repo::import_legacy_checkpoint(
        pool,
        checkpoint_path,
        run_id,
        source_label,
    )
    .await
}

#[cfg(test)]
mod tests;

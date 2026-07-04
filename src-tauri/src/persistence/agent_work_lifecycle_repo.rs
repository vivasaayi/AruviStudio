use crate::domain::agent_work::{AgentWorkBatch, AgentWorkItem};
use crate::error::AppError;
use crate::persistence::agent_work_batch_repo::complete_batch;
use crate::persistence::agent_work_event_repo::{append_event, AppendAgentWorkEventInput};
use crate::persistence::agent_work_item_repo::get_item;
use crate::persistence::agent_work_status::normalize_status;
use sqlx::SqlitePool;

const DEFAULT_LEASE_SECONDS: i64 = 900;

fn lease_modifier(lease_seconds: i64) -> String {
    format!("+{} seconds", lease_seconds.clamp(30, 86_400))
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
        AppendAgentWorkEventInput {
            run_id: input.run_id,
            event_type: "status",
            batch_id: input.batch_id,
            feature_id: Some(input.feature_id),
            work_item_id: None,
            agent: input.agent,
            command: None,
            status: Some(status),
            details: input.details,
            metadata: None,
        },
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
        AppendAgentWorkEventInput {
            run_id,
            event_type: "requeue",
            batch_id: current.batch_id.as_deref(),
            feature_id: Some(feature_id),
            work_item_id: current.work_item_id.as_deref(),
            agent,
            command: None,
            status: Some("pending"),
            details,
            metadata: None,
        },
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
            AppendAgentWorkEventInput {
                run_id,
                event_type: "commit",
                batch_id: Some(batch_id),
                feature_id: Some(feature_id),
                work_item_id: None,
                agent,
                command: Some("git commit"),
                status: Some("committed"),
                details,
                metadata: None,
            },
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

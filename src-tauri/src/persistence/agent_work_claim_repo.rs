use crate::domain::agent_work::{AgentWorkBatch, AgentWorkClaim, AgentWorkItem};
use crate::error::AppError;
use sqlx::SqlitePool;

const DEFAULT_LEASE_SECONDS: i64 = 900;
const MAX_CANDIDATES: i64 = 250;

fn parse_conflict_zones(item: &AgentWorkItem) -> Result<Vec<String>, AppError> {
    let parsed = serde_json::from_str::<Vec<String>>(&item.conflict_zones_json)?;
    let mut zones = parsed
        .into_iter()
        .map(|zone| zone.trim().to_string())
        .filter(|zone| !zone.is_empty())
        .collect::<Vec<_>>();
    if zones.is_empty() {
        if !item.product_area.trim().is_empty() {
            zones.push(format!("product_area:{}", item.product_area.trim()));
        } else if let Some(service) = item.service_or_domain.as_deref() {
            if !service.trim().is_empty() {
                zones.push(format!("service:{}", service.trim()));
            }
        }
    }
    if zones.is_empty() {
        zones.push(format!("feature:{}", item.feature_id));
    }
    zones.sort();
    zones.dedup();
    Ok(zones)
}

fn lease_modifier(lease_seconds: i64) -> String {
    format!("+{} seconds", lease_seconds.clamp(30, 86_400))
}

pub async fn claim_next_item(
    pool: &SqlitePool,
    run_id: &str,
    agent: &str,
    batch_id: Option<&str>,
    selection_rule: Option<&str>,
    lease_seconds: Option<i64>,
) -> Result<Option<AgentWorkClaim>, AppError> {
    let lease_seconds = lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS);
    let lease_modifier = lease_modifier(lease_seconds);
    let batch_id = batch_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("batch-{}", uuid::Uuid::new_v4()));
    let claim_token = uuid::Uuid::new_v4().to_string();
    let mut conn = pool.acquire().await?;

    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
    let result = async {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND released_at IS NULL AND lease_expires_at <= datetime('now')",
        )
        .bind(run_id)
        .execute(&mut *conn)
        .await?;

        let candidates = sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items
             WHERE run_id=? AND status='pending'
               AND NOT EXISTS (
                 SELECT 1
                 FROM agent_work_dependencies dep
                 LEFT JOIN agent_work_items prereq
                   ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
                 WHERE dep.run_id=agent_work_items.run_id
                   AND dep.feature_id=agent_work_items.feature_id
                   AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
               )
             ORDER BY
               CASE priority WHEN 'P0' THEN 0 WHEN 'critical' THEN 0 WHEN 'P1' THEN 1 WHEN 'high' THEN 1 WHEN 'P2' THEN 2 WHEN 'medium' THEN 2 ELSE 3 END,
               feature_id
             LIMIT ?",
        )
        .bind(run_id)
        .bind(MAX_CANDIDATES)
        .fetch_all(&mut *conn)
        .await?;

        for candidate in candidates {
            let zones = parse_conflict_zones(&candidate)?;
            let mut blocked = false;
            for zone in &zones {
                let active_lock: Option<i64> = sqlx::query_scalar(
                    "SELECT 1 FROM agent_work_locks
                     WHERE run_id=? AND zone_key=? AND released_at IS NULL
                     LIMIT 1",
                )
                .bind(run_id)
                .bind(zone)
                .fetch_optional(&mut *conn)
                .await?;
                if active_lock.is_some() {
                    blocked = true;
                    break;
                }
            }
            if blocked {
                continue;
            }

            let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
                .bind(&lease_modifier)
                .fetch_one(&mut *conn)
                .await?;

            let updated = sqlx::query(
                "UPDATE agent_work_items
                 SET status='claimed',
                     batch_id=?,
                     agent=?,
                     claim_token=?,
                     lease_expires_at=?,
                     heartbeat_at=datetime('now'),
                     updated_at=datetime('now')
                 WHERE run_id=? AND feature_id=? AND status='pending'",
            )
            .bind(&batch_id)
            .bind(agent)
            .bind(&claim_token)
            .bind(&lease_expires_at)
            .bind(run_id)
            .bind(&candidate.feature_id)
            .execute(&mut *conn)
            .await?;

            if updated.rows_affected() == 0 {
                continue;
            }

            sqlx::query(
                "INSERT INTO agent_work_batches (id, run_id, status, selection_rule, agent)
                 VALUES (?, ?, 'claimed', ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    status='claimed',
                    selection_rule=COALESCE(excluded.selection_rule, agent_work_batches.selection_rule),
                    agent=excluded.agent,
                    updated_at=datetime('now')",
            )
            .bind(&batch_id)
            .bind(run_id)
            .bind(selection_rule)
            .bind(agent)
            .execute(&mut *conn)
            .await?;

            for zone in &zones {
                sqlx::query(
                    "INSERT INTO agent_work_locks (
                        id, run_id, zone_key, batch_id, feature_id, agent, claim_token, lease_expires_at
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(run_id)
                .bind(zone)
                .bind(&batch_id)
                .bind(&candidate.feature_id)
                .bind(agent)
                .bind(&claim_token)
                .bind(&lease_expires_at)
                .execute(&mut *conn)
                .await?;
            }

            sqlx::query(
                "UPDATE agent_work_runs
                 SET current_batch_id=?, next_action=?, updated_at=datetime('now')
                 WHERE id=?",
            )
            .bind(&batch_id)
            .bind(format!("Implement {}", candidate.feature_id))
            .bind(run_id)
            .execute(&mut *conn)
            .await?;

            sqlx::query(
                "INSERT INTO agent_work_events (
                    run_id, event_type, batch_id, feature_id, work_item_id, agent, status, details
                 ) VALUES (?, 'claim', ?, ?, ?, ?, 'claimed', ?)",
            )
            .bind(run_id)
            .bind(&batch_id)
            .bind(&candidate.feature_id)
            .bind(&candidate.work_item_id)
            .bind(agent)
            .bind(selection_rule.unwrap_or("claimed next pending feature"))
            .execute(&mut *conn)
            .await?;

            let item = get_item_with_conn(&mut conn, run_id, &candidate.feature_id).await?;
            let batch = get_batch_with_conn(&mut conn, &batch_id).await?;
            return Ok(Some(AgentWorkClaim {
                item,
                batch,
                claim_token,
                lease_expires_at,
                conflict_zones: zones,
            }));
        }

        Ok(None)
    }
    .await;

    match result {
        Ok(value) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(value)
        }
        Err(error) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(error)
        }
    }
}

async fn get_item_with_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    run_id: &str,
    feature_id: &str,
) -> Result<AgentWorkItem, AppError> {
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .fetch_one(&mut **conn)
    .await
    .map_err(AppError::from)
}

async fn get_batch_with_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    batch_id: &str,
) -> Result<AgentWorkBatch, AppError> {
    sqlx::query_as::<_, AgentWorkBatch>(
        "SELECT id,run_id,status,selection_rule,agent,commit_sha,metadata_json,started_at,completed_at,updated_at
         FROM agent_work_batches WHERE id=?",
    )
    .bind(batch_id)
    .fetch_one(&mut **conn)
    .await
    .map_err(AppError::from)
}

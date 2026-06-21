use crate::domain::agent_work::AgentWorkItem;
use crate::error::AppError;
use crate::persistence::agent_work_status::normalize_status;
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

fn json_array_string(value: Option<Value>) -> Result<String, AppError> {
    match value {
        Some(value @ Value::Array(_)) => serde_json::to_string(&value).map_err(AppError::from),
        Some(Value::Null) | None => Ok("[]".to_string()),
        Some(Value::String(raw)) => {
            let parsed: Value = serde_json::from_str(&raw)?;
            if !parsed.is_array() {
                return Err(AppError::Validation(
                    "conflict zones must be a JSON array".to_string(),
                ));
            }
            serde_json::to_string(&parsed).map_err(AppError::from)
        }
        Some(_) => Err(AppError::Validation(
            "conflict zones must be a JSON array".to_string(),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    work_item_id: Option<&str>,
    product_area: &str,
    service_or_domain: Option<&str>,
    priority: Option<&str>,
    release_phase: Option<&str>,
    title: &str,
    description: &str,
    status: Option<&str>,
    batch_id: Option<&str>,
    agent: Option<&str>,
    commit_sha: Option<&str>,
    conflict_zones: Option<Value>,
    metadata: Option<Value>,
) -> Result<AgentWorkItem, AppError> {
    let status = normalize_status(status.unwrap_or("pending"))?;
    let conflict_zones_json = json_array_string(conflict_zones)?;
    let metadata_json = json_object_string(metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_items (
            id, run_id, feature_id, work_item_id, product_area, service_or_domain, priority,
            release_phase, title, description, status, batch_id, agent, commit_sha,
            conflict_zones_json, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(run_id, feature_id) DO UPDATE SET
            work_item_id=COALESCE(excluded.work_item_id, agent_work_items.work_item_id),
            product_area=excluded.product_area,
            service_or_domain=excluded.service_or_domain,
            priority=excluded.priority,
            release_phase=excluded.release_phase,
            title=excluded.title,
            description=excluded.description,
            status=excluded.status,
            batch_id=excluded.batch_id,
            agent=excluded.agent,
            commit_sha=COALESCE(excluded.commit_sha, agent_work_items.commit_sha),
            conflict_zones_json=excluded.conflict_zones_json,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(product_area)
    .bind(service_or_domain)
    .bind(priority)
    .bind(release_phase)
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(batch_id)
    .bind(agent)
    .bind(commit_sha)
    .bind(conflict_zones_json)
    .bind(metadata_json)
    .execute(pool)
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub async fn get_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
) -> Result<AgentWorkItem, AppError> {
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work item {feature_id} not found")))
}

pub async fn list_items(
    pool: &SqlitePool,
    run_id: &str,
    status: Option<&str>,
    agent: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    match (status, agent) {
        (Some(status), Some(agent)) => {
            let status = normalize_status(status)?;
            sqlx::query_as::<_, AgentWorkItem>(
                "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
                 FROM agent_work_items WHERE run_id=? AND status=? AND agent=? ORDER BY feature_id LIMIT ? OFFSET ?",
            )
            .bind(run_id)
            .bind(status)
            .bind(agent)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
        }
        (Some(status), None) => {
            let status = normalize_status(status)?;
            sqlx::query_as::<_, AgentWorkItem>(
                "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
                 FROM agent_work_items WHERE run_id=? AND status=? ORDER BY feature_id LIMIT ? OFFSET ?",
            )
            .bind(run_id)
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
        }
        (None, Some(agent)) => sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items WHERE run_id=? AND agent=? ORDER BY feature_id LIMIT ? OFFSET ?",
        )
        .bind(run_id)
        .bind(agent)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None) => sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items WHERE run_id=? ORDER BY feature_id LIMIT ? OFFSET ?",
        )
        .bind(run_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
}

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

pub struct UpsertAgentWorkItemInput<'a> {
    pub run_id: &'a str,
    pub feature_id: &'a str,
    pub work_item_id: Option<&'a str>,
    pub product_area: &'a str,
    pub service_or_domain: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub release_phase: Option<&'a str>,
    pub title: &'a str,
    pub description: &'a str,
    pub status: Option<&'a str>,
    pub batch_id: Option<&'a str>,
    pub agent: Option<&'a str>,
    pub commit_sha: Option<&'a str>,
    pub conflict_zones: Option<Value>,
    pub metadata: Option<Value>,
}

pub async fn upsert_item(
    pool: &SqlitePool,
    input: UpsertAgentWorkItemInput<'_>,
) -> Result<AgentWorkItem, AppError> {
    let status = normalize_status(input.status.unwrap_or("pending"))?;
    let conflict_zones_json = json_array_string(input.conflict_zones)?;
    let metadata_json = json_object_string(input.metadata)?;
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
    .bind(input.run_id)
    .bind(input.feature_id)
    .bind(input.work_item_id)
    .bind(input.product_area)
    .bind(input.service_or_domain)
    .bind(input.priority)
    .bind(input.release_phase)
    .bind(input.title)
    .bind(input.description)
    .bind(status)
    .bind(input.batch_id)
    .bind(input.agent)
    .bind(input.commit_sha)
    .bind(conflict_zones_json)
    .bind(metadata_json)
    .execute(pool)
    .await?;
    get_item(pool, input.run_id, input.feature_id).await
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

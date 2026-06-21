use crate::domain::agent_work::{AgentWorkDependency, AgentWorkItem};
use crate::error::AppError;
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

pub(crate) async fn ready_item_count(pool: &SqlitePool, run_id: &str) -> Result<i64, AppError> {
    sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM agent_work_items item
         WHERE item.run_id=? AND item.status='pending'
           AND NOT EXISTS (
             SELECT 1
             FROM agent_work_dependencies dep
             LEFT JOIN agent_work_items prereq
               ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
             WHERE dep.run_id=item.run_id
               AND dep.feature_id=item.feature_id
               AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
           )",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_ready_items(
    pool: &SqlitePool,
    run_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items item
         WHERE item.run_id=? AND item.status='pending'
           AND NOT EXISTS (
             SELECT 1
             FROM agent_work_dependencies dep
             LEFT JOIN agent_work_items prereq
               ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
             WHERE dep.run_id=item.run_id
               AND dep.feature_id=item.feature_id
               AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
           )
         ORDER BY
           CASE priority WHEN 'P0' THEN 0 WHEN 'critical' THEN 0 WHEN 'P1' THEN 1 WHEN 'high' THEN 1 WHEN 'P2' THEN 2 WHEN 'medium' THEN 2 ELSE 3 END,
           feature_id
         LIMIT ? OFFSET ?",
    )
    .bind(run_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn upsert_dependency(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    depends_on_feature_id: &str,
    dependency_kind: Option<&str>,
    metadata: Option<Value>,
) -> Result<AgentWorkDependency, AppError> {
    if feature_id == depends_on_feature_id {
        return Err(AppError::Validation(
            "feature_id cannot depend on itself".to_string(),
        ));
    }
    let metadata_json = json_object_string(metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_dependencies (
            id, run_id, feature_id, depends_on_feature_id, dependency_kind, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(run_id, feature_id, depends_on_feature_id) DO UPDATE SET
            dependency_kind=excluded.dependency_kind,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .bind(dependency_kind.unwrap_or("blocks"))
    .bind(metadata_json)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentWorkDependency>(
        "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
         FROM agent_work_dependencies
         WHERE run_id=? AND feature_id=? AND depends_on_feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn delete_dependency(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    depends_on_feature_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM agent_work_dependencies
         WHERE run_id=? AND feature_id=? AND depends_on_feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_dependencies(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: Option<&str>,
) -> Result<Vec<AgentWorkDependency>, AppError> {
    if let Some(feature_id) = feature_id {
        sqlx::query_as::<_, AgentWorkDependency>(
            "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
             FROM agent_work_dependencies
             WHERE run_id=? AND feature_id=?
             ORDER BY depends_on_feature_id",
        )
        .bind(run_id)
        .bind(feature_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, AgentWorkDependency>(
            "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
             FROM agent_work_dependencies
             WHERE run_id=?
             ORDER BY feature_id, depends_on_feature_id",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

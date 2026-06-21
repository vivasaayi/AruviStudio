use crate::domain::agent_work::{AgentWorkEvent, AgentWorkEvidence};
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
    let metadata_json = json_object_string(metadata)?;
    let id = sqlx::query(
        "INSERT INTO agent_work_events (
            run_id, event_type, batch_id, feature_id, work_item_id, agent, command, status, details, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(run_id)
    .bind(event_type)
    .bind(batch_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(agent)
    .bind(command)
    .bind(status)
    .bind(details)
    .bind(metadata_json)
    .execute(pool)
    .await?
    .last_insert_rowid();
    sqlx::query_as::<_, AgentWorkEvent>(
        "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
         FROM agent_work_events WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_events(
    pool: &SqlitePool,
    run_id: &str,
    after_id: Option<i64>,
    feature_id: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvent>, AppError> {
    let limit = limit.clamp(1, 1000);
    match (after_id, feature_id) {
        (Some(after_id), Some(feature_id)) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND id>? AND feature_id=?
             ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(after_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (Some(after_id), None) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND id>? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, Some(feature_id)) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND feature_id=? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
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
    let changed_files_json = json_array_string(changed_files)?;
    let artifact_refs_json = json_array_string(artifact_refs)?;
    let metadata_json = json_object_string(metadata)?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agent_work_evidence (
            id, run_id, batch_id, feature_id, work_item_id, agent, evidence_type,
            command, exit_code, status, summary, details, changed_files_json,
            artifact_refs_json, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(run_id)
    .bind(batch_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(agent)
    .bind(evidence_type)
    .bind(command)
    .bind(exit_code)
    .bind(status)
    .bind(summary)
    .bind(details)
    .bind(changed_files_json)
    .bind(artifact_refs_json)
    .bind(metadata_json)
    .execute(pool)
    .await?;
    append_event(
        pool,
        run_id,
        "evidence",
        batch_id,
        feature_id,
        work_item_id,
        agent,
        command,
        status,
        Some(summary),
        None,
    )
    .await?;
    get_evidence(pool, &id).await
}

pub async fn get_evidence(pool: &SqlitePool, id: &str) -> Result<AgentWorkEvidence, AppError> {
    sqlx::query_as::<_, AgentWorkEvidence>(
        "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
         FROM agent_work_evidence WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work evidence {id} not found")))
}

pub async fn list_evidence(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: Option<&str>,
    batch_id: Option<&str>,
    agent: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvidence>, AppError> {
    let limit = limit.clamp(1, 1000);
    match (feature_id, batch_id, agent) {
        (Some(feature_id), _, _) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND feature_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, Some(batch_id), _) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND batch_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(batch_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None, Some(agent)) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND agent=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(agent)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None, None) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
}

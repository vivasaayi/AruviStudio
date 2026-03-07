use crate::error::AppError;
use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: String,
    pub workflow_run_id: Option<String>,
    pub agent_run_id: Option<String>,
}

pub async fn insert_log(
    pool: &SqlitePool,
    id: &str,
    level: &str,
    target: &str,
    message: &str,
    fields: &str,
    workflow_run_id: Option<&str>,
    agent_run_id: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO structured_logs (id,level,target,message,fields,workflow_run_id,agent_run_id) VALUES (?,?,?,?,?,?,?)")
        .bind(id).bind(level).bind(target).bind(message).bind(fields).bind(workflow_run_id).bind(agent_run_id)
        .execute(pool).await?;
    Ok(())
}

pub async fn get_logs(
    pool: &SqlitePool,
    level: Option<&str>,
    target: Option<&str>,
    workflow_run_id: Option<&str>,
    limit: i64,
) -> Result<Vec<LogEntry>, AppError> {
    let mut query = String::from("SELECT id,timestamp,level,target,message,fields,workflow_run_id,agent_run_id FROM structured_logs WHERE 1=1");
    if level.is_some() {
        query.push_str(" AND level = ?");
    }
    if target.is_some() {
        query.push_str(" AND target = ?");
    }
    if workflow_run_id.is_some() {
        query.push_str(" AND workflow_run_id = ?");
    }
    query.push_str(" ORDER BY timestamp DESC LIMIT ?");

    let mut q = sqlx::query_as::<_, LogEntry>(&query);
    if let Some(v) = level {
        q = q.bind(v);
    }
    if let Some(v) = target {
        q = q.bind(v);
    }
    if let Some(v) = workflow_run_id {
        q = q.bind(v);
    }
    q = q.bind(limit);
    q.fetch_all(pool).await.map_err(|e| e.into())
}

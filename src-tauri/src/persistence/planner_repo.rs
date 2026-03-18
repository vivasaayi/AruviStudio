use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlannerSessionRecord {
    pub id: String,
    pub provider_id: Option<String>,
    pub model_name: Option<String>,
    pub pending_plan_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlannerConversationEntryRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlannerChannelBindingRecord {
    pub id: String,
    pub channel: String,
    pub remote_user_id: String,
    pub remote_conversation_id: String,
    pub planner_session_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_session(
    pool: &SqlitePool,
    id: &str,
    provider_id: Option<&str>,
    model_name: Option<&str>,
) -> Result<PlannerSessionRecord, AppError> {
    sqlx::query_as::<_, PlannerSessionRecord>(
        "INSERT INTO planner_sessions (id, provider_id, model_name)
         VALUES (?, ?, ?)
         RETURNING id, provider_id, model_name, pending_plan_json, created_at, updated_at",
    )
    .bind(id)
    .bind(provider_id)
    .bind(model_name)
    .fetch_one(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> Result<PlannerSessionRecord, AppError> {
    sqlx::query_as::<_, PlannerSessionRecord>(
        "SELECT id, provider_id, model_name, pending_plan_json, created_at, updated_at
         FROM planner_sessions
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Planner session {} not found", id)))
}

pub async fn update_session(
    pool: &SqlitePool,
    id: &str,
    provider_id: Option<&str>,
    model_name: Option<&str>,
) -> Result<PlannerSessionRecord, AppError> {
    sqlx::query_as::<_, PlannerSessionRecord>(
        "UPDATE planner_sessions
         SET provider_id = ?, model_name = ?, updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, provider_id, model_name, pending_plan_json, created_at, updated_at",
    )
    .bind(provider_id)
    .bind(model_name)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn update_pending_plan(
    pool: &SqlitePool,
    id: &str,
    pending_plan_json: Option<&str>,
) -> Result<PlannerSessionRecord, AppError> {
    sqlx::query_as::<_, PlannerSessionRecord>(
        "UPDATE planner_sessions
         SET pending_plan_json = ?, updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, provider_id, model_name, pending_plan_json, created_at, updated_at",
    )
    .bind(pending_plan_json)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn append_conversation_entry(
    pool: &SqlitePool,
    id: &str,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<PlannerConversationEntryRecord, AppError> {
    sqlx::query_as::<_, PlannerConversationEntryRecord>(
        "INSERT INTO planner_conversation_entries (id, session_id, role, content)
         VALUES (?, ?, ?, ?)
         RETURNING id, session_id, role, content, created_at",
    )
    .bind(id)
    .bind(session_id)
    .bind(role)
    .bind(content)
    .fetch_one(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn list_conversation_entries(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<PlannerConversationEntryRecord>, AppError> {
    sqlx::query_as::<_, PlannerConversationEntryRecord>(
        "SELECT id, session_id, role, content, created_at
         FROM planner_conversation_entries
         WHERE session_id = ?
         ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn get_channel_binding(
    pool: &SqlitePool,
    channel: &str,
    remote_conversation_id: &str,
) -> Result<Option<PlannerChannelBindingRecord>, AppError> {
    sqlx::query_as::<_, PlannerChannelBindingRecord>(
        "SELECT id, channel, remote_user_id, remote_conversation_id, planner_session_id, created_at, updated_at
         FROM planner_channel_bindings
         WHERE channel = ? AND remote_conversation_id = ?",
    )
    .bind(channel)
    .bind(remote_conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn create_channel_binding(
    pool: &SqlitePool,
    id: &str,
    channel: &str,
    remote_user_id: &str,
    remote_conversation_id: &str,
    planner_session_id: &str,
) -> Result<PlannerChannelBindingRecord, AppError> {
    sqlx::query_as::<_, PlannerChannelBindingRecord>(
        "INSERT INTO planner_channel_bindings (
            id, channel, remote_user_id, remote_conversation_id, planner_session_id
         )
         VALUES (?, ?, ?, ?, ?)
         RETURNING id, channel, remote_user_id, remote_conversation_id, planner_session_id, created_at, updated_at",
    )
    .bind(id)
    .bind(channel)
    .bind(remote_user_id)
    .bind(remote_conversation_id)
    .bind(planner_session_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.into())
}

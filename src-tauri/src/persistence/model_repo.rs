use crate::domain::model::{ModelDefinition, ModelProvider};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};

fn row_to_model_definition(row: sqlx::sqlite::SqliteRow) -> ModelDefinition {
    ModelDefinition {
        id: row.get("id"),
        provider_id: row.get("provider_id"),
        name: row.get("name"),
        context_window: row.get("context_window"),
        capability_tags: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("capability_tags").as_str(),
        )
        .unwrap_or_default(),
        notes: row.get("notes"),
        enabled: row.get("enabled"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn create_provider(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    provider_type: &str,
    base_url: &str,
    auth_secret_ref: Option<&str>,
) -> Result<ModelProvider, AppError> {
    sqlx::query_as::<_, ModelProvider>("INSERT INTO model_providers (id,name,provider_type,base_url,auth_secret_ref,enabled,created_at,updated_at) VALUES (?,?,?,?,?,1,datetime('now'),datetime('now')) RETURNING id,name,provider_type,base_url,auth_secret_ref,enabled,created_at,updated_at")
        .bind(id).bind(name).bind(provider_type).bind(base_url).bind(auth_secret_ref)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_providers(pool: &SqlitePool) -> Result<Vec<ModelProvider>, AppError> {
    sqlx::query_as::<_, ModelProvider>("SELECT id,name,provider_type,base_url,auth_secret_ref,enabled,created_at,updated_at FROM model_providers ORDER BY name")
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn get_provider(pool: &SqlitePool, id: &str) -> Result<ModelProvider, AppError> {
    sqlx::query_as::<_, ModelProvider>("SELECT id,name,provider_type,base_url,auth_secret_ref,enabled,created_at,updated_at FROM model_providers WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Provider {id} not found")))
}

pub async fn update_provider(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    provider_type: Option<&str>,
    base_url: Option<&str>,
    auth_secret_ref: Option<&str>,
    enabled: Option<bool>,
) -> Result<ModelProvider, AppError> {
    let existing = get_provider(pool, id).await?;
    let next_name = name.unwrap_or(&existing.name);
    let existing_provider_type = existing.provider_type.as_str().to_string();
    let next_provider_type = provider_type.unwrap_or(&existing_provider_type);
    let next_base_url = base_url.unwrap_or(&existing.base_url);
    let next_auth_secret_ref = auth_secret_ref.or(existing.auth_secret_ref.as_deref());
    let next_enabled = enabled.unwrap_or(existing.enabled);

    sqlx::query_as::<_, ModelProvider>(
        "UPDATE model_providers
         SET name=?, provider_type=?, base_url=?, auth_secret_ref=?, enabled=?, updated_at=datetime('now')
         WHERE id=?
         RETURNING id,name,provider_type,base_url,auth_secret_ref,enabled,created_at,updated_at",
    )
    .bind(next_name)
    .bind(next_provider_type)
    .bind(next_base_url)
    .bind(next_auth_secret_ref)
    .bind(next_enabled)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn delete_provider(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM model_providers WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_model_definition(
    pool: &SqlitePool,
    id: &str,
    provider_id: &str,
    name: &str,
    context_window: Option<i64>,
) -> Result<ModelDefinition, AppError> {
    sqlx::query("INSERT INTO model_definitions (id,provider_id,name,context_window,enabled,created_at,updated_at) VALUES (?,?,?,?,1,datetime('now'),datetime('now')) RETURNING id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at")
        .bind(id).bind(provider_id).bind(name).bind(context_window)
        .map(row_to_model_definition)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_model_definitions(pool: &SqlitePool) -> Result<Vec<ModelDefinition>, AppError> {
    sqlx::query("SELECT id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at FROM model_definitions ORDER BY name")
        .map(row_to_model_definition)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn get_model_definition(
    pool: &SqlitePool,
    id: &str,
) -> Result<ModelDefinition, AppError> {
    sqlx::query("SELECT id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at FROM model_definitions WHERE id=?")
        .bind(id)
        .map(row_to_model_definition)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Model {id} not found")))
}

pub async fn update_model_definition(
    pool: &SqlitePool,
    id: &str,
    provider_id: Option<&str>,
    name: Option<&str>,
    context_window: Option<i64>,
    enabled: Option<bool>,
) -> Result<ModelDefinition, AppError> {
    let existing = get_model_definition(pool, id).await?;
    let next_provider_id = provider_id.unwrap_or(&existing.provider_id);
    let next_name = name.unwrap_or(&existing.name);
    let next_context_window = context_window.or(existing.context_window);
    let next_enabled = enabled.unwrap_or(existing.enabled);

    sqlx::query(
        "UPDATE model_definitions
         SET provider_id=?, name=?, context_window=?, enabled=?, updated_at=datetime('now')
         WHERE id=?
         RETURNING id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at",
    )
    .bind(next_provider_id)
    .bind(next_name)
    .bind(next_context_window)
    .bind(next_enabled)
    .bind(id)
    .map(row_to_model_definition)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn delete_model_definition(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM model_definitions WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

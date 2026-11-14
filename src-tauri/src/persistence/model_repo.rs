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
    create_provider_with_protocol(pool, id, name, provider_type, base_url, auth_secret_ref, "openai_compatible").await
}

pub fn validate_provider_protocol(provider_type: &str, protocol: &str) -> Result<(), AppError> {
    match (provider_type, protocol) {
        ("openai_compatible", "openai_compatible" | "anthropic") | ("local_runtime", "openai_compatible") => Ok(()),
        _ => Err(AppError::Validation("Unsupported provider type and protocol combination".into())),
    }
}

/// Remote provider URLs must be usable by the shared client and must not put
/// credentials in URL user-info or query fields. Local runtimes use file paths.
pub fn validate_provider_base_url(provider_type: &str, base_url: &str) -> Result<(), AppError> {
    if provider_type == "local_runtime" { return Ok(()); }
    let valid = reqwest::Url::parse(base_url).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
            && url.username().is_empty() && url.password().is_none()
            && url.query().is_none() && url.fragment().is_none()
    });
    if valid { Ok(()) } else { Err(AppError::Validation("Remote provider requires an HTTP(S) base URL without credentials, query, or fragment".into())) }
}

pub fn validate_completion_profile(provider_type: &str, protocol: &str, profile: &str) -> Result<(), AppError> {
    match (provider_type, protocol, profile) {
        (_, _, "standard") | ("openai_compatible", "openai_compatible", "openai_reasoning") => Ok(()),
        _ => Err(AppError::Validation("Unsupported provider completion profile".into())),
    }
}

pub async fn create_provider_with_protocol(
    pool: &SqlitePool, id: &str, name: &str, provider_type: &str, base_url: &str,
    auth_secret_ref: Option<&str>, protocol: &str,
) -> Result<ModelProvider, AppError> {
    create_provider_configured(pool, id, name, provider_type, base_url, auth_secret_ref, protocol, "standard").await
}

pub async fn create_provider_configured(
    pool: &SqlitePool, id: &str, name: &str, provider_type: &str, base_url: &str,
    auth_secret_ref: Option<&str>, protocol: &str, completion_profile: &str,
) -> Result<ModelProvider, AppError> {
    validate_provider_protocol(provider_type, protocol)?;
    validate_completion_profile(provider_type, protocol, completion_profile)?;
    validate_provider_base_url(provider_type, base_url)?;
    sqlx::query_as::<_, ModelProvider>("INSERT INTO model_providers (id,name,provider_type,base_url,auth_secret_ref,protocol,completion_profile,enabled,created_at,updated_at) VALUES (?,?,?,?,?,?,?,1,datetime('now'),datetime('now')) RETURNING id,name,provider_type,protocol,completion_profile,base_url,auth_secret_ref,enabled,created_at,updated_at")
        .bind(id).bind(name).bind(provider_type).bind(base_url).bind(auth_secret_ref).bind(protocol).bind(completion_profile)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_providers(pool: &SqlitePool) -> Result<Vec<ModelProvider>, AppError> {
    sqlx::query_as::<_, ModelProvider>("SELECT id,name,provider_type,protocol,completion_profile,base_url,auth_secret_ref,enabled,created_at,updated_at FROM model_providers ORDER BY name")
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn get_provider(pool: &SqlitePool, id: &str) -> Result<ModelProvider, AppError> {
    sqlx::query_as::<_, ModelProvider>("SELECT id,name,provider_type,protocol,completion_profile,base_url,auth_secret_ref,enabled,created_at,updated_at FROM model_providers WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Provider {id} not found")))
}

pub async fn update_provider(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    provider_type: Option<&str>,
    protocol: Option<&str>,
    completion_profile: Option<&str>,
    base_url: Option<&str>,
    auth_secret_ref: Option<&str>,
    enabled: Option<bool>,
) -> Result<ModelProvider, AppError> {
    let existing = get_provider(pool, id).await?;
    let next_name = name.unwrap_or(&existing.name);
    let existing_provider_type = existing.provider_type.as_str().to_string();
    let next_provider_type = provider_type.unwrap_or(&existing_provider_type);
    let next_protocol = protocol.unwrap_or(&existing.protocol);
    validate_provider_protocol(next_provider_type, next_protocol)?;
    let next_profile = completion_profile.unwrap_or(&existing.completion_profile);
    validate_completion_profile(next_provider_type, next_protocol, next_profile)?;
    let next_base_url = base_url.unwrap_or(&existing.base_url);
    // Existing invalid legacy URLs can still be disabled or renamed; any new
    // URL or provider type must satisfy the current transport contract.
    if base_url.is_some() || provider_type.is_some() {
        validate_provider_base_url(next_provider_type, next_base_url)?;
    }
    let next_auth_secret_ref = auth_secret_ref.or(existing.auth_secret_ref.as_deref());
    let next_enabled = enabled.unwrap_or(existing.enabled);

    sqlx::query_as::<_, ModelProvider>(
        "UPDATE model_providers
         SET name=?, provider_type=?, protocol=?, completion_profile=?, base_url=?, auth_secret_ref=?, enabled=?, updated_at=datetime('now')
         WHERE id=?
         RETURNING id,name,provider_type,protocol,completion_profile,base_url,auth_secret_ref,enabled,created_at,updated_at",
    )
    .bind(next_name)
    .bind(next_provider_type)
    .bind(next_protocol)
    .bind(next_profile)
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
    capability_tags: Option<&str>,
    notes: Option<&str>,
) -> Result<ModelDefinition, AppError> {
    sqlx::query("INSERT INTO model_definitions (id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at) VALUES (?,?,?,?,?,?,1,datetime('now'),datetime('now')) RETURNING id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at")
        .bind(id).bind(provider_id).bind(name).bind(context_window).bind(capability_tags.unwrap_or("[]")).bind(notes.unwrap_or(""))
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
    patch: UpdateModelDefinitionPatch<'_>,
) -> Result<ModelDefinition, AppError> {
    let existing = get_model_definition(pool, patch.id).await?;
    let next_provider_id = patch.provider_id.unwrap_or(&existing.provider_id);
    let next_name = patch.name.unwrap_or(&existing.name);
    let next_context_window = patch.context_window.or(existing.context_window);
    let existing_capability_tags =
        serde_json::to_string(&existing.capability_tags).unwrap_or_else(|_| "[]".to_string());
    let next_capability_tags = patch.capability_tags.unwrap_or(&existing_capability_tags);
    let next_notes = patch.notes.unwrap_or(&existing.notes);
    let next_enabled = patch.enabled.unwrap_or(existing.enabled);

    sqlx::query(
        "UPDATE model_definitions
         SET provider_id=?, name=?, context_window=?, capability_tags=?, notes=?, enabled=?, updated_at=datetime('now')
         WHERE id=?
         RETURNING id,provider_id,name,context_window,capability_tags,notes,enabled,created_at,updated_at",
    )
    .bind(next_provider_id)
    .bind(next_name)
    .bind(next_context_window)
    .bind(next_capability_tags)
    .bind(next_notes)
    .bind(next_enabled)
    .bind(patch.id)
    .map(row_to_model_definition)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub struct UpdateModelDefinitionPatch<'a> {
    pub id: &'a str,
    pub provider_id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub context_window: Option<i64>,
    pub capability_tags: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub enabled: Option<bool>,
}

pub async fn delete_model_definition(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM model_definitions WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod provider_validation_tests {
    use super::*;

    #[tokio::test]
    async fn completion_profile_persists_and_rejects_incompatible_updates() {
        let directory = tempfile::tempdir().unwrap();
        let db = crate::persistence::db::create_pool(&format!("sqlite:{}", directory.path().join("profile.db").display())).await.unwrap();
        let standard = create_provider(&db, "standard", "Standard", "openai_compatible", "http://localhost:1234/v1", None).await.unwrap();
        assert_eq!(standard.completion_profile, "standard");
        let reasoning = create_provider_configured(&db, "reasoning", "Reasoning", "openai_compatible", "https://example.invalid/v1", None, "openai_compatible", "openai_reasoning").await.unwrap();
        assert_eq!(get_provider(&db, &reasoning.id).await.unwrap().completion_profile, "openai_reasoning");
        assert!(update_provider(&db, &reasoning.id, None, None, Some("anthropic"), None, None, None, None).await.is_err());
        assert_eq!(get_provider(&db, &reasoning.id).await.unwrap().protocol, "openai_compatible");
        let changed = update_provider(&db, &reasoning.id, None, None, Some("anthropic"), Some("standard"), None, None, None).await.unwrap();
        assert_eq!(changed.protocol, "anthropic");
        assert_eq!(changed.completion_profile, "standard");
        assert!(create_provider_configured(&db, "bad", "Bad", "local_runtime", "/tmp/model", None, "openai_compatible", "openai_reasoning").await.is_err());
        db.close().await;
    }

    #[tokio::test]
    async fn remote_urls_are_validated_without_breaking_local_paths_or_legacy_edits() {
        let directory = tempfile::tempdir().unwrap();
        let db = crate::persistence::db::create_pool(&format!("sqlite:{}", directory.path().join("provider.db").display())).await.unwrap();
        for url in ["http://user:secret@example.invalid/v1", "https://example.invalid/v1?key=secret", "https://example.invalid/v1#secret", "file:///tmp/model", "not a url"] {
            let result = create_provider(&db, "invalid", "Invalid", "openai_compatible", url, None).await;
            assert!(matches!(result, Err(AppError::Validation(message)) if !message.contains("secret")), "{url}");
        }
        let provider = create_provider(&db, "remote", "Remote", "openai_compatible", "http://127.0.0.1:1234/v1", None).await.unwrap();
        assert!(update_provider(&db, &provider.id, None, None, None, None, Some("http://localhost/v1?key=secret"), None, None).await.is_err());
        assert_eq!(get_provider(&db, &provider.id).await.unwrap().base_url, provider.base_url);
        sqlx::query("UPDATE model_providers SET base_url='http://localhost/v1?legacy=1' WHERE id=?").bind(&provider.id).execute(&db).await.unwrap();
        let renamed = update_provider(&db, &provider.id, Some("Renamed"), None, None, None, None, None, Some(false)).await.unwrap();
        assert_eq!(renamed.name, "Renamed");
        assert!(!renamed.enabled);
        assert_eq!(renamed.base_url, "http://localhost/v1?legacy=1");
        let corrected = update_provider(&db, &provider.id, None, None, None, None, Some("https://example.invalid/v1"), None, None).await.unwrap();
        assert_eq!(corrected.base_url, "https://example.invalid/v1");
        let local = create_provider(&db, "local", "Local", "local_runtime", "/tmp/whisper-model.bin", None).await.unwrap();
        assert_eq!(local.base_url, "/tmp/whisper-model.bin");
        db.close().await;
    }
}

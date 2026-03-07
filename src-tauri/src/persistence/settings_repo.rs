use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, AppError> {
    let row = sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO settings (key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')")
        .bind(key).bind(value).execute(pool).await?;
    Ok(())
}

pub async fn get_bool_setting(
    pool: &SqlitePool,
    key: &str,
    default: bool,
) -> Result<bool, AppError> {
    let value = get_setting(pool, key).await?;
    let parsed = value
        .as_deref()
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .and_then(|candidate| match candidate.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        });
    Ok(parsed.unwrap_or(default))
}

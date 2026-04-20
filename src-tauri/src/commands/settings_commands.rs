use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::services::webhook_service::{self, McpBridgeStatus, MobileBridgeStatus};
use crate::state::AppState;
use serde::Serialize;
use sqlx::FromRow;
use sqlx::Row;
use tauri::State;

#[derive(Debug, Serialize, FromRow)]
pub struct MigrationStatus {
    pub version: i64,
    pub description: String,
    pub success: bool,
    pub installed_on: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealth {
    pub applied_migrations: usize,
    pub latest_version: Option<i64>,
    pub migrations: Vec<MigrationStatus>,
}

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, AppError> {
    settings_repo::get_setting(&state.db, &key).await
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    settings_repo::set_setting(&state.db, &key, &value).await
}

#[tauri::command]
pub async fn get_mobile_bridge_status(
    state: State<'_, AppState>,
) -> Result<MobileBridgeStatus, AppError> {
    webhook_service::resolve_mobile_bridge_status(&state)
        .await
        .map_err(AppError::Internal)
}

#[tauri::command]
pub async fn get_mcp_bridge_status(
    state: State<'_, AppState>,
) -> Result<McpBridgeStatus, AppError> {
    webhook_service::resolve_mcp_bridge_status(&state)
        .await
        .map_err(AppError::Internal)
}

#[tauri::command]
pub async fn get_database_health(state: State<'_, AppState>) -> Result<DatabaseHealth, AppError> {
    let migrations = sqlx::query_as::<_, MigrationStatus>(
        "SELECT version, description, success, datetime(installed_on, 'unixepoch') AS installed_on
         FROM _sqlx_migrations
         ORDER BY version ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let latest_version = migrations.last().map(|migration| migration.version);

    Ok(DatabaseHealth {
        applied_migrations: migrations.len(),
        latest_version,
        migrations,
    })
}

#[tauri::command]
pub async fn get_active_database_path(state: State<'_, AppState>) -> Result<String, AppError> {
    let rows = sqlx::query("PRAGMA database_list")
        .fetch_all(&state.db)
        .await?;
    let main_path = rows
        .iter()
        .find(|row| row.get::<String, _>("name") == "main")
        .map(|row| row.get::<String, _>("file"))
        .ok_or_else(|| {
            AppError::Internal("Unable to resolve active SQLite database path".to_string())
        })?;
    Ok(main_path)
}

#[tauri::command]
pub async fn get_database_path_override(
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let override_path = state.app_data_dir.join("db_override_path.txt");
    let value = match std::fs::read_to_string(&override_path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(value)
}

#[tauri::command]
pub async fn set_database_path_override(
    state: State<'_, AppState>,
    db_path: String,
) -> Result<(), AppError> {
    let trimmed = db_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Database path cannot be empty".to_string(),
        ));
    }
    if !std::path::Path::new(trimmed).is_absolute() {
        return Err(AppError::Validation(
            "Database path must be an absolute path".to_string(),
        ));
    }

    let override_path = state.app_data_dir.join("db_override_path.txt");
    std::fs::write(override_path, trimmed)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_database_path_override(state: State<'_, AppState>) -> Result<(), AppError> {
    let override_path = state.app_data_dir.join("db_override_path.txt");
    match std::fs::remove_file(override_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

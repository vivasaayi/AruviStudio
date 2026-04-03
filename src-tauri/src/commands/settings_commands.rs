use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::services::webhook_service;
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

#[derive(Debug, Serialize)]
pub struct MobileBridgeStatus {
    pub bind_host: String,
    pub bind_port: u16,
    pub host_source: String,
    pub port_source: String,
    pub bind_scope: String,
    pub detected_lan_ip: Option<String>,
    pub desktop_base_url: String,
    pub phone_base_url: Option<String>,
    pub lan_ready: bool,
    pub bind_changes_require_restart: bool,
    pub env_overrides_settings: bool,
    pub guidance: String,
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
    let bind_config = webhook_service::resolve_webhook_bind_config(&state)
        .await
        .map_err(AppError::Internal)?;
    let detected_lan_ip = webhook_service::detect_primary_lan_ip();
    let desktop_base_url =
        webhook_service::build_desktop_base_url(&bind_config.host, bind_config.port);
    let phone_base_url = webhook_service::build_phone_base_url(
        &bind_config.host,
        bind_config.port,
        detected_lan_ip.as_deref(),
    );
    let bind_scope = webhook_service::classify_bind_scope(&bind_config.host).to_string();
    let lan_ready = phone_base_url.is_some();
    let guidance = if lan_ready {
        "Use the phone base URL from the same Wi-Fi network. Bind host or port changes apply on next app launch.".to_string()
    } else if let Some(lan_ip) = &detected_lan_ip {
        format!(
            "This bridge is currently localhost-only. Set mobile.bind_host to 0.0.0.0 and restart, then connect the iPhone to http://{}:{}.",
            lan_ip, bind_config.port
        )
    } else {
        "This bridge is currently localhost-only. Set mobile.bind_host to 0.0.0.0 and restart to enable same-LAN iPhone access.".to_string()
    };

    Ok(MobileBridgeStatus {
        bind_host: bind_config.host.clone(),
        bind_port: bind_config.port,
        host_source: bind_config.host_source.clone(),
        port_source: bind_config.port_source.clone(),
        bind_scope,
        detected_lan_ip,
        desktop_base_url,
        phone_base_url,
        lan_ready,
        bind_changes_require_restart: true,
        env_overrides_settings: bind_config.host_source == "env" || bind_config.port_source == "env",
        guidance,
    })
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

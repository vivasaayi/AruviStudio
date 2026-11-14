use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::services::webhook_service::{self, McpBridgeStatus, MobileBridgeStatus};
use crate::state::AppState;
use tauri::State;

use crate::services::settings_database_service::DatabaseHealth;

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
    crate::services::settings_database_service::get_database_health(&state).await
}

#[tauri::command]
pub async fn get_active_database_path(state: State<'_, AppState>) -> Result<String, AppError> {
    crate::services::settings_database_service::get_active_database_path(&state).await
}

#[tauri::command]
pub async fn get_database_path_override(state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    crate::services::settings_database_service::get_database_path_override(&state).await
}

#[tauri::command]
pub async fn set_database_path_override(state: State<'_, AppState>, db_path: String) -> Result<(), AppError> {
    crate::services::settings_database_service::set_database_path_override(&state, db_path).await
}

#[tauri::command]
pub async fn clear_database_path_override(state: State<'_, AppState>) -> Result<(), AppError> {
    crate::services::settings_database_service::clear_database_path_override(&state).await
}

#[cfg(test)]
mod tests;

use crate::commands::settings_commands::{DatabaseHealth, MigrationStatus};
use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::services::webhook_service;
use crate::state::AppState;
use serde_json::{json, Value};
use sqlx::Row;
use std::path::Path;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "get_setting" => action_result(
            "get_setting",
            json!({
                "key": args.required_string(&["key"], "key")?,
                "value": settings_repo::get_setting(&state.db, &args.required_string(&["key"], "key")?).await?
            }),
        ),
        "set_setting" => {
            settings_repo::set_setting(
                &state.db,
                &args.required_string(&["key"], "key")?,
                &args.required_string(&["value"], "value")?,
            )
            .await?;
            Ok(action_ok("set_setting"))
        }
        "get_mobile_bridge_status" => action_result(
            "get_mobile_bridge_status",
            webhook_service::resolve_mobile_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_mcp_bridge_status" => action_result(
            "get_mcp_bridge_status",
            webhook_service::resolve_mcp_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_database_health" => {
            let migrations = sqlx::query_as::<_, MigrationStatus>(
                "SELECT version, description, success, datetime(installed_on, 'unixepoch') AS installed_on
                 FROM _sqlx_migrations
                 ORDER BY version ASC",
            )
            .fetch_all(&state.db)
            .await?;
            let latest_version = migrations.last().map(|migration| migration.version);
            action_result(
                "get_database_health",
                DatabaseHealth {
                    applied_migrations: migrations.len(),
                    latest_version,
                    migrations,
                },
            )
        }
        "get_active_database_path" => {
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
            action_result("get_active_database_path", json!({ "path": main_path }))
        }
        "get_database_path_override" => {
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
            action_result("get_database_path_override", json!({ "path": value }))
        }
        "set_database_path_override" => {
            let db_path = args.required_string(&["db_path", "dbPath"], "db_path")?;
            if !Path::new(&db_path).is_absolute() {
                return Err(AppError::Validation(
                    "Database path must be an absolute path".to_string(),
                ));
            }
            std::fs::write(
                state.app_data_dir.join("db_override_path.txt"),
                db_path.trim(),
            )?;
            Ok(action_ok("set_database_path_override"))
        }
        "clear_database_path_override" => {
            let override_path = state.app_data_dir.join("db_override_path.txt");
            match std::fs::remove_file(override_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Ok(action_ok("clear_database_path_override"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_settings action: {other}"
        ))),
    }
}

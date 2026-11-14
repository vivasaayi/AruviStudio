use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::services::{settings_database_service as database, webhook_service};
use crate::state::AppState;
use serde_json::{json, Value};

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
        "get_database_health" => action_result("get_database_health", database::get_database_health(state).await?),
        "get_active_database_path" => action_result("get_active_database_path", json!({ "path": database::get_active_database_path(state).await? })),
        "get_database_path_override" => action_result("get_database_path_override", json!({ "path": database::get_database_path_override(state).await? })),
        "set_database_path_override" => {
            database::set_database_path_override(state, args.required_string(&["db_path", "dbPath"], "db_path")?).await?;
            Ok(action_ok("set_database_path_override"))
        }
        "clear_database_path_override" => {
            database::clear_database_path_override(state).await?;
            Ok(action_ok("clear_database_path_override"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_settings action: {other}"
        ))),
    }
}

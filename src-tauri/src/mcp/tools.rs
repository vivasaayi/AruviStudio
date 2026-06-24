use crate::error::AppError;
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};

mod action_args;
mod agent_work;
mod agent_work_lifecycle;
mod agents;
mod catalog;
mod channels;
mod checkpoints;
mod definitions;
mod feature_context;
mod first_class_definitions;
mod models;
mod planner;
mod repositories;
mod repository_helpers;
mod settings;
mod speech;
mod translation;
mod work_items;
mod workflows;

use definitions::legacy_tool_definitions;
pub use definitions::ToolDefinition;
use translation::{is_legacy_tool_name, translate_first_class_tool};

pub fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = legacy_tool_definitions();
    definitions.extend(first_class_definitions::definitions());
    definitions
}

pub async fn dispatch_tool(
    state: &AppState,
    tool_name: &str,
    payload: Value,
) -> Result<Value, AppError> {
    if is_legacy_tool_name(tool_name) {
        return dispatch_namespace_tool(state, tool_name, payload).await;
    }

    if let Some((namespace_tool, adapted_payload)) = translate_first_class_tool(tool_name, payload)?
    {
        return dispatch_namespace_tool(state, namespace_tool, adapted_payload).await;
    }

    Err(AppError::Validation(format!(
        "Unknown MCP tool: {tool_name}"
    )))
}

async fn dispatch_namespace_tool(
    state: &AppState,
    tool_name: &str,
    payload: Value,
) -> Result<Value, AppError> {
    match tool_name {
        "aruvi_catalog" => catalog::handle(state, payload).await,
        "aruvi_work_items" => work_items::handle(state, payload).await,
        "aruvi_repositories" => repositories::handle(state, payload).await,
        "aruvi_planner" => planner::handle(state, payload).await,
        "aruvi_workflows" => workflows::handle(state, payload).await,
        "aruvi_checkpoints" => checkpoints::handle(state, payload).await,
        "aruvi_agents" => agents::handle(state, payload).await,
        "aruvi_agent_work" => agent_work::handle(state, payload).await,
        "aruvi_models" => models::handle(state, payload).await,
        "aruvi_settings" => settings::handle(state, payload).await,
        "aruvi_channels" => channels::handle(state, payload).await,
        "aruvi_speech" => speech::handle(state, payload).await,
        _ => Err(AppError::Validation(format!(
            "Unknown MCP namespace tool: {tool_name}"
        ))),
    }
}

fn action_result<T: Serialize>(action: &str, result: T) -> Result<Value, AppError> {
    Ok(json!({
        "action": action,
        "result": serde_json::to_value(result)?
    }))
}

fn action_ok(action: &str) -> Value {
    json!({
        "action": action,
        "result": { "ok": true }
    })
}

#[cfg(test)]
mod tests;

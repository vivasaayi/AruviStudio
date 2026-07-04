use crate::mcp;
use crate::state::AppState;
use serde_json::{json, Value};

pub(crate) async fn execute_mobile_planner_mcp_tool(
    state: &AppState,
    step: u8,
    tool_name: &str,
    arguments: Value,
) -> Result<Value, String> {
    if !is_mobile_planner_tool_allowed(tool_name) {
        return Err(format!(
            "Tool is not allowed in mobile planner chat: {tool_name}"
        ));
    }
    let response = mcp::handle_json_rpc_value(
        state,
        json!({
            "jsonrpc": "2.0",
            "id": format!("mobile-planner-chat-{step}"),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments,
            }
        }),
    )
    .await
    .ok_or_else(|| "MCP tool did not return a response.".to_string())?;

    if let Some(error) = response.get("error") {
        return Err(error.to_string());
    }
    let result = response
        .get("result")
        .cloned()
        .ok_or_else(|| "MCP response is missing result.".to_string())?;
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(render_mcp_tool_text(&result).unwrap_or_else(|| result.to_string()));
    }
    Ok(result.get("structuredContent").cloned().unwrap_or(result))
}

fn is_mobile_planner_tool_allowed(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "catalog.products.list"
            | "catalog.products.get"
            | "catalog.products.get_tree"
            | "catalog.product_areas.list"
            | "catalog.product_areas.create"
            | "catalog.product_areas.update"
            | "catalog.product_areas.reorder"
            | "catalog.capabilities.list"
            | "catalog.capabilities.create"
            | "catalog.capabilities.update"
            | "catalog.capabilities.reorder"
            | "catalog.capabilities.apply_template"
            | "catalog.capabilities.convert_kind"
            | "work_items.list"
            | "work_items.get"
            | "work_items.create"
            | "work_items.stories.create"
            | "work_items.tasks.create"
            | "work_items.update"
            | "work_items.list_children"
            | "work_items.summarize_by_product"
            | "repositories.list"
            | "repositories.resolution.for_scope"
            | "repositories.resolution.for_work_item"
            | "repositories.trees.list"
            | "repositories.files.read"
    )
}

fn render_mcp_tool_text(result: &Value) -> Option<String> {
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

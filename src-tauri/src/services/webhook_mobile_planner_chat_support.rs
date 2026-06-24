use crate::persistence::{planner_repo, product_repo};
use crate::state::AppState;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MobilePlannerToolTraceEntry {
    pub(crate) step: u8,
    pub(crate) tool_name: String,
    pub(crate) arguments: Value,
    pub(crate) result: Option<Value>,
    pub(crate) error: Option<String>,
}

pub(crate) fn build_mobile_planner_system_prompt(
    product_id: Option<&str>,
    product_name: Option<&str>,
) -> String {
    let product_context = product_id
        .map(|id| {
            format!(
                "Current selected product: {} ({id})\n",
                product_name.unwrap_or("unknown")
            )
        })
        .unwrap_or_else(|| {
            "No product is selected yet. Use catalog.products.list if needed.\n".to_string()
        });
    format!(
        "You are Aruvi Studio's first-class mobile planner.\n\
{product_context}\
Use MCP tools to inspect or update the product plan when the user asks for planning work. \
Prefer the selected product when one is provided. Keep replies short enough for mobile.\n\
Use the canonical hierarchy Product > Product Area > Capability > Feature, then Story > Task for delivery. \
Do not describe product areas as product_areas.\n\
\n\
Allowed MCP tools:\n\
- catalog.products.list, catalog.products.get, catalog.products.get_tree\n\
- catalog.product_areas.list, catalog.product_areas.create, catalog.product_areas.update, catalog.product_areas.reorder\n\
- catalog.capabilities.list, catalog.capabilities.create, catalog.capabilities.update, catalog.capabilities.reorder, catalog.capabilities.apply_template, catalog.capabilities.convert_kind\n\
- work_items.list, work_items.get, work_items.create, work_items.stories.create, work_items.tasks.create, work_items.update, work_items.list_children, work_items.summarize_by_product\n\
- repositories.list, repositories.resolution.for_scope, repositories.resolution.for_work_item, repositories.trees.list, repositories.files.read\n\
\n\
Return exactly one JSON object, with no markdown.\n\
To call a tool: {{\"type\":\"tool_call\",\"tool\":\"catalog.products.get_tree\",\"arguments\":{{\"productId\":\"...\"}},\"reason\":\"...\"}}\n\
To answer: {{\"type\":\"final\",\"message\":\"...\"}}\n\
Final message style: be natural and explicit. If you changed data, say what changed, name the created/updated items, say where they were added, and end with a short follow-up invitation such as \"Want me to split any of these further?\".\n\
Before creating hundreds of nodes, inspect the existing tree and create a small useful slice unless the user explicitly asks for a broad commit. \
If you mutate catalog or work items, mention the exact objects changed in the final message."
    )
}

pub(crate) async fn resolve_mobile_planner_product_context(
    state: &AppState,
    requested_product_id: Option<&str>,
    current_product_id: Option<&str>,
    user_input: Option<&str>,
) -> Result<Option<crate::domain::product::Product>, String> {
    if let Some(product_id) = requested_product_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return product_repo::get_product(&state.db, product_id)
            .await
            .map(Some)
            .map_err(|error| error.to_string());
    }

    let products = product_repo::list_products(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(input) = user_input.map(normalize_product_match_text) {
        let mut matches = products
            .iter()
            .filter(|product| {
                let product_name = normalize_product_match_text(&product.name);
                !product_name.is_empty() && input.contains(&product_name)
            })
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.name.len().cmp(&left.name.len()));
        if let Some(product) = matches.into_iter().next() {
            return Ok(Some(product));
        }
    }

    if let Some(product_id) = current_product_id {
        if let Some(product) = products
            .into_iter()
            .find(|product| product.id == product_id)
        {
            return Ok(Some(product));
        }
    }

    Ok(None)
}

fn normalize_product_match_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) async fn persist_mobile_planner_assistant_message(
    state: &AppState,
    session_id: &str,
    content: &str,
) -> Result<(), String> {
    planner_repo::append_mobile_planner_chat_message(
        &state.db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        "assistant",
        content,
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

pub(crate) async fn persist_mobile_planner_tool_trace(
    state: &AppState,
    session_id: &str,
    trace: &MobilePlannerToolTraceEntry,
) -> Result<(), String> {
    let arguments_json =
        serde_json::to_string(&trace.arguments).map_err(|error| error.to_string())?;
    let result_json = trace
        .result
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    planner_repo::append_mobile_planner_chat_tool_trace(
        &state.db,
        planner_repo::AppendMobilePlannerChatToolTraceInput {
            id: &trace_id,
            session_id,
            step: i64::from(trace.step),
            tool_name: &trace.tool_name,
            arguments_json: &arguments_json,
            result_json: result_json.as_deref(),
            error: trace.error.as_deref(),
        },
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

pub(crate) fn truncate_for_prompt(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for ch in value.chars().take(max_chars) {
        output.push(ch);
    }
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

pub(crate) fn non_zero_token_count(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

pub(crate) fn extract_json_payload(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(trimmed[start..=end].to_string())
}

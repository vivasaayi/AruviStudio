use crate::error::AppError;
use crate::services::planner_service::PlannerPlan;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlannerToolCall {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) tool: String,
    pub(crate) arguments: Option<Value>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PlannerFinalResponse {
    #[serde(rename = "type")]
    _kind: Option<String>,
    assistant_response: String,
    needs_confirmation: bool,
    clarification_question: Option<String>,
    actions: Vec<Value>,
}

fn extract_json_object(raw: &str) -> Result<String, AppError> {
    let without_fences = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();
    if without_fences.starts_with('{') && without_fences.ends_with('}') {
        return Ok(without_fences);
    }

    let bytes = without_fences.as_bytes();
    let mut depth = 0_i32;
    let mut start: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        let ch = *byte as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '{' {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                if let Some(start_idx) = start {
                    return Ok(without_fences[start_idx..=index].to_string());
                }
            }
        }
    }
    Err(AppError::Validation(
        "Planner model did not return JSON".to_string(),
    ))
}

pub(crate) fn parse_final_response(raw: &str) -> Result<PlannerPlan, AppError> {
    let object = extract_json_object(raw)?;
    let parsed: PlannerFinalResponse = serde_json::from_str(&object)?;
    let actions = parsed
        .actions
        .into_iter()
        .filter_map(normalize_planner_action)
        .collect::<Vec<_>>();
    Ok(PlannerPlan {
        assistant_response: parsed.assistant_response,
        needs_confirmation: parsed.needs_confirmation,
        clarification_question: parsed.clarification_question,
        actions,
    })
}

pub(crate) fn parse_agent_turn(
    raw: &str,
) -> Result<Result<PlannerToolCall, PlannerPlan>, AppError> {
    let object = extract_json_object(raw)?;
    let value: Value = serde_json::from_str(&object)?;
    if value.get("type").and_then(Value::as_str) == Some("tool_call") {
        let tool_call: PlannerToolCall = serde_json::from_value(value)?;
        return Ok(Ok(tool_call));
    }
    Ok(Err(parse_final_response(&object)?))
}

fn normalized_target_string(action: &Value, key: &str) -> Option<String> {
    action.get("target").and_then(|target| match target {
        Value::Object(map) => map
            .get(key)
            .and_then(Value::as_str)
            .map(ToString::to_string),
        _ => None,
    })
}

fn target_as_string(action: &Value) -> Option<String> {
    action
        .get("target")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn alternate_string_field(action: &Value, key: &str) -> Option<String> {
    action
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn normalize_planner_action(action: Value) -> Option<Value> {
    let action_type = action.get("type").and_then(Value::as_str)?.to_string();
    let raw_target_name = target_as_string(&action);
    let target_product_name = normalized_target_string(&action, "productName");
    let target_product_area_name = normalized_target_string(&action, "productAreaName")
        .or_else(|| alternate_string_field(&action, "product_area_name"));
    let target_capability_name = normalized_target_string(&action, "capabilityName")
        .or_else(|| alternate_string_field(&action, "capability_name"));
    let target_work_item_title = normalized_target_string(&action, "workItemTitle")
        .or_else(|| alternate_string_field(&action, "work_item_title"))
        .or_else(|| alternate_string_field(&action, "work_item_name"));
    let mut action = action;
    let object = action.as_object_mut()?;

    match action_type.as_str() {
        "create_product" => {
            if let Some(target_name) = raw_target_name.clone() {
                if let Some(Value::String(_)) = object.get("target") {
                    object.insert(
                        "target".to_string(),
                        json!({ "productName": target_name.clone() }),
                    );
                }
            }
            if !object.contains_key("name") {
                if let Some(name) = target_product_name.or(raw_target_name) {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
        }
        "create_product_area" => {
            if let Some(target_name) = raw_target_name.clone() {
                if let Some(Value::String(_)) = object.get("target") {
                    object.insert("target".to_string(), json!({ "productName": target_name }));
                }
            }
            if !object.contains_key("name") {
                if let Some(name) = target_product_area_name {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
            if !object.contains_key("productAreaName") {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    object.insert(
                        "productAreaName".to_string(),
                        Value::String(name.to_string()),
                    );
                }
            }
        }
        "create_capability" => {
            if let Some(target_name) = raw_target_name.clone() {
                if let Some(Value::String(_)) = object.get("target") {
                    object.insert(
                        "target".to_string(),
                        json!({ "productAreaName": target_name }),
                    );
                }
            }
            if !object.contains_key("name") {
                if let Some(name) = target_capability_name {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
            if !object.contains_key("capabilityName") {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    object.insert(
                        "capabilityName".to_string(),
                        Value::String(name.to_string()),
                    );
                }
            }
        }
        "create_work_item" => {
            if let Some(target_name) = raw_target_name {
                if let Some(Value::String(_)) = object.get("target") {
                    object.insert(
                        "target".to_string(),
                        json!({ "capabilityName": target_name }),
                    );
                }
            }
            if !object.contains_key("title") {
                if let Some(title) = target_work_item_title {
                    object.insert("title".to_string(), Value::String(title));
                } else if let Some(title) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                {
                    object.insert("title".to_string(), Value::String(title));
                }
            }
        }
        _ => {}
    }

    Some(action)
}

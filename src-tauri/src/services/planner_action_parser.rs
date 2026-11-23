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
        .enumerate()
        .map(|(index, action)| {
            normalize_planner_action(action)
                .filter(|action| {
                    action
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| !kind.trim().is_empty())
                })
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "Planner action {} has an invalid type or conflicting fields",
                        index + 1
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut has_draft = false;
    let mut has_operation = false;
    let mut has_status_report = false;
    for (index, action) in actions.iter().enumerate() {
        let kind = action
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match kind {
            "create_product"
            | "update_product"
            | "archive_product"
            | "create_product_area"
            | "update_product_area"
            | "delete_product_area"
            | "create_capability"
            | "update_capability"
            | "delete_capability"
            | "apply_capability_template"
            | "convert_capability_kind"
            | "create_work_item"
            | "update_work_item"
            | "delete_work_item" => has_draft = true,
            "approve_work_item"
            | "reject_work_item"
            | "approve_work_item_plan"
            | "reject_work_item_plan"
            | "approve_work_item_test_review"
            | "start_workflow"
            | "workflow_action" => has_operation = true,
            "report_status" => has_status_report = true,
            "report_tree" => {}
            _ => {
                return Err(AppError::Validation(format!(
                    "Planner action {} has unsupported type {}",
                    index + 1,
                    kind
                )))
            }
        }
    }
    if has_draft && (has_operation || has_status_report) {
        return Err(AppError::Validation(
            "Planner cannot combine staged design changes with workflow actions or status reports in one plan".into(),
        ));
    }
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
    let mut action = action;
    if matches!(action_type.as_str(), "create_product_area" | "create_capability" | "create_work_item") {
        // Some compatible models place create fields under `details`. Promote
        // only supported content fields, never identifiers or parent links.
        let groups: &[&[&str]] = &[
            &["name"], &["title"], &["description"], &["purpose"], &["nodeKind", "node_kind"],
            &["acceptanceCriteria", "acceptance_criteria"], &["technicalNotes", "technical_notes"],
            &["problemStatement", "problem_statement"], &["constraints"], &["workItemType", "work_item_type"],
            &["priority"], &["risk"], &["complexity"], &["explanation"], &["examples"],
            &["implementationNotes", "implementation_notes"], &["testGuidance", "test_guidance"],
        ];
        let object = action.as_object_mut()?;
        let details = match object.get("details") {
            None | Some(Value::Null) => None,
            Some(Value::Object(details)) => Some(details.clone()),
            Some(_) => return None,
        };
        if let Some(details) = details {
            for aliases in groups {
                let mut chosen: Option<Value> = None;
                for key in *aliases {
                    for value in [object.get(*key), details.get(*key)].into_iter().flatten() {
                        if value.is_null() { continue; }
                        let normalized = if aliases[0] == "examples" && value.is_array() {
                            let examples: Option<Vec<&str>> = value.as_array()?.iter().map(Value::as_str).collect();
                            Value::String(examples?.join("\n"))
                        } else if value.is_string() { value.clone() } else { return None };
                        if chosen.as_ref().is_some_and(|existing| existing != &normalized) { return None; }
                        chosen = Some(normalized);
                    }
                }
                if let Some(value) = chosen { object.insert(aliases[0].to_string(), value); }
            }
        }
    }
    if matches!(action_type.as_str(), "update_product" | "update_product_area" | "update_capability" | "update_work_item") {
        // Promote only fields the update handlers actually apply. Model output
        // may put them in fields, details, or at the top level; conflicts and
        // empty updates must fail rather than silently report success.
        let groups: &[&[&str]] = match action_type.as_str() {
            "update_product" => &[&["name"], &["description"], &["vision"], &["goals"], &["tags"]],
            "update_product_area" => &[&["name"], &["description"], &["purpose"], &["nodeKind", "node_kind"], &["explanation"], &["examples"], &["implementationNotes", "implementation_notes"], &["testGuidance", "test_guidance"]],
            "update_capability" => &[&["name"], &["description"], &["acceptanceCriteria", "acceptance_criteria"], &["priority"], &["risk"], &["technicalNotes", "technical_notes"], &["nodeKind", "node_kind"], &["explanation"], &["examples"], &["implementationNotes", "implementation_notes"], &["testGuidance", "test_guidance"]],
            _ => &[&["title"], &["description"], &["problemStatement", "problem_statement"], &["acceptanceCriteria", "acceptance_criteria"], &["constraints"], &["status"]],
        };
        let object = action.as_object_mut()?;
        let fields = match object.get("fields") { None | Some(Value::Null) => serde_json::Map::new(), Some(Value::Object(map)) => map.clone(), Some(_) => return None };
        let details = match object.get("details") { None | Some(Value::Null) => serde_json::Map::new(), Some(Value::Object(map)) => map.clone(), Some(_) => return None };
        let is_supported = |key: &str| groups.iter().any(|aliases| aliases.contains(&key));
        if fields.keys().any(|key| !is_supported(key)) || details.keys().any(|key| !is_supported(key) && !matches!(key.as_str(), "id" | "node_id" | "parent_id" | "node_type" | "type" | "_catalog_binding")) {
            return None;
        }
        let mut normalized = fields.clone();
        let mut found = false;
        for aliases in groups {
            let mut chosen: Option<Value> = None;
            for key in *aliases {
                for value in [object.get(*key), fields.get(*key), details.get(*key)].into_iter().flatten() {
                    if value.is_null() { continue; }
                    let value = if matches!(aliases[0], "goals" | "tags") {
                        let values = value.as_array()?;
                        if values.iter().any(|item| !item.is_string()) { return None; }
                        value.clone()
                    } else if aliases[0] == "examples" && value.is_array() {
                        let examples: Option<Vec<&str>> = value.as_array()?.iter().map(Value::as_str).collect();
                        Value::String(examples?.join("\n"))
                    } else if value.is_string() { value.clone() } else { return None };
                    if chosen.as_ref().is_some_and(|existing| existing != &value) { return None; }
                    chosen = Some(value);
                }
            }
            if let Some(value) = chosen { found = true; normalized.insert(aliases[0].into(), value); }
        }
        if !found { return None; }
        object.insert("fields".into(), Value::Object(normalized));
    }
    let raw_target_name = target_as_string(&action);
    let target_product_name = normalized_target_string(&action, "productName");
    let target_product_area_name = normalized_target_string(&action, "productAreaName")
        .or_else(|| alternate_string_field(&action, "product_area_name"));
    let target_capability_name = normalized_target_string(&action, "capabilityName")
        .or_else(|| alternate_string_field(&action, "capability_name"));
    let target_work_item_title = normalized_target_string(&action, "workItemTitle")
        .or_else(|| alternate_string_field(&action, "work_item_title"))
        .or_else(|| alternate_string_field(&action, "work_item_name"));
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

#[cfg(test)]
mod malformed_response_tests {
    use super::*;
    #[test]
    fn promotes_supported_nested_create_details_without_trusting_parent_ids() {
        let raw = json!({"assistant_response":"Ready", "needs_confirmation":true, "actions":[{
            "type":"create_capability", "details":{"name":"Stock Tracking", "nodeKind":"capability", "technical_notes":"Use stock events", "parent_id":"untrusted", "explanation":"Track inventory", "examples":["First example", "Second example"]}
        }]}).to_string();
        let plan = parse_final_response(&raw).unwrap();
        assert_eq!(plan.actions[0]["name"], "Stock Tracking");
        assert_eq!(plan.actions[0]["nodeKind"], "capability");
        assert_eq!(plan.actions[0]["technicalNotes"], "Use stock events");
        assert_eq!(plan.actions[0]["explanation"], "Track inventory");
        assert_eq!(plan.actions[0]["examples"], "First example\nSecond example");
        assert!(plan.actions[0].get("parent_id").is_none());
    }

    #[test]
    fn rejects_conflicting_or_invalid_nested_create_fields() {
        for action in [
            json!({"type":"create_capability", "name":"One", "details":{"name":"Other"}}),
            json!({"type":"create_capability", "nodeKind":"feature", "details":{"node_kind":"capability"}}),
            json!({"type":"create_work_item", "details":{"title":123}}),
            json!({"type":"create_product_area", "name":"Area", "details":"invalid"}),
            json!({"type":"create_capability", "details":{"examples":["valid", 2]}}),
        ] {
            let raw = json!({"assistant_response":"Ready", "needs_confirmation":true, "actions":[action]}).to_string();
            assert!(parse_final_response(&raw).is_err());
        }
    }

    #[test]
    fn normalizes_nested_work_item_updates_and_rejects_empty_or_conflicting_updates() {
        let response = |action: Value| json!({"assistant_response":"Ready", "needs_confirmation":true, "actions":[action]}).to_string();
        let plan = parse_final_response(&response(json!({"type":"update_work_item", "details":{"acceptanceCriteria":"Notify staff", "id":"untrusted"}}))).unwrap();
        assert_eq!(plan.actions[0]["fields"]["acceptanceCriteria"], "Notify staff");
        assert!(plan.actions[0]["fields"].get("id").is_none());
        for action in [json!({"type":"update_work_item", "details":{}}), json!({"type":"update_work_item", "fields":{"acceptanceCriteria":"One"}, "details":{"acceptance_criteria":"Other"}}), json!({"type":"update_work_item", "details":{"constraints":123}})] {
            assert!(parse_final_response(&response(action)).is_err());
        }
    }

    #[test]
    fn normalizes_nested_product_and_hierarchy_updates_without_silent_noops() {
        let response = |action: Value| json!({"assistant_response":"Ready", "needs_confirmation":true, "actions":[action]}).to_string();
        for action in [
            json!({"type":"update_product", "details":{"description":"New product description", "goals":["One"]}}),
            json!({"type":"update_product_area", "details":{"description":"New area description", "examples":["First","Second"]}}),
            json!({"type":"update_capability", "details":{"description":"New capability description", "technical_notes":"Notes"}}),
        ] {
            let plan = parse_final_response(&response(action)).unwrap();
            assert!(plan.actions[0]["fields"]["description"].is_string());
        }
        for action in [json!({"type":"update_product", "details":{"goals":["ok",3]}}), json!({"type":"update_product_area", "details":{"ignored":"value"}}), json!({"type":"update_capability", "fields":{"description":"One"}, "details":{"description":"Two"}}), json!({"type":"update_product", "details":{"description":"New", "lifecycle":"active"}})] {
            assert!(parse_final_response(&response(action)).is_err());
        }
    }

    #[test]
    fn rejects_partially_malformed_actions_instead_of_dropping_them() {
        let raw = json!({"assistant_response":"Ready", "needs_confirmation":false, "actions":[{"type":"create_product_area", "name":"First"}, {"node_type":"product_area", "name":"Second"}]}).to_string();
        assert!(parse_final_response(&raw).is_err());
    }
    #[test]
    fn rejects_unknown_or_mixed_workflow_actions_without_losing_supported_edits() {
        let response = |actions: Value| {
            json!({"assistant_response":"Ready", "needs_confirmation":false, "actions":actions})
                .to_string()
        };
        assert!(parse_final_response(&response(json!([{ "type":"create_product_area", "name":"Inventory" }, {"type":"invented_operation"}]))).is_err());
        assert!(parse_final_response(&response(
            json!([{ "type":"create_product_area", "name":"Inventory" }, {"type":"start_workflow"}])
        ))
        .is_err());
        assert!(parse_final_response(&response(
            json!([{ "type":"create_product_area", "name":"Inventory" }, {"type":"report_tree"}])
        ))
        .is_ok());
        assert!(parse_final_response(&response(
            json!([{ "type":"start_workflow" }, {"type":"approve_work_item"}])
        ))
        .is_ok());
    }
}

use crate::error::AppError;
use crate::services::planner_action_fields::{
    fields_string, fields_string_array, set_optional_string_value, set_string_array_value,
    set_string_value, target_field,
};
use crate::services::planner_draft::{
    find_draft_node, infer_selected_draft_context, resolve_draft_capability_node_id,
    resolve_draft_product_area_name, resolve_draft_product_name,
};
use crate::services::planner_draft_mutation::update_descendant_targets_for_rename;
use crate::services::planner_service::PlannerDraftPlan;
use serde_json::Value;

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

pub(crate) fn apply_update_draft_action(
    draft_plan: &mut PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action_type: &str,
    action: &Value,
) -> Result<bool, AppError> {
    match action_type {
        "update_product" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
            let node_index = draft_plan
                .nodes
                .iter()
                .position(|node| {
                    node.node_type == "product"
                        && node.parent_id.is_none()
                        && normalize_text(Some(&node.name)) == normalize_text(Some(&product_name))
                })
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
            let previous_name = draft_plan.nodes[node_index].name.clone();
            let next_name = fields_string(action, "name");
            let next_description = fields_string(action, "description");
            let next_vision = fields_string(action, "vision");
            let next_goals = fields_string_array(action, "goals");
            let next_tags = fields_string_array(action, "tags");
            {
                let node = &mut draft_plan.nodes[node_index];
                if let Some(name) = next_name.clone() {
                    node.name = name.clone();
                    set_string_value(&mut node.details, "name", &name);
                }
                if let Some(description) = next_description.clone() {
                    node.summary = Some(description.clone());
                    set_string_value(&mut node.details, "description", &description);
                }
                if let Some(vision) = next_vision {
                    set_string_value(&mut node.details, "vision", &vision);
                }
                if let Some(goals) = next_goals {
                    set_string_array_value(&mut node.details, "goals", &goals);
                }
                if let Some(tags) = next_tags {
                    set_string_array_value(&mut node.details, "tags", &tags);
                }
            }
            if let Some(name) = next_name {
                if normalize_text(Some(&previous_name)) != normalize_text(Some(&name)) {
                    let node_id = draft_plan.nodes[node_index].id.clone();
                    update_descendant_targets_for_rename(
                        draft_plan,
                        &node_id,
                        "product",
                        &previous_name,
                        &name,
                    );
                }
            }
        }
        "update_product_area" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft product area needs a product".to_string())
                    })?;
            let product_id = find_draft_node(draft_plan, "product", &product_name, None)
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?
                .id
                .clone();
            let product_area_name =
                resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft product area is required".to_string())
                    })?;
            let node_index = draft_plan
                .nodes
                .iter()
                .position(|node| {
                    node.node_type == "product_area"
                        && node.parent_id.as_deref() == Some(product_id.as_str())
                        && normalize_text(Some(&node.name))
                            == normalize_text(Some(&product_area_name))
                })
                .ok_or_else(|| {
                    AppError::Validation("Draft product area is required".to_string())
                })?;
            let previous_name = draft_plan.nodes[node_index].name.clone();
            let next_name = fields_string(action, "name");
            let next_description = fields_string(action, "description");
            {
                let node = &mut draft_plan.nodes[node_index];
                if let Some(name) = next_name.clone() {
                    node.name = name.clone();
                    set_string_value(&mut node.details, "name", &name);
                    set_string_value(&mut node.details, "product_area_name", &name);
                }
                if let Some(description) = next_description.clone() {
                    node.summary = Some(description.clone());
                    set_string_value(&mut node.details, "description", &description);
                }
                set_optional_string_value(
                    &mut node.details,
                    "purpose",
                    fields_string(action, "purpose"),
                );
                set_optional_string_value(
                    &mut node.details,
                    "nodeKind",
                    fields_string(action, "nodeKind")
                        .or_else(|| fields_string(action, "node_kind")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "explanation",
                    fields_string(action, "explanation"),
                );
                set_optional_string_value(
                    &mut node.details,
                    "examples",
                    fields_string(action, "examples"),
                );
                set_optional_string_value(
                    &mut node.details,
                    "implementationNotes",
                    fields_string(action, "implementationNotes")
                        .or_else(|| fields_string(action, "implementation_notes")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "testGuidance",
                    fields_string(action, "testGuidance")
                        .or_else(|| fields_string(action, "test_guidance")),
                );
            }
            if let Some(name) = next_name {
                if normalize_text(Some(&previous_name)) != normalize_text(Some(&name)) {
                    let node_id = draft_plan.nodes[node_index].id.clone();
                    update_descendant_targets_for_rename(
                        draft_plan,
                        &node_id,
                        "product_area",
                        &previous_name,
                        &name,
                    );
                }
            }
        }
        "update_capability" => {
            let capability_id =
                resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft capability is required".to_string())
                    })?;
            let node_index = draft_plan
                .nodes
                .iter()
                .position(|node| node.id == capability_id)
                .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
            let previous_name = draft_plan.nodes[node_index].name.clone();
            let next_name = fields_string(action, "name");
            let next_description = fields_string(action, "description");
            {
                let node = &mut draft_plan.nodes[node_index];
                if let Some(name) = next_name.clone() {
                    node.name = name.clone();
                    set_string_value(&mut node.details, "name", &name);
                    set_string_value(&mut node.details, "capability_name", &name);
                }
                if let Some(description) = next_description.clone() {
                    node.summary = Some(description.clone());
                    set_string_value(&mut node.details, "description", &description);
                }
                set_optional_string_value(
                    &mut node.details,
                    "acceptanceCriteria",
                    fields_string(action, "acceptanceCriteria")
                        .or_else(|| fields_string(action, "acceptance_criteria")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "priority",
                    fields_string(action, "priority"),
                );
                set_optional_string_value(&mut node.details, "risk", fields_string(action, "risk"));
                set_optional_string_value(
                    &mut node.details,
                    "technicalNotes",
                    fields_string(action, "technicalNotes")
                        .or_else(|| fields_string(action, "technical_notes")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "nodeKind",
                    fields_string(action, "nodeKind")
                        .or_else(|| fields_string(action, "node_kind")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "explanation",
                    fields_string(action, "explanation"),
                );
                set_optional_string_value(
                    &mut node.details,
                    "examples",
                    fields_string(action, "examples"),
                );
                set_optional_string_value(
                    &mut node.details,
                    "implementationNotes",
                    fields_string(action, "implementationNotes")
                        .or_else(|| fields_string(action, "implementation_notes")),
                );
                set_optional_string_value(
                    &mut node.details,
                    "testGuidance",
                    fields_string(action, "testGuidance")
                        .or_else(|| fields_string(action, "test_guidance")),
                );
            }
            if let Some(name) = next_name {
                if normalize_text(Some(&previous_name)) != normalize_text(Some(&name)) {
                    let node_id = draft_plan.nodes[node_index].id.clone();
                    update_descendant_targets_for_rename(
                        draft_plan,
                        &node_id,
                        "capability",
                        &previous_name,
                        &name,
                    );
                }
            }
        }
        "update_work_item" => {
            let (_, _, _, selected_work_item_title) =
                infer_selected_draft_context(Some(draft_plan), selected_draft_node_id);
            let title = target_field(action, "workItemTitle")
                .map(ToString::to_string)
                .or(selected_work_item_title)
                .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
            let node = draft_plan
                .nodes
                .iter_mut()
                .find(|node| {
                    node.node_type == "work_item"
                        && normalize_text(Some(&node.name)) == normalize_text(Some(&title))
                })
                .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
            if let Some(name) = fields_string(action, "title") {
                node.name = name;
                set_string_value(&mut node.details, "title", &node.name);
                set_string_value(&mut node.details, "work_item_name", &node.name);
            }
            if let Some(description) = fields_string(action, "description") {
                node.summary = Some(description.clone());
                set_string_value(&mut node.details, "description", &description);
            }
            set_optional_string_value(
                &mut node.details,
                "problemStatement",
                fields_string(action, "problemStatement")
                    .or_else(|| fields_string(action, "problem_statement")),
            );
            set_optional_string_value(
                &mut node.details,
                "acceptanceCriteria",
                fields_string(action, "acceptanceCriteria")
                    .or_else(|| fields_string(action, "acceptance_criteria")),
            );
            set_optional_string_value(
                &mut node.details,
                "constraints",
                fields_string(action, "constraints"),
            );
            set_optional_string_value(&mut node.details, "status", fields_string(action, "status"));
        }
        _ => return Ok(false),
    }
    Ok(true)
}

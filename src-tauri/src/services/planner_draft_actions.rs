use crate::domain::product::{ChildReparentStrategy, HierarchyNodeKind, SemanticTemplateKind};
use crate::error::AppError;
use crate::services::planner_action_fields::{
    copy_analysis, fields_string, fields_string_array, remove_target_value,
    set_optional_string_value, set_string_array_value, set_string_value, set_target_string_value,
    string_array_field, string_field, target_field,
};
use crate::services::planner_draft::{
    draft_node_kind, find_draft_ancestor_name, find_draft_node, find_draft_node_by_id,
    infer_selected_draft_context, node_kind_field, parse_node_kind_value,
    remove_draft_node_subtree, resolve_draft_capability_name, resolve_draft_capability_node_id,
    resolve_draft_product_area_name, resolve_draft_product_area_node_id,
    resolve_draft_product_name, update_descendant_targets_for_rename,
};
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan};
use serde_json::{json, Value};

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

pub(crate) fn apply_create_draft_action(
    draft_plan: &mut PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action_type: &str,
    action: &Value,
) -> Result<bool, AppError> {
    match action_type {
        "create_product" => {
            let name = string_field(action, "name").ok_or_else(|| {
                AppError::Validation("Draft product name is required".to_string())
            })?;
            if find_draft_node(draft_plan, "product", &name, None).is_none() {
                let description = string_field(action, "description");
                let vision = string_field(action, "vision");
                let mut details = json!({
                    "type": "create_product",
                    "name": string_field(action, "name").unwrap_or_default(),
                    "description": description,
                    "vision": vision,
                    "goals": string_array_field(action, "goals"),
                    "tags": string_array_field(action, "tags"),
                });
                copy_analysis(&mut details, action);
                draft_plan.nodes.push(PlannerDraftNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    node_type: "product".to_string(),
                    name,
                    summary: description.clone().or(vision.clone()),
                    details,
                });
            }
        }
        "create_product_area" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft product area needs a product".to_string())
                    })?;
            let product = find_draft_node(draft_plan, "product", &product_name, None)
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
            let name = string_field(action, "name").ok_or_else(|| {
                AppError::Validation("Draft product area name is required".to_string())
            })?;
            if find_draft_node(draft_plan, "product_area", &name, Some(&product.id)).is_none() {
                let description = string_field(action, "description");
                let purpose = string_field(action, "purpose");
                let explanation = string_field(action, "explanation");
                let implementation_notes = string_field(action, "implementationNotes")
                    .or_else(|| string_field(action, "implementation_notes"));
                let mut details = json!({
                    "type": "create_product_area",
                    "name": string_field(action, "name").unwrap_or_default(),
                    "product_area_name": string_field(action, "name").unwrap_or_default(),
                    "description": description,
                    "purpose": purpose,
                    "nodeKind": parse_node_kind_value(node_kind_field(action))
                        .unwrap_or_else(HierarchyNodeKind::default_root)
                        .to_string(),
                    "explanation": explanation,
                    "examples": string_field(action, "examples"),
                    "implementationNotes": implementation_notes,
                    "testGuidance": string_field(action, "testGuidance")
                        .or_else(|| string_field(action, "test_guidance")),
                    "target": {
                        "productName": product_name,
                    }
                });
                copy_analysis(&mut details, action);
                draft_plan.nodes.push(PlannerDraftNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: Some(product.id.clone()),
                    node_type: "product_area".to_string(),
                    name,
                    summary: description
                        .clone()
                        .or(purpose.clone())
                        .or(explanation.clone())
                        .or(implementation_notes.clone()),
                    details,
                });
            }
        }
        "create_capability" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft capability needs a product".to_string())
                    })?;
            let product = find_draft_node(draft_plan, "product", &product_name, None)
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
            let product_area_name =
                resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft capability needs a product area".to_string())
                    })?;
            let product_area = find_draft_node(
                draft_plan,
                "product_area",
                &product_area_name,
                Some(&product.id),
            )
            .ok_or_else(|| AppError::Validation("Draft product area is required".to_string()))?;
            let parent_capability_id =
                resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?;
            let parent_node = parent_capability_id
                .as_deref()
                .and_then(|node_id| find_draft_node_by_id(draft_plan, Some(node_id)))
                .cloned();
            let name = string_field(action, "name").ok_or_else(|| {
                AppError::Validation("Draft capability name is required".to_string())
            })?;
            let parent_id = parent_capability_id
                .clone()
                .unwrap_or_else(|| product_area.id.clone());
            if find_draft_node(draft_plan, "capability", &name, Some(&parent_id)).is_none() {
                let description = string_field(action, "description");
                let explanation = string_field(action, "explanation");
                let acceptance_criteria = string_field(action, "acceptanceCriteria")
                    .or_else(|| string_field(action, "acceptance_criteria"));
                let implementation_notes = string_field(action, "implementationNotes")
                    .or_else(|| string_field(action, "implementation_notes"));
                let test_guidance = string_field(action, "testGuidance")
                    .or_else(|| string_field(action, "test_guidance"));
                let parent_kind = parent_node
                    .as_ref()
                    .map(|node| draft_node_kind(draft_plan, node))
                    .unwrap_or_else(|| draft_node_kind(draft_plan, product_area));
                let mut details = json!({
                    "type": "create_capability",
                    "name": string_field(action, "name").unwrap_or_default(),
                    "capability_name": string_field(action, "name").unwrap_or_default(),
                    "description": description,
                    "acceptanceCriteria": acceptance_criteria,
                    "priority": string_field(action, "priority")
                        .unwrap_or_else(|| "medium".to_string()),
                    "risk": string_field(action, "risk")
                        .unwrap_or_else(|| "medium".to_string()),
                    "technicalNotes": string_field(action, "technicalNotes")
                        .or_else(|| string_field(action, "technical_notes")),
                    "nodeKind": parse_node_kind_value(node_kind_field(action))
                        .unwrap_or_else(|| HierarchyNodeKind::default_child(&parent_kind))
                        .to_string(),
                    "explanation": explanation,
                    "examples": string_field(action, "examples"),
                    "implementationNotes": implementation_notes,
                    "testGuidance": test_guidance,
                    "target": {
                        "productName": product_name,
                        "productAreaName": product_area_name,
                        "capabilityName": parent_node.as_ref().map(|node| node.name.clone()),
                    }
                });
                copy_analysis(&mut details, action);
                draft_plan.nodes.push(PlannerDraftNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: Some(parent_id),
                    node_type: "capability".to_string(),
                    name,
                    summary: description
                        .clone()
                        .or(explanation.clone())
                        .or(acceptance_criteria.clone())
                        .or(implementation_notes.clone())
                        .or(test_guidance.clone()),
                    details,
                });
            }
        }
        "create_work_item" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
                    .ok_or_else(|| {
                        AppError::Validation("Draft work item needs a product".to_string())
                    })?;
            let product = find_draft_node(draft_plan, "product", &product_name, None)
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
            let title = string_field(action, "title").ok_or_else(|| {
                AppError::Validation("Draft work item title is required".to_string())
            })?;
            let product_area_id =
                resolve_draft_product_area_node_id(draft_plan, selected_draft_node_id, action)?;
            let capability_id =
                resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?;
            let parent_id = if let Some(capability_id) = capability_id.clone() {
                Some(capability_id)
            } else if let Some(product_area_id) = product_area_id.clone() {
                Some(product_area_id)
            } else {
                Some(product.id.clone())
            };
            if find_draft_node(draft_plan, "work_item", &title, parent_id.as_deref()).is_none() {
                let product_area_name = product_area_id
                    .as_deref()
                    .and_then(|node_id| find_draft_node_by_id(draft_plan, Some(node_id)))
                    .filter(|node| node.node_type == "product_area")
                    .map(|node| node.name.clone())
                    .or_else(|| {
                        capability_id
                            .as_deref()
                            .and_then(|node_id| find_draft_node_by_id(draft_plan, Some(node_id)))
                            .and_then(|node| {
                                find_draft_ancestor_name(draft_plan, node, "product_area")
                            })
                    });
                let capability_name = capability_id
                    .as_deref()
                    .and_then(|node_id| find_draft_node_by_id(draft_plan, Some(node_id)))
                    .map(|node| node.name.clone());
                let mut details = json!({
                    "type": "create_work_item",
                    "title": string_field(action, "title").unwrap_or_default(),
                    "work_item_name": string_field(action, "title").unwrap_or_default(),
                    "description": string_field(action, "description"),
                    "problemStatement": string_field(action, "problemStatement")
                        .or_else(|| string_field(action, "problem_statement")),
                    "acceptanceCriteria": string_field(action, "acceptanceCriteria")
                        .or_else(|| string_field(action, "acceptance_criteria")),
                    "constraints": string_field(action, "constraints"),
                    "workItemType": string_field(action, "workItemType")
                        .or_else(|| string_field(action, "work_item_type"))
                        .unwrap_or_else(|| "story".to_string()),
                    "priority": string_field(action, "priority")
                        .unwrap_or_else(|| "medium".to_string()),
                    "complexity": string_field(action, "complexity")
                        .unwrap_or_else(|| "medium".to_string()),
                    "target": {
                        "productName": product_name,
                        "productAreaName": product_area_name,
                        "capabilityName": capability_name,
                    }
                });
                copy_analysis(&mut details, action);
                draft_plan.nodes.push(PlannerDraftNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id,
                    node_type: "work_item".to_string(),
                    name: title,
                    summary: string_field(action, "description")
                        .or_else(|| string_field(action, "problemStatement"))
                        .or_else(|| string_field(action, "problem_statement")),
                    details,
                });
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
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

pub(crate) fn build_capability_template_actions(
    draft_plan: &PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<Vec<Value>, AppError> {
    let product_name =
        resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
            .ok_or_else(|| AppError::Validation("Draft template needs a product".to_string()))?;
    let product_area_name =
        resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?
            .ok_or_else(|| {
                AppError::Validation("Draft template needs a product area".to_string())
            })?;
    let parent_capability_name =
        resolve_draft_capability_name(Some(draft_plan), selected_draft_node_id, action)?;
    let name = string_field(action, "name")
        .ok_or_else(|| AppError::Validation("Draft template name is required".to_string()))?;
    let template_kind_value = string_field(action, "templateKind")
        .or_else(|| string_field(action, "template_kind"))
        .ok_or_else(|| AppError::Validation("Draft template kind is required".to_string()))?;
    let template_kind = SemanticTemplateKind::parse(&template_kind_value).ok_or_else(|| {
        AppError::Validation(
            "Unsupported template kind. Use operator_chapter or technical_topic_book.".to_string(),
        )
    })?;
    let priority = string_field(action, "priority").unwrap_or_else(|| "medium".to_string());
    let risk = string_field(action, "risk").unwrap_or_else(|| "medium".to_string());
    let explanation = string_field(action, "explanation").unwrap_or_default();
    let examples = string_field(action, "examples").unwrap_or_default();
    let implementation_notes = string_field(action, "implementationNotes")
        .or_else(|| string_field(action, "implementation_notes"))
        .unwrap_or_default();
    let test_guidance = string_field(action, "testGuidance")
        .or_else(|| string_field(action, "test_guidance"))
        .unwrap_or_default();
    let description =
        string_field(action, "description").unwrap_or_else(|| format!("{name} book section."));
    let (definition_label, examples_label, implementation_label, tests_label) = match template_kind
    {
        SemanticTemplateKind::OperatorChapter => (
            format!("{name} Definition"),
            format!("{name} Examples"),
            format!("{name} Implementation"),
            format!("{name} Tests"),
        ),
        SemanticTemplateKind::TechnicalTopicBook => (
            format!("{name} Overview"),
            format!("{name} Examples"),
            format!("{name} Implementation"),
            format!("{name} Tests"),
        ),
    };

    Ok(vec![
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": parent_capability_name,
            },
            "name": name,
            "description": description,
            "acceptanceCriteria": format!("{name} has definition, examples, implementation guidance, and test guidance captured."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Template-generated semantic chapter root.",
            "nodeKind": "capability"
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": definition_label,
            "description": format!("Explain what {name} is and when it should be used."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature chapter for explanation and conceptual boundaries.",
            "nodeKind": "feature",
            "explanation": explanation,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": examples_label,
            "description": format!("Capture worked examples and expected behaviors for {name}."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature chapter for examples and concrete edge cases.",
            "nodeKind": "feature",
            "examples": examples,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": implementation_label,
            "description": format!("Describe how {name} should be implemented."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature execution notes for implementation stories.",
            "nodeKind": "feature",
            "implementationNotes": implementation_notes,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": tests_label,
            "description": format!("Describe how {name} should be validated."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature execution notes for test and verification stories.",
            "nodeKind": "feature",
            "testGuidance": test_guidance,
        }),
        json!({
            "type": "create_work_item",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": implementation_label,
            },
            "title": format!("Implement {name}"),
            "problemStatement": format!("{name} needs implementation aligned to the authored chapter structure."),
            "description": implementation_notes,
            "acceptanceCriteria": format!("{name} is implemented and matches the documented behavior, examples, and edge cases."),
            "constraints": "Preserve the authored semantic structure and keep behavior deterministic.",
            "workItemType": "story",
            "priority": priority,
            "complexity": "medium",
        }),
        json!({
            "type": "create_work_item",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": tests_label,
            },
            "title": format!("Write {name} test cases"),
            "problemStatement": format!("{name} needs verification that matches the documented examples and risks."),
            "description": test_guidance,
            "acceptanceCriteria": format!("Coverage validates happy paths, edge cases, and regressions for {name}."),
            "constraints": "Keep tests aligned with the authored examples and implementation notes.",
            "workItemType": "test",
            "priority": priority,
            "complexity": "medium",
        }),
    ])
}

pub(crate) fn convert_draft_capability_kind(
    draft_plan: &mut PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<(), AppError> {
    let capability_id =
        resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?
            .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
    let target_kind_value = string_field(action, "nodeKind")
        .or_else(|| string_field(action, "node_kind"))
        .ok_or_else(|| AppError::Validation("Draft node kind is required".to_string()))?;
    let target_kind = HierarchyNodeKind::parse(&target_kind_value).ok_or_else(|| {
        AppError::Validation(format!("Unsupported node kind {}", target_kind_value))
    })?;
    let strategy = string_field(action, "childStrategy")
        .or_else(|| string_field(action, "child_strategy"))
        .map(|value| {
            ChildReparentStrategy::parse(&value).ok_or_else(|| {
                AppError::Validation(format!("Unsupported child strategy {}", value))
            })
        })
        .transpose()?
        .unwrap_or(ChildReparentStrategy::Reject);
    let target_node = draft_plan
        .nodes
        .iter()
        .find(|node| node.id == capability_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
    let parent_kind = target_node
        .parent_id
        .as_deref()
        .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)))
        .map(|parent| draft_node_kind(draft_plan, parent))
        .unwrap_or(HierarchyNodeKind::Capability);
    if !parent_kind.supports_child_kind(&target_kind) {
        return Err(AppError::Validation(format!(
            "{} cannot contain {}.",
            parent_kind, target_kind
        )));
    }

    let direct_structural_children = draft_plan
        .nodes
        .iter()
        .filter(|child| {
            child.parent_id.as_deref() == Some(capability_id.as_str())
                && child.node_type == "capability"
        })
        .cloned()
        .collect::<Vec<_>>();
    if !direct_structural_children.is_empty() && !target_kind.can_have_children() {
        if strategy != ChildReparentStrategy::ReparentToParent {
            return Err(AppError::Validation(format!(
                "{} cannot contain structural children. Re-run with childStrategy=reparent_to_parent to preserve descendants.",
                target_kind
            )));
        }
        for child in &direct_structural_children {
            let child_kind = draft_node_kind(draft_plan, child);
            if !parent_kind.supports_child_kind(&child_kind) {
                return Err(AppError::Validation(format!(
                    "{} cannot receive existing {} children.",
                    parent_kind, child_kind
                )));
            }
        }
        let new_parent = target_node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)))
            .cloned();
        for child in direct_structural_children {
            if let Some(child_node) = draft_plan.nodes.iter_mut().find(|node| node.id == child.id) {
                child_node.parent_id = target_node.parent_id.clone();
                if let Some(parent) = new_parent.as_ref() {
                    if parent.node_type == "capability" {
                        set_target_string_value(
                            &mut child_node.details,
                            "capabilityName",
                            &parent.name,
                        );
                    } else {
                        remove_target_value(&mut child_node.details, "capabilityName");
                    }
                } else {
                    remove_target_value(&mut child_node.details, "capabilityName");
                }
            }
        }
    }
    if let Some(node) = draft_plan
        .nodes
        .iter_mut()
        .find(|node| node.id == capability_id)
    {
        set_string_value(&mut node.details, "nodeKind", &target_kind.to_string());
    }
    Ok(())
}

pub(crate) fn apply_delete_draft_action(
    draft_plan: &mut PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action_type: &str,
    action: &Value,
) -> Result<bool, AppError> {
    let candidate = match action_type {
        "archive_product" => {
            resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?.and_then(
                |name| {
                    find_draft_node(draft_plan, "product", &name, None).map(|node| node.id.clone())
                },
            )
        }
        "delete_product_area" => {
            let product_name =
                resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?;
            let product = product_name
                .as_deref()
                .and_then(|name| find_draft_node(draft_plan, "product", name, None));
            let product_area_name =
                resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?;
            match (product, product_area_name) {
                (Some(product), Some(product_area_name)) => find_draft_node(
                    draft_plan,
                    "product_area",
                    &product_area_name,
                    Some(&product.id),
                )
                .map(|node| node.id.clone()),
                _ => None,
            }
        }
        "delete_capability" => {
            resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?
        }
        "delete_work_item" => {
            let title = target_field(action, "workItemTitle")
                .map(ToString::to_string)
                .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
            draft_plan
                .nodes
                .iter()
                .find(|node| {
                    node.node_type == "work_item"
                        && normalize_text(Some(&node.name)) == normalize_text(Some(&title))
                })
                .map(|node| node.id.clone())
        }
        _ => return Ok(false),
    };
    if let Some(node_id) = candidate {
        remove_draft_node_subtree(draft_plan, &node_id);
    }
    Ok(true)
}

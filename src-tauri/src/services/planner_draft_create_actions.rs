use crate::domain::product::HierarchyNodeKind;
use crate::error::AppError;
use crate::services::planner_action_fields::{copy_analysis, string_array_field, string_field};
use crate::services::planner_draft::{
    draft_node_kind, find_draft_ancestor_name, find_draft_node, find_draft_node_by_id,
    node_kind_field, parse_node_kind_value, resolve_draft_capability_node_id,
    resolve_draft_product_area_name, resolve_draft_product_area_node_id,
    resolve_draft_product_name,
};
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan};
use serde_json::{json, Value};

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

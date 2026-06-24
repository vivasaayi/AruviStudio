use crate::domain::product::HierarchyNodeKind;
use crate::error::AppError;
use crate::services::planner_action_fields::{
    set_string_value, set_target_string_value, target_field,
};
use crate::services::planner_draft::{draft_node_kind, find_draft_ancestor_name};
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan};
use serde_json::json;

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

pub(crate) fn remove_draft_node_subtree(draft_plan: &mut PlannerDraftPlan, node_id: &str) {
    let mut to_remove = vec![node_id.to_string()];
    let mut index = 0;
    while index < to_remove.len() {
        let current = to_remove[index].clone();
        for child in draft_plan
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(current.as_str()))
        {
            to_remove.push(child.id.clone());
        }
        index += 1;
    }
    draft_plan
        .nodes
        .retain(|node| !to_remove.contains(&node.id));
}

fn draft_name_taken(
    draft_plan: &PlannerDraftPlan,
    node_type: &str,
    parent_id: Option<&str>,
    name: &str,
    excluding_node_id: Option<&str>,
) -> bool {
    let normalized_name = normalize_text(Some(name));
    draft_plan.nodes.iter().any(|node| {
        node.node_type == node_type
            && node.parent_id.as_deref() == parent_id
            && excluding_node_id != Some(node.id.as_str())
            && normalize_text(Some(&node.name)) == normalized_name
    })
}

fn allowed_draft_child_types(parent_type: &str) -> &'static [&'static str] {
    match parent_type {
        "product" => &["product_area", "work_item"],
        "product_area" => &["capability", "work_item"],
        "capability" => &["capability", "work_item"],
        _ => &[],
    }
}

fn normalize_draft_child_type(value: &str) -> Option<&'static str> {
    match normalize_text(Some(value)).as_str() {
        "product area" | "product_area" | "productarea" => Some("product_area"),
        "capability" => Some("capability"),
        "work item" | "work_item" | "workitem" => Some("work_item"),
        _ => None,
    }
}

pub(crate) fn update_descendant_targets_for_rename(
    draft_plan: &mut PlannerDraftPlan,
    renamed_node_id: &str,
    renamed_node_type: &str,
    previous_name: &str,
    next_name: &str,
) {
    let mut descendant_ids = vec![];
    let mut index = 0;
    let mut queue = vec![renamed_node_id.to_string()];
    while index < queue.len() {
        let current = queue[index].clone();
        for child in draft_plan
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(current.as_str()))
        {
            descendant_ids.push(child.id.clone());
            queue.push(child.id.clone());
        }
        index += 1;
    }

    let target_key = match renamed_node_type {
        "product" => "productName",
        "product_area" => "productAreaName",
        "capability" => "capabilityName",
        "work_item" => "workItemTitle",
        _ => return,
    };
    let previous_normalized = normalize_text(Some(previous_name));

    for node in draft_plan
        .nodes
        .iter_mut()
        .filter(|node| descendant_ids.contains(&node.id))
    {
        let existing_target = target_field(&node.details, target_key).map(ToString::to_string);
        if existing_target
            .as_deref()
            .map(|value| normalize_text(Some(value)) == previous_normalized)
            .unwrap_or(false)
        {
            set_target_string_value(&mut node.details, target_key, next_name);
        }
    }
}

pub(crate) fn rename_draft_node(
    draft_plan: &mut PlannerDraftPlan,
    node_id: &str,
    next_name: &str,
) -> Result<PlannerDraftNode, AppError> {
    let next_name = next_name.trim();
    if next_name.is_empty() {
        return Err(AppError::Validation(
            "Draft node name cannot be empty".to_string(),
        ));
    }
    let node_index = draft_plan
        .nodes
        .iter()
        .position(|node| node.id == node_id)
        .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))?;
    let current = draft_plan.nodes[node_index].clone();
    if draft_name_taken(
        draft_plan,
        &current.node_type,
        current.parent_id.as_deref(),
        next_name,
        Some(node_id),
    ) {
        return Err(AppError::Validation(format!(
            "A sibling {} named \"{}\" already exists",
            current.node_type.replace('_', " "),
            next_name
        )));
    }
    if normalize_text(Some(&current.name)) == normalize_text(Some(next_name)) {
        return Ok(current);
    }

    {
        let node = draft_plan
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))?;
        node.name = next_name.to_string();
        match node.node_type.as_str() {
            "work_item" => {
                set_string_value(&mut node.details, "title", next_name);
                set_string_value(&mut node.details, "work_item_name", next_name);
                set_target_string_value(&mut node.details, "workItemTitle", next_name);
            }
            "product" => {
                set_string_value(&mut node.details, "name", next_name);
                set_target_string_value(&mut node.details, "productName", next_name);
            }
            "product_area" => {
                set_string_value(&mut node.details, "name", next_name);
                set_string_value(&mut node.details, "product_area_name", next_name);
                set_target_string_value(&mut node.details, "productAreaName", next_name);
            }
            "capability" => {
                set_string_value(&mut node.details, "name", next_name);
                set_string_value(&mut node.details, "capability_name", next_name);
                set_target_string_value(&mut node.details, "capabilityName", next_name);
            }
            _ => {
                set_string_value(&mut node.details, "name", next_name);
            }
        }
    }

    update_descendant_targets_for_rename(
        draft_plan,
        node_id,
        &current.node_type,
        &current.name,
        next_name,
    );

    draft_plan
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))
}

pub(crate) fn add_draft_child_node(
    draft_plan: &mut PlannerDraftPlan,
    parent_node_id: &str,
    child_type: &str,
    name: &str,
    summary: Option<&str>,
) -> Result<PlannerDraftNode, AppError> {
    let parent = draft_plan
        .nodes
        .iter()
        .find(|node| node.id == parent_node_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("Parent design node was not found".to_string()))?;
    let child_type = normalize_draft_child_type(child_type).ok_or_else(|| {
        AppError::Validation(format!("Unsupported draft child type {}", child_type))
    })?;
    if !allowed_draft_child_types(&parent.node_type).contains(&child_type) {
        return Err(AppError::Validation(format!(
            "Cannot add a {} under a {}",
            child_type.replace('_', " "),
            parent.node_type.replace('_', " ")
        )));
    }
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::Validation(
            "Draft child name cannot be empty".to_string(),
        ));
    }
    if draft_name_taken(
        draft_plan,
        child_type,
        Some(parent.id.as_str()),
        trimmed_name,
        None,
    ) {
        return Err(AppError::Validation(format!(
            "A sibling {} named \"{}\" already exists",
            child_type.replace('_', " "),
            trimmed_name
        )));
    }

    let product_name = if parent.node_type == "product" {
        Some(parent.name.clone())
    } else {
        find_draft_ancestor_name(draft_plan, &parent, "product")
    };
    let product_area_name = if parent.node_type == "product_area" {
        Some(parent.name.clone())
    } else {
        find_draft_ancestor_name(draft_plan, &parent, "product_area")
    };
    let capability_name = if parent.node_type == "capability" {
        Some(parent.name.clone())
    } else {
        find_draft_ancestor_name(draft_plan, &parent, "capability")
    };
    let trimmed_summary = summary.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let details = match child_type {
        "product_area" => json!({
            "type": "create_product_area",
            "name": trimmed_name,
            "product_area_name": trimmed_name,
            "description": trimmed_summary,
            "nodeKind": HierarchyNodeKind::default_root().to_string(),
            "target": {
                "productName": product_name
            }
        }),
        "capability" => json!({
            "type": "create_capability",
            "name": trimmed_name,
            "capability_name": trimmed_name,
            "description": trimmed_summary,
            "nodeKind": HierarchyNodeKind::default_child(&draft_node_kind(draft_plan, &parent)).to_string(),
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": if parent.node_type == "capability" { capability_name } else { None }
            }
        }),
        "work_item" => json!({
            "type": "create_work_item",
            "title": trimmed_name,
            "work_item_name": trimmed_name,
            "description": trimmed_summary,
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": capability_name
            }
        }),
        _ => unreachable!(),
    };

    let created = PlannerDraftNode {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: Some(parent.id.clone()),
        node_type: child_type.to_string(),
        name: trimmed_name.to_string(),
        summary: trimmed_summary,
        details,
    };
    draft_plan.nodes.push(created.clone());
    Ok(created)
}

pub(crate) fn delete_draft_node(
    draft_plan: &mut PlannerDraftPlan,
    node_id: &str,
) -> Result<(PlannerDraftNode, Option<String>), AppError> {
    let removed = draft_plan
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))?;
    let fallback_parent_id = removed.parent_id.clone();
    remove_draft_node_subtree(draft_plan, node_id);
    Ok((removed, fallback_parent_id))
}

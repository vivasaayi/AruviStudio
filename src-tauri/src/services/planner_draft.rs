use crate::domain::product::HierarchyNodeKind;
use crate::error::AppError;
use crate::services::planner_action_fields::{
    analysis_string, analysis_string_array, fields_string, string_field, target_field,
};
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan, PlannerTreeNode};
use serde_json::Value;

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

pub(crate) fn node_kind_field(value: &Value) -> Option<String> {
    string_field(value, "nodeKind")
        .or_else(|| string_field(value, "node_kind"))
        .or_else(|| fields_string(value, "nodeKind"))
        .or_else(|| fields_string(value, "node_kind"))
}

pub(crate) fn parse_node_kind_value(value: Option<String>) -> Option<HierarchyNodeKind> {
    value.and_then(|candidate| HierarchyNodeKind::parse(candidate.trim()))
}

pub(crate) fn draft_node_kind(
    draft_plan: &PlannerDraftPlan,
    node: &PlannerDraftNode,
) -> HierarchyNodeKind {
    if let Some(kind) = parse_node_kind_value(node_kind_field(&node.details)) {
        return kind;
    }
    match node.node_type.as_str() {
        "product_area" => HierarchyNodeKind::default_root(),
        "capability" => node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)))
            .map(|parent| HierarchyNodeKind::default_child(&draft_node_kind(draft_plan, parent)))
            .unwrap_or(HierarchyNodeKind::Capability),
        _ => HierarchyNodeKind::Capability,
    }
}

fn draft_node_has_long_form_fields(node: &PlannerDraftNode) -> bool {
    [
        string_field(&node.details, "explanation"),
        string_field(&node.details, "examples"),
        string_field(&node.details, "implementationNotes")
            .or_else(|| string_field(&node.details, "implementation_notes")),
        string_field(&node.details, "testGuidance")
            .or_else(|| string_field(&node.details, "test_guidance")),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}

fn draft_node_meta(draft_plan: &PlannerDraftPlan, node: &PlannerDraftNode) -> String {
    let mut meta = match node.node_type.as_str() {
        "product" => "draft product",
        "product_area" => "draft product area",
        "capability" => "draft capability",
        "work_item" => "draft work item",
        _ => "design node",
    }
    .to_string();
    if matches!(node.node_type.as_str(), "product_area" | "capability") {
        meta.push_str(&format!(" · {}", draft_node_kind(draft_plan, node)));
    }
    if draft_node_has_long_form_fields(node) {
        meta.push_str(" · long-form");
    }
    meta
}

fn draft_node_source(node: &PlannerDraftNode) -> Option<String> {
    analysis_string(&node.details, "source")
}

fn draft_node_confidence(node: &PlannerDraftNode) -> Option<String> {
    analysis_string(&node.details, "confidence")
}

fn draft_node_evidence(node: &PlannerDraftNode) -> Vec<String> {
    analysis_string_array(&node.details, "evidence")
}

fn build_draft_tree_children(
    draft_plan: &PlannerDraftPlan,
    parent_id: Option<&str>,
    selected_node_id: Option<&str>,
) -> Vec<PlannerTreeNode> {
    let mut nodes = draft_plan
        .nodes
        .iter()
        .filter(|node| node.parent_id.as_deref() == parent_id)
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes
        .into_iter()
        .map(|node| {
            let mut meta = draft_node_meta(draft_plan, &node);
            if let Some(source) = draft_node_source(&node) {
                let confidence = draft_node_confidence(&node)
                    .map(|value| format!("{} confidence", value))
                    .unwrap_or_else(|| "inferred".to_string());
                meta = format!("{meta} · {} · {}", source.replace('_', " "), confidence);
            }
            if selected_node_id == Some(node.id.as_str()) {
                meta = format!("{meta} selected");
            }
            PlannerTreeNode {
                id: node.id.clone(),
                label: node.name.clone(),
                meta: Some(meta),
                node_type: Some(node.node_type.clone()),
                summary: node.summary.clone(),
                source: draft_node_source(&node),
                confidence: draft_node_confidence(&node),
                evidence: draft_node_evidence(&node),
                children: build_draft_tree_children(draft_plan, Some(&node.id), selected_node_id),
            }
        })
        .collect()
}

pub(crate) fn build_draft_tree_nodes(
    draft_plan: &PlannerDraftPlan,
    selected_node_id: Option<&str>,
) -> Vec<PlannerTreeNode> {
    build_draft_tree_children(draft_plan, None, selected_node_id)
}

pub(crate) fn find_draft_node<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_type: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Option<&'a PlannerDraftNode> {
    let normalized = normalize_text(Some(name));
    draft_plan.nodes.iter().find(|node| {
        node.node_type == node_type
            && node.parent_id.as_deref() == parent_id
            && normalize_text(Some(&node.name)) == normalized
    })
}

pub(crate) fn find_draft_node_by_id<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_id: Option<&str>,
) -> Option<&'a PlannerDraftNode> {
    let node_id = node_id?;
    draft_plan.nodes.iter().find(|node| node.id == node_id)
}

pub(crate) fn build_draft_node_path<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_id: Option<&str>,
) -> Vec<&'a PlannerDraftNode> {
    let mut reversed = vec![];
    let mut current = find_draft_node_by_id(draft_plan, node_id);
    while let Some(node) = current {
        reversed.push(node);
        current = node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    }
    reversed.reverse();
    reversed
}

fn find_unique_draft_node_by_name<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_type: &str,
    name: &str,
) -> Result<Option<&'a PlannerDraftNode>, AppError> {
    let normalized = normalize_text(Some(name));
    let matches = draft_plan
        .nodes
        .iter()
        .filter(|node| {
            node.node_type == node_type && normalize_text(Some(&node.name)) == normalized
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "Multiple draft {} nodes match {}",
            node_type, name
        )));
    }
    Ok(matches.into_iter().next())
}

pub(crate) fn find_draft_ancestor_name(
    draft_plan: &PlannerDraftPlan,
    node: &PlannerDraftNode,
    ancestor_type: &str,
) -> Option<String> {
    let mut current = node
        .parent_id
        .as_deref()
        .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    while let Some(parent) = current {
        if parent.node_type == ancestor_type {
            return Some(parent.name.clone());
        }
        current = parent
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    }
    None
}

pub(crate) fn find_draft_ancestor_node_by_type<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node: &'a PlannerDraftNode,
    ancestor_type: &str,
) -> Option<&'a PlannerDraftNode> {
    let mut current = node
        .parent_id
        .as_deref()
        .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    while let Some(parent) = current {
        if parent.node_type == ancestor_type {
            return Some(parent);
        }
        current = parent
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    }
    None
}

pub(crate) fn resolve_draft_product_area_node_id(
    draft_plan: &PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    let product_name =
        resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?;
    let product_id = product_name
        .as_deref()
        .and_then(|name| find_draft_node(draft_plan, "product", name, None))
        .map(|node| node.id.clone());
    let product_area_name =
        resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?;
    let Some(product_area_name) = product_area_name else {
        return Ok(None);
    };
    if let Some(product_id) = product_id.as_deref() {
        if let Some(product_area) = find_draft_node(
            draft_plan,
            "product_area",
            &product_area_name,
            Some(product_id),
        ) {
            return Ok(Some(product_area.id.clone()));
        }
    }
    Ok(
        find_unique_draft_node_by_name(draft_plan, "product_area", &product_area_name)?
            .map(|node| node.id.clone()),
    )
}

pub(crate) fn resolve_draft_capability_node_id(
    draft_plan: &PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    let capability_name =
        resolve_draft_capability_name(Some(draft_plan), selected_draft_node_id, action)?;
    let Some(capability_name) = capability_name else {
        return Ok(None);
    };
    let product_area_id =
        resolve_draft_product_area_node_id(draft_plan, selected_draft_node_id, action)?;
    let normalized_name = normalize_text(Some(&capability_name));
    let matches = draft_plan
        .nodes
        .iter()
        .filter(|node| {
            node.node_type == "capability"
                && normalize_text(Some(&node.name)) == normalized_name
                && product_area_id
                    .as_ref()
                    .map(|product_area_id| {
                        find_draft_ancestor_node_by_type(draft_plan, node, "product_area")
                            .map(|product_area| product_area.id == *product_area_id)
                            .unwrap_or(false)
                    })
                    .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "Multiple draft capability nodes match {}",
            capability_name
        )));
    }
    Ok(matches.first().map(|node| node.id.clone()))
}

pub(crate) fn infer_selected_draft_context(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(draft_plan) = draft_plan else {
        return (None, None, None, None);
    };
    let Some(selected) = find_draft_node_by_id(draft_plan, selected_node_id) else {
        return (None, None, None, None);
    };

    let mut product_name = None;
    let mut product_area_name = None;
    let mut capability_name = None;
    let mut work_item_title = None;
    let mut current = Some(selected);
    while let Some(node) = current {
        match node.node_type.as_str() {
            "product" => product_name = Some(node.name.clone()),
            "product_area" => product_area_name = Some(node.name.clone()),
            "capability" => capability_name = Some(node.name.clone()),
            "work_item" => work_item_title = Some(node.name.clone()),
            _ => {}
        }
        current = node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    }
    (
        product_name,
        product_area_name,
        capability_name,
        work_item_title,
    )
}

pub(crate) fn resolve_draft_product_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "productName") {
        if let Some(draft_plan) = draft_plan {
            if find_draft_node(draft_plan, "product", name, None).is_some() {
                return Ok(Some(name.to_string()));
            }
            if let Some(node) = find_unique_draft_node_by_name(draft_plan, "product_area", name)? {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
            if let Some(node) = find_unique_draft_node_by_name(draft_plan, "capability", name)? {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
            if let Some(node) = find_unique_draft_node_by_name(draft_plan, "work_item", name)? {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
        }
        return Ok(Some(name.to_string()));
    }
    if let Some(product_area_name) = target_field(action, "productAreaName") {
        if let Some(draft_plan) = draft_plan {
            if let Some(node) =
                find_unique_draft_node_by_name(draft_plan, "product_area", product_area_name)?
            {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
        }
    }
    if let Some(capability_name) = target_field(action, "capabilityName") {
        if let Some(draft_plan) = draft_plan {
            if let Some(node) =
                find_unique_draft_node_by_name(draft_plan, "capability", capability_name)?
            {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
        }
    }
    let work_item_title = target_field(action, "workItemTitle")
        .map(ToString::to_string)
        .or_else(|| target_field(action, "work_item_title").map(ToString::to_string))
        .or_else(|| target_field(action, "work_item_name").map(ToString::to_string))
        .or_else(|| string_field(action, "title"))
        .or_else(|| string_field(action, "work_item_title"))
        .or_else(|| string_field(action, "work_item_name"));
    if let Some(work_item_title) = work_item_title {
        if let Some(draft_plan) = draft_plan {
            if let Some(node) =
                find_unique_draft_node_by_name(draft_plan, "work_item", &work_item_title)?
            {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product"));
            }
        }
    }
    let (product_name, _, _, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    if product_name.is_some() {
        return Ok(product_name);
    }
    let Some(draft_plan) = draft_plan else {
        return Ok(None);
    };
    let products = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "product")
        .collect::<Vec<_>>();
    if products.len() == 1 {
        return Ok(Some(products[0].name.clone()));
    }
    Ok(None)
}

pub(crate) fn resolve_draft_product_area_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "productAreaName") {
        return Ok(Some(name.to_string()));
    }
    if let Some(capability_name) = target_field(action, "capabilityName") {
        if let Some(draft_plan) = draft_plan {
            if let Some(node) =
                find_unique_draft_node_by_name(draft_plan, "capability", capability_name)?
            {
                return Ok(find_draft_ancestor_name(draft_plan, node, "product_area"));
            }
        }
    }
    let (_, product_area_name, _, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    Ok(product_area_name)
}

pub(crate) fn resolve_draft_capability_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "capabilityName") {
        return Ok(Some(name.to_string()));
    }
    let (_, _, capability_name, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    Ok(capability_name)
}

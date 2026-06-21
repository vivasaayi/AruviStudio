use crate::domain::product::HierarchyNodeKind;
use crate::error::AppError;
use crate::services::planner_action_fields::{
    analysis_string, analysis_string_array, fields_string, set_string_value,
    set_target_string_value, string_field, target_field,
};
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan, PlannerTreeNode};
use serde_json::{json, Value};
use std::collections::BTreeMap;

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

fn planner_draft_node_type_label(node_type: &str) -> &'static str {
    match node_type {
        "product" => "product",
        "product_area" => "product area",
        "capability" => "capability",
        "work_item" => "work item",
        _ => "node",
    }
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

fn find_draft_path_ancestor_by_type<'a>(
    path: &[&'a PlannerDraftNode],
    node_type: &str,
) -> Option<&'a PlannerDraftNode> {
    path.iter()
        .rev()
        .find(|node| node.node_type == node_type)
        .copied()
}

pub(crate) fn parse_voice_node_reference(spoken_remainder: &str) -> (Option<&'static str>, String) {
    let trimmed = spoken_remainder.trim();
    let prefixes = [
        ("work item ", "work_item"),
        ("work-item ", "work_item"),
        ("workitem ", "work_item"),
        ("capability ", "capability"),
        ("product area ", "product_area"),
        ("product ", "product"),
        ("node ", "node"),
    ];
    for (prefix, node_type) in prefixes {
        if trimmed == prefix.trim() {
            return (
                Some(node_type),
                format!("selected {}", node_type.replace('_', " ")),
            );
        }
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return (Some(node_type), rest.trim().to_string());
        }
    }
    (None, trimmed.to_string())
}

pub(crate) fn resolve_voice_draft_node_reference(
    draft_plan: &PlannerDraftPlan,
    selected_node_id: Option<&str>,
    raw_reference: &str,
    explicit_type: Option<&str>,
) -> Result<Option<PlannerDraftNode>, AppError> {
    let reference = normalize_text(Some(raw_reference));
    if reference.is_empty() {
        return Ok(None);
    }

    let selected_path = build_draft_node_path(draft_plan, selected_node_id);
    let selected_node = selected_path.last().copied();
    let effective_type = explicit_type.filter(|value| *value != "node");

    let special_match = match reference.as_str() {
        "this" | "selected" | "this node" | "selected node" | "current" | "current node" => {
            if let Some(node_type) = effective_type {
                find_draft_path_ancestor_by_type(&selected_path, node_type)
            } else {
                selected_node
            }
        }
        "root" | "product" | "this product" | "selected product" | "root product" => {
            find_draft_path_ancestor_by_type(&selected_path, "product").or_else(|| {
                draft_plan
                    .nodes
                    .iter()
                    .find(|node| node.node_type == "product" && node.parent_id.is_none())
            })
        }
        "this product area" | "selected product area" | "current product area" => {
            find_draft_path_ancestor_by_type(&selected_path, "product_area")
        }
        "this capability" | "selected capability" | "current capability" => {
            find_draft_path_ancestor_by_type(&selected_path, "capability")
        }
        "this work item" | "selected work item" | "current work item" => {
            find_draft_path_ancestor_by_type(&selected_path, "work_item")
        }
        _ => None,
    };
    if let Some(node) = special_match {
        return Ok(Some(node.clone()));
    }

    let flattened = draft_plan
        .nodes
        .iter()
        .filter(|node| {
            effective_type
                .map(|value| node.node_type == value)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    let exact = flattened
        .iter()
        .filter(|node| normalize_text(Some(&node.name)) == reference)
        .copied()
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return Ok(Some(exact[0].clone()));
    }
    if exact.len() > 1 {
        return Err(AppError::Validation(format!(
            "Multiple draft {} nodes match {}",
            effective_type
                .map(planner_draft_node_type_label)
                .unwrap_or("node"),
            raw_reference
        )));
    }

    let partial = flattened
        .iter()
        .filter(|node| normalize_text(Some(&node.name)).contains(&reference))
        .copied()
        .collect::<Vec<_>>();
    if partial.len() == 1 {
        return Ok(Some(partial[0].clone()));
    }
    if partial.len() > 1 {
        return Err(AppError::Validation(format!(
            "Multiple draft {} nodes partially match {}",
            effective_type
                .map(planner_draft_node_type_label)
                .unwrap_or("node"),
            raw_reference
        )));
    }

    Ok(None)
}

pub(crate) fn summarize_selected_draft_node(
    draft_plan: &PlannerDraftPlan,
    node: &PlannerDraftNode,
) -> String {
    let children = draft_plan
        .nodes
        .iter()
        .filter(|candidate| candidate.parent_id.as_deref() == Some(node.id.as_str()))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return format!(
            "Selected {} \"{}\". It has no staged children yet.",
            planner_draft_node_type_label(&node.node_type),
            node.name
        );
    }

    let mut counts = BTreeMap::<String, usize>::new();
    for child in &children {
        *counts
            .entry(planner_draft_node_type_label(&child.node_type).to_string())
            .or_insert(0) += 1;
    }
    let counts_text = counts
        .iter()
        .map(|(node_type, count)| {
            if *count == 1 {
                format!("1 {node_type}")
            } else {
                format!("{count} {}s", node_type)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sample_names = children
        .iter()
        .take(3)
        .map(|child| child.name.clone())
        .collect::<Vec<_>>();
    let sample_text = if sample_names.is_empty() {
        String::new()
    } else {
        format!(" Examples: {}.", sample_names.join(", "))
    };

    format!(
        "Selected {} \"{}\". It currently contains {}.{}",
        planner_draft_node_type_label(&node.node_type),
        node.name,
        counts_text,
        sample_text
    )
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

use crate::error::AppError;
use crate::services::planner_draft::build_draft_node_path;
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan};
use std::collections::BTreeMap;

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
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

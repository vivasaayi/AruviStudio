use crate::services::planner_action_fields::{
    analysis_string, analysis_string_array, string_field,
};
use crate::services::planner_draft::draft_node_kind;
use crate::services::planner_service::{PlannerDraftNode, PlannerDraftPlan, PlannerTreeNode};

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

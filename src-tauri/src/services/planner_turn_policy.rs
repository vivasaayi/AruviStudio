use crate::domain::product::Product;
use crate::services::planner_types::{PlannerDraftNode, PlannerDraftPlan, PlannerPlan};
use serde_json::{json, Value};

pub(crate) fn normalize(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

pub(crate) fn seed_draft_with_product(
    draft_plan: Option<PlannerDraftPlan>,
    product: &Product,
) -> PlannerDraftPlan {
    let mut draft_plan = draft_plan.unwrap_or(PlannerDraftPlan { nodes: vec![] });
    let has_matching_product_root = draft_plan.nodes.iter().any(|node| {
        node.node_type == "product"
            && node.parent_id.is_none()
            && (node.id == product.id
                || normalize(Some(node.name.as_str())) == normalize(Some(product.name.as_str())))
    });
    if has_matching_product_root {
        return draft_plan;
    }
    if draft_plan
        .nodes
        .iter()
        .any(|node| node.node_type == "product" && node.parent_id.is_none())
    {
        draft_plan.nodes.clear();
    }
    draft_plan.nodes.push(PlannerDraftNode {
        id: product.id.clone(),
        parent_id: None,
        node_type: "product".to_string(),
        name: product.name.clone(),
        summary: Some(if product.description.trim().is_empty() {
            product.vision.clone()
        } else {
            product.description.clone()
        }),
        details: json!({
            "type": "update_product",
            "name": product.name.clone(),
            "description": product.description.clone(),
            "vision": product.vision.clone(),
            "goals": product.goals.clone(),
            "tags": product.tags.clone(),
            "target": {
                "productName": product.name.clone(),
            },
        }),
    });
    draft_plan
}

pub(crate) fn scope_plan_to_selected_product(plan: &mut PlannerPlan, product: &Product) {
    plan.actions.retain(|action| {
        !matches!(
            action.get("type").and_then(Value::as_str),
            Some("create_product" | "archive_product")
        )
    });
    for action in &mut plan.actions {
        let Some(action_object) = action.as_object_mut() else {
            continue;
        };
        let target = action_object
            .entry("target")
            .or_insert_with(|| json!({ "productName": product.name.clone() }));
        if !target.is_object() {
            *target = json!({});
        }
        if let Some(target_object) = target.as_object_mut() {
            target_object
                .entry("productName".to_string())
                .or_insert_with(|| Value::String(product.name.clone()));
        }
    }
}

pub(crate) fn heuristic_plan(input: &str) -> PlannerPlan {
    let lower = input.trim().to_lowercase();
    if (lower.contains("tree") || lower.contains("hierarch"))
        && (lower.contains("work item") || lower.contains("workitem") || lower.contains("tasks"))
    {
        return PlannerPlan {
            assistant_response: "I'll show the current work items in a hierarchical tree."
                .to_string(),
            needs_confirmation: false,
            clarification_question: None,
            actions: vec![json!({ "type": "report_tree" })],
        };
    }
    if lower.contains("status") {
        return PlannerPlan {
            assistant_response: "I'll report the current status from local workspace data."
                .to_string(),
            needs_confirmation: false,
            clarification_question: None,
            actions: vec![json!({ "type": "report_status" })],
        };
    }
    PlannerPlan {
        assistant_response: "I need a configured model to turn open-ended planning conversation into structured suggestions.".to_string(),
        needs_confirmation: false,
        clarification_question: Some(
            "Configure a model, or tell me explicitly what product, capability, or work item you want me to assess.".to_string(),
        ),
        actions: vec![],
    }
}

pub(crate) fn is_informational_only(plan: &PlannerPlan) -> bool {
    !plan.actions.is_empty()
        && plan.actions.iter().all(|action| {
            matches!(
                action.get("type").and_then(Value::as_str),
                Some("report_status") | Some("report_tree")
            )
        })
}

pub(crate) fn has_draft_mutations(plan: &PlannerPlan) -> bool {
    plan.actions.iter().any(|action| {
        matches!(
            action.get("type").and_then(Value::as_str),
            Some(
                "create_product"
                    | "create_product_area"
                    | "create_capability"
                    | "apply_capability_template"
                    | "convert_capability_kind"
                    | "create_work_item"
                    | "update_product"
                    | "update_product_area"
                    | "update_capability"
                    | "update_work_item"
                    | "archive_product"
                    | "delete_product_area"
                    | "delete_capability"
                    | "delete_work_item"
            )
        )
    })
}

pub(crate) fn requires_confirmation(plan: &PlannerPlan) -> bool {
    !plan.actions.is_empty() && (plan.needs_confirmation || !is_informational_only(plan))
}

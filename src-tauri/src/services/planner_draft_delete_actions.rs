use crate::error::AppError;
use crate::services::planner_action_fields::target_field;
use crate::services::planner_draft::{
    find_draft_node, resolve_draft_capability_node_id, resolve_draft_product_area_name,
    resolve_draft_product_name,
};
use crate::services::planner_draft_mutation::remove_draft_node_subtree;
use crate::services::planner_service::PlannerDraftPlan;
use serde_json::Value;

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
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

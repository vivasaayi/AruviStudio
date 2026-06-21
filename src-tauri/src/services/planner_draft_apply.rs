use crate::error::AppError;
use crate::services::planner_draft_actions::{
    apply_create_draft_action, apply_delete_draft_action, apply_update_draft_action,
    build_capability_template_actions, convert_draft_capability_kind,
};
use crate::services::planner_types::PlannerDraftPlan;
use serde_json::Value;

pub(crate) fn apply_actions_to_draft(
    draft_plan: Option<PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
    actions: &[Value],
) -> Result<PlannerDraftPlan, AppError> {
    let mut draft_plan = draft_plan.unwrap_or(PlannerDraftPlan { nodes: vec![] });

    for action in actions {
        let action_type = action
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("Planner action missing type".to_string()))?;
        match action_type {
            "create_product" | "create_product_area" | "create_capability" | "create_work_item" => {
                apply_create_draft_action(
                    &mut draft_plan,
                    selected_draft_node_id,
                    action_type,
                    action,
                )?;
            }
            "update_product" | "update_product_area" | "update_capability" | "update_work_item" => {
                apply_update_draft_action(
                    &mut draft_plan,
                    selected_draft_node_id,
                    action_type,
                    action,
                )?;
            }
            "apply_capability_template" => {
                let template_actions =
                    build_capability_template_actions(&draft_plan, selected_draft_node_id, action)?;
                draft_plan = apply_actions_to_draft(
                    Some(draft_plan),
                    selected_draft_node_id,
                    &template_actions,
                )?;
            }
            "convert_capability_kind" => {
                convert_draft_capability_kind(&mut draft_plan, selected_draft_node_id, action)?;
            }
            "archive_product"
            | "delete_product_area"
            | "delete_capability"
            | "delete_work_item" => {
                apply_delete_draft_action(
                    &mut draft_plan,
                    selected_draft_node_id,
                    action_type,
                    action,
                )?;
            }
            "report_tree" | "report_status" => {}
            _ => {}
        }
    }

    Ok(draft_plan)
}

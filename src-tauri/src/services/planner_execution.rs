use crate::error::AppError;
use crate::services::planner_execution_catalog::execute_catalog_action;
use crate::services::planner_execution_reports::execute_report_action;
use crate::services::planner_execution_work_items::execute_work_item_action;
use crate::services::planner_service::PlannerPlan;
use crate::state::AppState;
use serde_json::Value;

async fn execute_action(state: &AppState, action: &Value) -> Result<Vec<String>, AppError> {
    let action_type = action
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Planner action missing type".to_string()))?;
    match action_type {
        "create_product"
        | "update_product"
        | "archive_product"
        | "create_product_area"
        | "update_product_area"
        | "delete_product_area"
        | "create_capability"
        | "update_capability"
        | "delete_capability"
        | "apply_capability_template"
        | "convert_capability_kind" => execute_catalog_action(state, action_type, action).await,
        "create_work_item"
        | "update_work_item"
        | "delete_work_item"
        | "approve_work_item"
        | "reject_work_item"
        | "approve_work_item_plan"
        | "reject_work_item_plan"
        | "approve_work_item_test_review"
        | "start_workflow"
        | "workflow_action" => execute_work_item_action(state, action_type, action).await,
        "report_status" | "report_tree" => execute_report_action(state, action_type, action).await,
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

pub(crate) async fn execute_plan(
    state: &AppState,
    plan: &PlannerPlan,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut lines = vec![];
    let mut errors = vec![];
    for action in &plan.actions {
        match execute_action(state, action).await {
            Ok(mut action_lines) => lines.append(&mut action_lines),
            Err(error) => errors.push(error.to_string()),
        }
    }
    Ok((lines, errors))
}

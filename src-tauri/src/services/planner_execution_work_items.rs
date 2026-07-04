use crate::domain::workflow::UserAction;
use crate::error::AppError;
use crate::persistence::{approval_repo, settings_repo, work_item_repo, workflow_repo};
use crate::services::planner_action_fields::{fields_string, string_field, target_field};
use crate::services::planner_catalog::{
    find_capability, find_product, find_product_area, find_work_item,
};
use crate::state::AppState;
use serde_json::Value;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

pub(crate) async fn execute_work_item_action(
    state: &AppState,
    action_type: &str,
    action: &Value,
) -> Result<Vec<String>, AppError> {
    match action_type {
        "create_work_item" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let product_area_id = if target_field(action, "productAreaName").is_some() {
                Some(
                    find_product_area(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "productAreaName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let capability_id = if target_field(action, "capabilityName").is_some() {
                Some(
                    find_capability(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "productAreaName"),
                        target_field(action, "capabilityName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let id = uuid::Uuid::new_v4().to_string();
            let title = string_field(action, "title")
                .ok_or_else(|| AppError::Validation("Missing work item title".to_string()))?;
            let problem_statement = string_field(action, "problemStatement")
                .or_else(|| string_field(action, "description"))
                .unwrap_or_default();
            let description = string_field(action, "description").unwrap_or_default();
            let acceptance_criteria =
                string_field(action, "acceptanceCriteria").unwrap_or_default();
            let constraints = string_field(action, "constraints").unwrap_or_default();
            let work_item_type =
                string_field(action, "workItemType").unwrap_or_else(|| "story".to_string());
            let priority = string_field(action, "priority").unwrap_or_else(|| "medium".to_string());
            let complexity =
                string_field(action, "complexity").unwrap_or_else(|| "medium".to_string());
            let work_item = work_item_repo::create_work_item(
                &state.db,
                work_item_repo::CreateWorkItemInput {
                    id: &id,
                    product_id: &product.id,
                    product_area_id: product_area_id.as_deref(),
                    capability_id: capability_id.as_deref(),
                    source_node_id: None,
                    source_node_type: None,
                    parent_work_item_id: None,
                    title: &title,
                    problem_statement: &problem_statement,
                    description: &description,
                    acceptance_criteria: &acceptance_criteria,
                    constraints: &constraints,
                    work_item_type: &work_item_type,
                    priority: &priority,
                    complexity: &complexity,
                },
            )
            .await?;
            Ok(vec![format!(
                "Created work item \"{}\" in \"{}\".",
                work_item.title, product.name
            )])
        }
        "update_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let title = fields_string(action, "title");
            let description = fields_string(action, "description");
            let status = fields_string(action, "status");
            let problem_statement = fields_string(action, "problemStatement");
            let acceptance_criteria = fields_string(action, "acceptanceCriteria");
            let constraints = fields_string(action, "constraints");
            let updated = work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    title: title.as_deref(),
                    description: description.as_deref(),
                    status: status.as_deref(),
                    problem_statement: problem_statement.as_deref(),
                    acceptance_criteria: acceptance_criteria.as_deref(),
                    constraints: constraints.as_deref(),
                },
            )
            .await?;
            Ok(vec![format!("Updated work item \"{}\".", updated.title)])
        }
        "delete_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            work_item_repo::delete_work_item(&state.db, &work_item.id).await?;
            Ok(vec![format!("Deleted work item \"{}\".", work_item.title)])
        }
        "approve_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    status: Some("approved"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
            )
            .await?;
            let auto_start = settings_repo::get_bool_setting(
                &state.db,
                AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY,
                true,
            )
            .await?;
            if auto_start {
                let workflow_service = state.workflow_service.lock().await;
                let _ = workflow_service
                    .start_work_item_workflow(&work_item.id)
                    .await?;
            }
            Ok(vec![format!("Approved work item \"{}\".", work_item.title)])
        }
        "reject_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    status: Some("draft"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
            )
            .await?;
            Ok(vec![format!("Rejected work item \"{}\".", work_item.title)])
        }
        "approve_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!("Approved plan for \"{}\".", work_item.title)])
        }
        "reject_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            Ok(vec![format!("Rejected plan for \"{}\".", work_item.title)])
        }
        "approve_work_item_test_review" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "test_review",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!(
                "Approved test review for \"{}\".",
                work_item.title
            )])
        }
        "start_workflow" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .start_work_item_workflow(&work_item.id)
                .await?;
            Ok(vec![format!(
                "Started workflow for \"{}\".",
                work_item.title
            )])
        }
        "workflow_action" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let run =
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(format!(
                            "No workflow run exists for {}",
                            work_item.title
                        ))
                    })?;
            let action_name = string_field(action, "action")
                .ok_or_else(|| AppError::Validation("Missing workflow action".to_string()))?;
            let user_action = match action_name.as_str() {
                "approve" => UserAction::Approve,
                "reject" => UserAction::Reject,
                "pause" => UserAction::Pause,
                "resume" => UserAction::Resume,
                "cancel" => UserAction::Cancel,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action {}",
                        action_name
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&run.id, user_action, string_field(action, "notes"))
                .await?;
            Ok(vec![format!(
                "Applied workflow action \"{}\" to \"{}\".",
                action_name, work_item.title
            )])
        }
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

use crate::domain::workflow::UserAction;
use crate::error::AppError;
use crate::persistence::{agent_repo, workflow_repo};
use crate::state::AppState;
use serde_json::{json, Value};

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "start_work_item_workflow" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let workflow_service = state.workflow_service.lock().await;
            let run = workflow_service
                .start_work_item_workflow(&work_item_id)
                .await?;
            action_result(
                "start_work_item_workflow",
                json!({ "workflow_run_id": run.id }),
            )
        }
        "get_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_run",
                workflow_service.get_workflow_run(&workflow_run_id).await?,
            )
        }
        "get_latest_workflow_run_for_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "get_latest_workflow_run_for_work_item",
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item_id)
                    .await?,
            )
        }
        "get_workflow_history" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_history",
                workflow_service
                    .get_workflow_history(&workflow_run_id)
                    .await?,
            )
        }
        "handle_workflow_user_action" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let action = match args.required_string(&["action"], "action")?.as_str() {
                "approve" => UserAction::Approve,
                "reject" => UserAction::Reject,
                "pause" => UserAction::Pause,
                "resume" => UserAction::Resume,
                "cancel" => UserAction::Cancel,
                other => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action: {other}"
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&workflow_run_id, action, args.optional_string(&["notes"])?)
                .await?;
            Ok(action_ok("handle_workflow_user_action"))
        }
        "advance_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service.advance_workflow(&workflow_run_id).await?;
            Ok(action_ok("advance_workflow"))
        }
        "list_agent_runs_for_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            action_result(
                "list_agent_runs_for_workflow",
                agent_repo::list_agent_runs_for_workflow(&state.db, &workflow_run_id).await?,
            )
        }
        "mark_workflow_run_failed" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            if run.current_stage != "failed" {
                workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "failed").await?;
                workflow_repo::record_stage_transition(
                    &state.db,
                    &uuid::Uuid::new_v4().to_string(),
                    &workflow_run_id,
                    &run.current_stage,
                    "failed",
                    "user_override",
                    args.optional_string(&["reason"])?
                        .as_deref()
                        .unwrap_or("Marked failed by MCP operator"),
                )
                .await?;
            }
            workflow_repo::update_workflow_lifecycle(
                &state.db,
                &workflow_run_id,
                "failed",
                args.optional_string(&["reason"])?.as_deref(),
                true,
            )
            .await?;
            Ok(action_ok("mark_workflow_run_failed"))
        }
        "restart_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            let workflow_service = state.workflow_service.lock().await;
            let next = workflow_service
                .start_work_item_workflow(&run.work_item_id)
                .await?;
            action_result(
                "restart_workflow_run",
                json!({ "workflow_run_id": next.id }),
            )
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_workflows action: {other}"
        ))),
    }
}

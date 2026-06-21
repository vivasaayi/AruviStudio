use crate::error::AppError;
use crate::services::planner_service::{
    add_planner_draft_child, analyze_repository_for_planner, clear_planner_pending,
    confirm_planner_plan, create_planner_session, delete_planner_draft_node,
    rename_planner_draft_node, submit_planner_turn, submit_planner_voice_turn,
    update_planner_session,
};
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::action_result;

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_planner_session" => action_result(
            "create_planner_session",
            create_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "update_planner_session" => action_result(
            "update_planner_session",
            update_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "clear_planner_pending" => action_result(
            "clear_planner_pending",
            clear_planner_pending(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "submit_planner_turn" => action_result(
            "submit_planner_turn",
            submit_planner_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["user_input", "userInput"], "user_input")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        "submit_planner_voice_turn" => action_result(
            "submit_planner_voice_turn",
            submit_planner_voice_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["transcript", "user_input", "userInput"], "transcript")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        "confirm_planner_plan" => action_result(
            "confirm_planner_plan",
            confirm_planner_plan(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "rename_planner_draft_node" => action_result(
            "rename_planner_draft_node",
            rename_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
                args.required_string(&["name"], "name")?,
            )
            .await?,
        ),
        "add_planner_draft_child" => action_result(
            "add_planner_draft_child",
            add_planner_draft_child(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["parent_node_id", "parentNodeId"], "parent_node_id")?,
                args.required_string(&["child_type", "childType"], "child_type")?,
                args.required_string(&["name"], "name")?,
                args.optional_string(&["summary"])?,
            )
            .await?,
        ),
        "delete_planner_draft_node" => action_result(
            "delete_planner_draft_node",
            delete_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
            )
            .await?,
        ),
        "analyze_repository_for_planner" => action_result(
            "analyze_repository_for_planner",
            analyze_repository_for_planner(
                state.planner_service.clone(),
                &state.db,
                &state.artifact_base_path,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_planner action: {other}"
        ))),
    }
}

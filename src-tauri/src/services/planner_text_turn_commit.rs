use crate::error::AppError;
use crate::services::planner_commit::commit_draft_plan;
use crate::services::planner_response::push_trace;
use crate::services::planner_session::{
    append_conversation, persist_draft_state, persist_pending_plan, PlannerConversationEntry,
    PlannerService, PlannerSession,
};
use crate::services::planner_text_turn_response::planner_error_response;
use crate::services::planner_types::{PlannerTraceEvent, PlannerTurnResponse};
use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) async fn commit_confirmed_draft_plan(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: &str,
    mut session: PlannerSession,
    mut trace: Vec<PlannerTraceEvent>,
) -> Result<PlannerTurnResponse, AppError> {
    let Some(draft_plan) = session.draft_plan.clone() else {
        return Ok(planner_error_response(
            session_id,
            &session,
            "No staged design is available to apply.".to_string(),
            None,
            trace,
        ));
    };

    push_trace(
        &mut trace,
        "commit",
        "Attempting draft commit",
        serde_json::to_string_pretty(&draft_plan)?,
    );
    let execution_lines = match commit_draft_plan(state, &draft_plan).await {
        Ok(lines) => lines,
        Err(error) => {
            push_trace(
                &mut trace,
                "error",
                "Draft commit failed",
                error.to_string(),
            );
            return Ok(planner_error_response(
                session_id,
                &session,
                error.to_string(),
                None,
                trace,
            ));
        }
    };
    append_conversation(&state.db, &session_id, "user", user_input).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: user_input.to_string(),
    });
    append_conversation(
        &state.db,
        &session_id,
        "assistant",
        "Applied design to catalog.",
    )
    .await?;
    session.conversation.push(PlannerConversationEntry {
        role: "assistant".to_string(),
        content: "Applied design to catalog.".to_string(),
    });
    session.pending_plan = None;
    session.draft_plan = None;
    session.selected_draft_node_id = None;
    persist_pending_plan(&state.db, &session_id, None).await?;
    persist_draft_state(&state.db, &session_id, None, None).await?;
    let mut service = planner_service.lock().await;
    service.save_session(&session_id, session);
    Ok(PlannerTurnResponse {
        session_id,
        status: "execution".to_string(),
        assistant_message: "Applied design to catalog.".to_string(),
        pending_plan: None,
        tree_nodes: None,
        draft_tree_nodes: None,
        selected_draft_node_id: None,
        execution_lines,
        execution_errors: vec![],
        trace_events: trace,
    })
}

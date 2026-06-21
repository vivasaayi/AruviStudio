use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_session::PlannerSession;
use crate::services::planner_types::{PlannerTraceEvent, PlannerTurnResponse};

pub(crate) fn push_trace(
    trace: &mut Vec<PlannerTraceEvent>,
    stage: impl Into<String>,
    title: impl Into<String>,
    detail: impl Into<String>,
) {
    let step = trace.len() + 1;
    trace.push(PlannerTraceEvent {
        step,
        stage: stage.into(),
        title: title.into(),
        detail: detail.into(),
    });
}

pub(crate) fn build_session_state_response(
    session_id: String,
    status: &str,
    assistant_message: String,
    session: &PlannerSession,
    execution_lines: Vec<String>,
    execution_errors: Vec<String>,
    trace_events: Vec<PlannerTraceEvent>,
) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id,
        status: status.to_string(),
        assistant_message,
        pending_plan: session.pending_plan.clone(),
        tree_nodes: None,
        draft_tree_nodes: session
            .draft_plan
            .as_ref()
            .map(|draft| build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())),
        selected_draft_node_id: session.selected_draft_node_id.clone(),
        execution_lines,
        execution_errors,
        trace_events,
    }
}

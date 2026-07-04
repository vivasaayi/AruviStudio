use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_session::PlannerSession;
use crate::services::planner_types::{
    PlannerPlan, PlannerTraceEvent, PlannerTreeNode, PlannerTurnResponse,
};

pub(crate) fn session_draft_tree_nodes(session: &PlannerSession) -> Option<Vec<PlannerTreeNode>> {
    session
        .draft_plan
        .as_ref()
        .map(|draft| build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref()))
}

pub(crate) fn planner_error_response(
    session_id: String,
    session: &PlannerSession,
    assistant_message: String,
    tree_nodes: Option<Vec<PlannerTreeNode>>,
    trace_events: Vec<PlannerTraceEvent>,
) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id,
        status: "error".to_string(),
        assistant_message: assistant_message.clone(),
        pending_plan: session.pending_plan.clone(),
        tree_nodes,
        draft_tree_nodes: session_draft_tree_nodes(session),
        selected_draft_node_id: session.selected_draft_node_id.clone(),
        execution_lines: vec![],
        execution_errors: vec![assistant_message],
        trace_events,
    }
}

pub(crate) fn selection_required_response(
    session_id: String,
    session: &PlannerSession,
    trace_events: Vec<PlannerTraceEvent>,
) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id,
        status: "clarification".to_string(),
        assistant_message:
            "Select a product before planning. Create the product in Products first, then return to Planner."
                .to_string(),
        pending_plan: session.pending_plan.clone(),
        tree_nodes: None,
        draft_tree_nodes: None,
        selected_draft_node_id: session.selected_draft_node_id.clone(),
        execution_lines: vec![],
        execution_errors: vec![],
        trace_events,
    }
}

pub(crate) fn proposal_response(
    session_id: String,
    assistant_message: String,
    pending_plan: PlannerPlan,
    tree_nodes: Option<Vec<PlannerTreeNode>>,
    draft_tree_nodes: Option<Vec<PlannerTreeNode>>,
    selected_draft_node_id: Option<String>,
    execution_lines: Vec<String>,
    trace_events: Vec<PlannerTraceEvent>,
) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id,
        status: "proposal".to_string(),
        assistant_message,
        pending_plan: Some(pending_plan),
        tree_nodes,
        draft_tree_nodes,
        selected_draft_node_id,
        execution_lines,
        execution_errors: vec![],
        trace_events,
    }
}

pub(crate) fn clarification_response(
    session_id: String,
    assistant_message: String,
    pending_plan: Option<PlannerPlan>,
    tree_nodes: Option<Vec<PlannerTreeNode>>,
    draft_tree_nodes: Option<Vec<PlannerTreeNode>>,
    selected_draft_node_id: Option<String>,
    trace_events: Vec<PlannerTraceEvent>,
) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id,
        status: "clarification".to_string(),
        assistant_message,
        pending_plan,
        tree_nodes,
        draft_tree_nodes,
        selected_draft_node_id,
        execution_lines: vec![],
        execution_errors: vec![],
        trace_events,
    }
}

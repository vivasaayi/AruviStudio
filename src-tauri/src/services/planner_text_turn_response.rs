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

pub(crate) struct ProposalResponseInput {
    pub(crate) session_id: String,
    pub(crate) assistant_message: String,
    pub(crate) pending_plan: PlannerPlan,
    pub(crate) tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub(crate) draft_tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub(crate) selected_draft_node_id: Option<String>,
    pub(crate) execution_lines: Vec<String>,
    pub(crate) trace_events: Vec<PlannerTraceEvent>,
}

pub(crate) fn proposal_response(input: ProposalResponseInput) -> PlannerTurnResponse {
    PlannerTurnResponse {
        session_id: input.session_id,
        status: "proposal".to_string(),
        assistant_message: input.assistant_message,
        pending_plan: Some(input.pending_plan),
        tree_nodes: input.tree_nodes,
        draft_tree_nodes: input.draft_tree_nodes,
        selected_draft_node_id: input.selected_draft_node_id,
        execution_lines: input.execution_lines,
        execution_errors: vec![],
        trace_events: input.trace_events,
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

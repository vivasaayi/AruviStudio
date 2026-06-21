use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerPlan {
    pub assistant_response: String,
    pub needs_confirmation: bool,
    pub clarification_question: Option<String>,
    pub actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTreeNode {
    pub id: String,
    pub label: String,
    pub meta: Option<String>,
    pub node_type: Option<String>,
    pub summary: Option<String>,
    pub source: Option<String>,
    pub confidence: Option<String>,
    pub evidence: Vec<String>,
    pub children: Vec<PlannerTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTraceEvent {
    pub step: usize,
    pub stage: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDraftNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub node_type: String,
    pub name: String,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDraftPlan {
    pub nodes: Vec<PlannerDraftNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTurnResponse {
    pub session_id: String,
    pub status: String,
    pub assistant_message: String,
    pub pending_plan: Option<PlannerPlan>,
    pub tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub draft_tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub selected_draft_node_id: Option<String>,
    pub execution_lines: Vec<String>,
    pub execution_errors: Vec<String>,
    pub trace_events: Vec<PlannerTraceEvent>,
}

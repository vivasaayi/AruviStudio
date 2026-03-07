use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    ProductCreated {
        product_id: String,
    },
    WorkItemCreated {
        work_item_id: String,
    },
    WorkItemStatusChanged {
        work_item_id: String,
        old_status: String,
        new_status: String,
    },
    WorkflowStarted {
        workflow_run_id: String,
        work_item_id: String,
    },
    WorkflowStageChanged {
        workflow_run_id: String,
        from: String,
        to: String,
    },
    WorkflowCompleted {
        workflow_run_id: String,
    },
    WorkflowFailed {
        workflow_run_id: String,
        error: String,
    },
    AgentRunStarted {
        agent_run_id: String,
        stage: String,
    },
    AgentRunCompleted {
        agent_run_id: String,
    },
    ApprovalRequired {
        work_item_id: String,
        approval_type: String,
    },
    ApprovalDecision {
        approval_id: String,
        decision: String,
    },
    ArtifactCreated {
        artifact_id: String,
        work_item_id: String,
    },
}

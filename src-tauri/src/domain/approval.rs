use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Approval {
    pub id: String,
    pub work_item_id: String,
    pub workflow_run_id: Option<String>,
    pub approval_type: ApprovalType,
    pub status: ApprovalStatus,
    pub notes: String,
    pub acted_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ApprovalType {
    TaskApproval,
    PlanApproval,
    TestReview,
}

impl std::fmt::Display for ApprovalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalType::TaskApproval => write!(f, "task_approval"),
            ApprovalType::PlanApproval => write!(f, "plan_approval"),
            ApprovalType::TestReview => write!(f, "test_review"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "pending"),
            ApprovalStatus::Approved => write!(f, "approved"),
            ApprovalStatus::Rejected => write!(f, "rejected"),
        }
    }
}

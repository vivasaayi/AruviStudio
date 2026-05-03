use serde::{Deserialize, Serialize};

use super::product::HierarchyNodeType;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkItem {
    pub id: String,
    pub product_id: Option<String>,
    pub module_id: Option<String>,
    pub capability_id: Option<String>,
    pub source_node_id: Option<String>,
    pub source_node_type: Option<HierarchyNodeType>,
    pub parent_work_item_id: Option<String>,
    pub title: String,
    pub problem_statement: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub constraints: String,
    pub work_item_type: WorkItemType,
    pub priority: super::product::Priority,
    pub complexity: Complexity,
    pub status: WorkItemStatus,
    pub repo_override_id: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch_name: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductWorkItemSummary {
    pub product_id: String,
    pub total_count: i64,
    pub active_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorkItemType {
    #[serde(rename = "feature")]
    #[sqlx(rename = "feature")]
    CapabilityDelivery,
    Setup,
    Bug,
    Refactor,
    Test,
    Review,
    SecurityFix,
    PerformanceImprovement,
}

impl std::fmt::Display for WorkItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkItemType::CapabilityDelivery => write!(f, "capability_delivery"),
            WorkItemType::Setup => write!(f, "setup"),
            WorkItemType::Bug => write!(f, "bug"),
            WorkItemType::Refactor => write!(f, "refactor"),
            WorkItemType::Test => write!(f, "test"),
            WorkItemType::Review => write!(f, "review"),
            WorkItemType::SecurityFix => write!(f, "security_fix"),
            WorkItemType::PerformanceImprovement => write!(f, "performance_improvement"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Complexity {
    Trivial,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Complexity::Trivial => write!(f, "trivial"),
            Complexity::Low => write!(f, "low"),
            Complexity::Medium => write!(f, "medium"),
            Complexity::High => write!(f, "high"),
            Complexity::VeryHigh => write!(f, "very_high"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorkItemStatus {
    Draft,
    ReadyForReview,
    Approved,
    InPlanning,
    InProgress,
    InValidation,
    WaitingHumanReview,
    Done,
    Blocked,
    Failed,
    Cancelled,
}

impl std::fmt::Display for WorkItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkItemStatus::Draft => write!(f, "draft"),
            WorkItemStatus::ReadyForReview => write!(f, "ready_for_review"),
            WorkItemStatus::Approved => write!(f, "approved"),
            WorkItemStatus::InPlanning => write!(f, "in_planning"),
            WorkItemStatus::InProgress => write!(f, "in_progress"),
            WorkItemStatus::InValidation => write!(f, "in_validation"),
            WorkItemStatus::WaitingHumanReview => write!(f, "waiting_human_review"),
            WorkItemStatus::Done => write!(f, "done"),
            WorkItemStatus::Blocked => write!(f, "blocked"),
            WorkItemStatus::Failed => write!(f, "failed"),
            WorkItemStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

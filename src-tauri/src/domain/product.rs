use serde::{Deserialize, Serialize};

mod hierarchy;
pub use hierarchy::{
    CapabilityTree, ChildReparentStrategy, HierarchyNodeKind, HierarchyNodeType, HierarchyTreeNode,
    NodeKindConversionResult, ProductAreaTree, ProductTree, ProductTreeSummary,
    SemanticTemplateApplicationResult, SemanticTemplateKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub goals: Vec<String>,
    pub tags: Vec<String>,
    pub status: ProductStatus,
    pub lifecycle: ProductLifecycle,
    pub health: ProductHealth,
    pub owner_label: String,
    pub investment_status: ProductInvestmentStatus,
    pub roadmap: String,
    pub evidence: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPlanResetResult {
    pub product_id: String,
    pub product_areas_deleted: i64,
    pub capabilities_deleted: i64,
    pub work_items_deleted: i64,
    pub agent_work_runs_deleted: i64,
    pub agent_work_items_deleted: i64,
    pub agent_work_events_deleted: i64,
    pub agent_work_evidence_deleted: i64,
    pub agent_work_dependencies_deleted: i64,
    pub agent_work_locks_deleted: i64,
    pub agent_work_batches_deleted: i64,
}

impl Product {
    pub fn is_example_product(&self) -> bool {
        self.tags.iter().any(|tag| tag == "example_product")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductStatus {
    Active,
    Archived,
}

impl std::fmt::Display for ProductStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductStatus::Active => write!(f, "active"),
            ProductStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductLifecycle {
    Idea,
    Incubating,
    Active,
    Maturing,
    Sunsetting,
    Retired,
}

impl std::fmt::Display for ProductLifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductLifecycle::Idea => write!(f, "idea"),
            ProductLifecycle::Incubating => write!(f, "incubating"),
            ProductLifecycle::Active => write!(f, "active"),
            ProductLifecycle::Maturing => write!(f, "maturing"),
            ProductLifecycle::Sunsetting => write!(f, "sunsetting"),
            ProductLifecycle::Retired => write!(f, "retired"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductHealth {
    Unknown,
    Healthy,
    Watch,
    AtRisk,
    Blocked,
}

impl std::fmt::Display for ProductHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductHealth::Unknown => write!(f, "unknown"),
            ProductHealth::Healthy => write!(f, "healthy"),
            ProductHealth::Watch => write!(f, "watch"),
            ProductHealth::AtRisk => write!(f, "at_risk"),
            ProductHealth::Blocked => write!(f, "blocked"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductInvestmentStatus {
    Evaluate,
    Invest,
    Maintain,
    Pause,
    Retire,
}

impl std::fmt::Display for ProductInvestmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductInvestmentStatus::Evaluate => write!(f, "evaluate"),
            ProductInvestmentStatus::Invest => write!(f, "invest"),
            ProductInvestmentStatus::Maintain => write!(f, "maintain"),
            ProductInvestmentStatus::Pause => write!(f, "pause"),
            ProductInvestmentStatus::Retire => write!(f, "retire"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductArea {
    pub id: String,
    pub product_id: String,
    pub node_kind: HierarchyNodeKind,
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub explanation: String,
    pub examples: String,
    pub implementation_notes: String,
    pub test_guidance: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Capability {
    pub id: String,
    pub product_area_id: String,
    pub parent_capability_id: Option<String>,
    pub level: i32,
    pub node_kind: HierarchyNodeKind,
    pub sort_order: i32,
    pub name: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub explanation: String,
    pub examples: String,
    pub priority: Priority,
    pub risk: Risk,
    pub status: CapabilityStatus,
    pub technical_notes: String,
    pub implementation_notes: String,
    pub test_guidance: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Risk {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Risk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Risk::High => write!(f, "high"),
            Risk::Medium => write!(f, "medium"),
            Risk::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Draft,
    InProgress,
    Done,
    Archived,
}

impl std::fmt::Display for CapabilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityStatus::Draft => write!(f, "draft"),
            CapabilityStatus::InProgress => write!(f, "in_progress"),
            CapabilityStatus::Done => write!(f, "done"),
            CapabilityStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductReference {
    pub id: String,
    pub scope_type: String,
    pub scope_id: String,
    pub title: String,
    pub reference_kind: String,
    pub uri: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

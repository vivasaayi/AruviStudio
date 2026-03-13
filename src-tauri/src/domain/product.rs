use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub goals: Vec<String>,
    pub tags: Vec<String>,
    pub status: ProductStatus,
    pub created_at: String,
    pub updated_at: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Module {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Capability {
    pub id: String,
    pub module_id: String,
    pub parent_capability_id: Option<String>,
    pub level: i32,
    pub sort_order: i32,
    pub name: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub priority: Priority,
    pub risk: Risk,
    pub status: CapabilityStatus,
    pub technical_notes: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductTree {
    pub product: Product,
    pub modules: Vec<ModuleTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleTree {
    pub module: Module,
    pub features: Vec<CapabilityTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTree {
    pub capability: Capability,
    pub children: Vec<CapabilityTree>,
}

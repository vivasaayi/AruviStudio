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
    pub node_kind: HierarchyNodeKind,
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
    pub node_kind: HierarchyNodeKind,
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
    pub roots: Vec<HierarchyTreeNode>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum HierarchyNodeKind {
    Area,
    Domain,
    Subdomain,
    System,
    Subsystem,
    FeatureSet,
    Capability,
    Rollout,
    Reference,
}

impl HierarchyNodeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "area" => Some(Self::Area),
            "domain" => Some(Self::Domain),
            "subdomain" => Some(Self::Subdomain),
            "system" => Some(Self::System),
            "subsystem" => Some(Self::Subsystem),
            "feature_set" => Some(Self::FeatureSet),
            "capability" => Some(Self::Capability),
            "rollout" => Some(Self::Rollout),
            "reference" => Some(Self::Reference),
            _ => None,
        }
    }

    pub fn default_root() -> Self {
        Self::Area
    }

    pub fn default_child(parent_kind: &Self) -> Self {
        match parent_kind {
            Self::Capability => Self::Rollout,
            Self::FeatureSet => Self::Capability,
            Self::Area | Self::Domain | Self::Subdomain | Self::System | Self::Subsystem => {
                Self::Capability
            }
            Self::Rollout | Self::Reference => Self::Reference,
        }
    }

    pub fn is_root_kind(&self) -> bool {
        matches!(self, Self::Area | Self::Domain | Self::System)
    }

    pub fn can_have_children(&self) -> bool {
        !matches!(self, Self::Rollout | Self::Reference)
    }

    pub fn allowed_child_kinds(&self) -> Vec<Self> {
        match self {
            Self::Area => vec![
                Self::Area,
                Self::Domain,
                Self::System,
                Self::Subsystem,
                Self::FeatureSet,
                Self::Capability,
                Self::Reference,
            ],
            Self::Domain => vec![
                Self::Subdomain,
                Self::System,
                Self::Subsystem,
                Self::FeatureSet,
                Self::Capability,
                Self::Reference,
            ],
            Self::Subdomain => vec![
                Self::Subdomain,
                Self::FeatureSet,
                Self::Capability,
                Self::Reference,
            ],
            Self::System => vec![
                Self::Subsystem,
                Self::FeatureSet,
                Self::Capability,
                Self::Reference,
            ],
            Self::Subsystem => vec![
                Self::Subsystem,
                Self::FeatureSet,
                Self::Capability,
                Self::Reference,
            ],
            Self::FeatureSet => vec![
                Self::FeatureSet,
                Self::Capability,
                Self::Rollout,
                Self::Reference,
            ],
            Self::Capability => vec![
                Self::FeatureSet,
                Self::Capability,
                Self::Rollout,
                Self::Reference,
            ],
            Self::Rollout | Self::Reference => Vec::new(),
        }
    }

    pub fn supports_child_kind(&self, child_kind: &Self) -> bool {
        self.allowed_child_kinds().contains(child_kind)
    }
}

impl std::fmt::Display for HierarchyNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Area => "area",
            Self::Domain => "domain",
            Self::Subdomain => "subdomain",
            Self::System => "system",
            Self::Subsystem => "subsystem",
            Self::FeatureSet => "feature_set",
            Self::Capability => "capability",
            Self::Rollout => "rollout",
            Self::Reference => "reference",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum HierarchyNodeType {
    Module,
    Capability,
}

impl std::fmt::Display for HierarchyNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Capability => write!(f, "capability"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyTreeNode {
    pub id: String,
    pub node_type: HierarchyNodeType,
    pub node_kind: HierarchyNodeKind,
    pub module_id: String,
    pub capability_id: Option<String>,
    pub parent_node_id: Option<String>,
    pub parent_node_type: Option<HierarchyNodeType>,
    pub depth: i32,
    pub name: String,
    pub description: String,
    pub summary: String,
    pub path: Vec<String>,
    pub allowed_child_kinds: Vec<HierarchyNodeKind>,
    pub children: Vec<HierarchyTreeNode>,
}

use super::{Capability, Product, ProductArea};
use crate::domain::work_item::WorkItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductTree {
    pub product: Product,
    pub product_areas: Vec<ProductAreaTree>,
    pub roots: Vec<HierarchyTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductTreeSummary {
    pub product_id: String,
    pub product_area_count: i64,
    pub capability_count: i64,
    pub total_node_count: i64,
    pub leaf_node_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductAreaTree {
    pub product_area: ProductArea,
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
    ProductArea,
    Capability,
    Feature,
}

impl HierarchyNodeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "product_area" => Some(Self::ProductArea),
            "capability" => Some(Self::Capability),
            "feature" => Some(Self::Feature),
            _ => None,
        }
    }

    pub fn default_root() -> Self {
        Self::ProductArea
    }

    pub fn default_child(parent_kind: &Self) -> Self {
        match parent_kind {
            Self::ProductArea => Self::Capability,
            Self::Capability => Self::Feature,
            Self::Feature => Self::Feature,
        }
    }

    pub fn is_root_kind(&self) -> bool {
        matches!(self, Self::ProductArea)
    }

    pub fn can_have_children(&self) -> bool {
        matches!(self, Self::ProductArea | Self::Capability)
    }

    pub fn allowed_child_kinds(&self) -> Vec<Self> {
        match self {
            Self::ProductArea => vec![Self::Capability],
            Self::Capability => vec![Self::Feature],
            Self::Feature => Vec::new(),
        }
    }

    pub fn supports_child_kind(&self, child_kind: &Self) -> bool {
        self.allowed_child_kinds().contains(child_kind)
    }
}

impl std::fmt::Display for HierarchyNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::ProductArea => "product_area",
            Self::Capability => "capability",
            Self::Feature => "feature",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum HierarchyNodeType {
    ProductArea,
    Capability,
}

impl std::fmt::Display for HierarchyNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProductArea => write!(f, "product_area"),
            Self::Capability => write!(f, "capability"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyTreeNode {
    pub id: String,
    pub node_type: HierarchyNodeType,
    pub node_kind: HierarchyNodeKind,
    pub product_area_id: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTemplateKind {
    OperatorChapter,
    TechnicalTopicBook,
}

impl SemanticTemplateKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "operator_chapter" => Some(Self::OperatorChapter),
            "technical_topic_book" | "book_topic" => Some(Self::TechnicalTopicBook),
            _ => None,
        }
    }
}

impl std::fmt::Display for SemanticTemplateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OperatorChapter => write!(f, "operator_chapter"),
            Self::TechnicalTopicBook => write!(f, "technical_topic_book"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChildReparentStrategy {
    Reject,
    ReparentToParent,
}

impl ChildReparentStrategy {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reject" => Some(Self::Reject),
            "reparent_to_parent" => Some(Self::ReparentToParent),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChildReparentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reject => write!(f, "reject"),
            Self::ReparentToParent => write!(f, "reparent_to_parent"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTemplateApplicationResult {
    pub template_kind: SemanticTemplateKind,
    pub parent_node_id: String,
    pub parent_node_type: HierarchyNodeType,
    pub topic_node: Capability,
    pub created_nodes: Vec<Capability>,
    pub created_work_items: Vec<WorkItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeKindConversionResult {
    pub capability: Capability,
    pub previous_node_kind: HierarchyNodeKind,
    pub child_strategy: Option<ChildReparentStrategy>,
    pub reparented_children: Vec<Capability>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum StrategyNodeKind {
    StrategicProductArea,
    Domain,
    SubDomain,
}

impl StrategyNodeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "strategic_product_area" => Some(Self::StrategicProductArea),
            "domain" => Some(Self::Domain),
            "sub_domain" => Some(Self::SubDomain),
            _ => None,
        }
    }

    pub fn supports_child_kind(&self, child_kind: &Self) -> bool {
        matches!(
            (self, child_kind),
            (Self::StrategicProductArea, Self::Domain) | (Self::Domain, Self::SubDomain)
        )
    }

    pub fn is_root_kind(&self) -> bool {
        matches!(self, Self::StrategicProductArea)
    }
}

impl std::fmt::Display for StrategyNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::StrategicProductArea => "strategic_product_area",
            Self::Domain => "domain",
            Self::SubDomain => "sub_domain",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StrategyNode {
    pub id: String,
    pub parent_node_id: Option<String>,
    pub node_kind: StrategyNodeKind,
    pub name: String,
    pub description: String,
    pub owner_label: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductStrategyLink {
    pub id: String,
    pub product_id: String,
    pub strategy_node_id: String,
    pub is_primary: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductDependencyKind {
    Platform,
    Capability,
    Data,
    Integration,
    Operational,
    Other,
}

impl ProductDependencyKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "platform" => Some(Self::Platform),
            "capability" => Some(Self::Capability),
            "data" => Some(Self::Data),
            "integration" => Some(Self::Integration),
            "operational" => Some(Self::Operational),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProductDependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Platform => "platform",
            Self::Capability => "capability",
            Self::Data => "data",
            Self::Integration => "integration",
            Self::Operational => "operational",
            Self::Other => "other",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProductDependencyStatus {
    Active,
    Planned,
    Blocked,
    Retired,
}

impl ProductDependencyStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "planned" => Some(Self::Planned),
            "blocked" => Some(Self::Blocked),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProductDependencyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Active => "active",
            Self::Planned => "planned",
            Self::Blocked => "blocked",
            Self::Retired => "retired",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductDependency {
    pub id: String,
    pub product_id: String,
    pub capability_id: Option<String>,
    pub depends_on_product_id: String,
    pub depends_on_capability_id: Option<String>,
    pub dependency_kind: ProductDependencyKind,
    pub description: String,
    pub status: ProductDependencyStatus,
    pub created_at: String,
    pub updated_at: String,
}

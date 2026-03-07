use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub remote_url: String,
    pub default_branch: String,
    pub auth_profile: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAttachment {
    pub id: String,
    pub scope_type: ScopeType,
    pub scope_id: String,
    pub repository_id: String,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ScopeType {
    Product,
    Module,
}

impl std::fmt::Display for ScopeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeType::Product => write!(f, "product"),
            ScopeType::Module => write!(f, "module"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryTreeNode {
    pub name: String,
    pub relative_path: String,
    pub node_type: String,
    pub size_bytes: Option<u64>,
    pub children: Vec<RepositoryTreeNode>,
}

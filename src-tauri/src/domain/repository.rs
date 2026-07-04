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
pub struct RepositoryTreeNode {
    pub name: String,
    pub relative_path: String,
    pub node_type: String,
    pub size_bytes: Option<u64>,
    pub children: Vec<RepositoryTreeNode>,
}

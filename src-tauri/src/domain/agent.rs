use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub prompt_template_ref: String,
    pub allowed_tools: Vec<String>,
    pub skill_tags: Vec<String>,
    pub boundaries: serde_json::Value,
    pub enabled: bool,
    pub employment_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentTeam {
    pub id: String,
    pub name: String,
    pub department: String,
    pub description: String,
    pub enabled: bool,
    pub max_concurrent_workflows: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentTeamMembership {
    pub id: String,
    pub team_id: String,
    pub agent_id: String,
    pub title: String,
    pub is_lead: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamAssignment {
    pub id: String,
    pub team_id: String,
    pub scope_type: String,
    pub scope_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub instructions: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentSkillLink {
    pub id: String,
    pub agent_id: String,
    pub skill_id: String,
    pub proficiency: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamSkillLink {
    pub id: String,
    pub team_id: String,
    pub skill_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkflowStagePolicy {
    pub id: String,
    pub stage_name: String,
    pub primary_roles: Vec<String>,
    pub fallback_roles: Vec<String>,
    pub coordinator_required: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentModelBinding {
    pub id: String,
    pub agent_id: String,
    pub model_id: String,
    pub priority: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentRun {
    pub id: String,
    pub workflow_run_id: String,
    pub agent_id: String,
    pub stage: String,
    pub status: AgentRunStatus,
    pub prompt_snapshot_path: Option<String>,
    pub output_snapshot_path: Option<String>,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AgentRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

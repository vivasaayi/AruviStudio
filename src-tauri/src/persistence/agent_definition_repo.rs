use crate::domain::agent::AgentDefinition;
use crate::error::AppError;
use sqlx::{Row, SqlitePool};

pub(crate) fn row_to_agent_definition(row: sqlx::sqlite::SqliteRow) -> AgentDefinition {
    AgentDefinition {
        id: row.get("id"),
        name: row.get("name"),
        role: row.get("role"),
        description: row.get("description"),
        prompt_template_ref: row.get("prompt_template_ref"),
        allowed_tools: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("allowed_tools").as_str(),
        )
        .unwrap_or_default(),
        skill_tags: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("skill_tags").as_str(),
        )
        .unwrap_or_default(),
        boundaries: serde_json::from_str::<serde_json::Value>(
            row.get::<String, _>("boundaries").as_str(),
        )
        .unwrap_or_default(),
        enabled: row.get("enabled"),
        employment_status: row.get("employment_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_agent_definitions(pool: &SqlitePool) -> Result<Vec<AgentDefinition>, AppError> {
    sqlx::query("SELECT id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status,created_at,updated_at FROM agent_definitions ORDER BY name")
        .map(row_to_agent_definition)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub struct CreateAgentDefinitionInput<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub role: &'a str,
    pub description: &'a str,
    pub prompt_template_ref: &'a str,
    pub allowed_tools: &'a str,
    pub skill_tags: &'a str,
    pub boundaries: &'a str,
    pub enabled: bool,
    pub employment_status: &'a str,
}

pub async fn create_agent_definition(
    pool: &SqlitePool,
    input: CreateAgentDefinitionInput<'_>,
) -> Result<AgentDefinition, AppError> {
    sqlx::query("INSERT INTO agent_definitions (id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status) VALUES (?,?,?,?,?,?,?,?,?,?)")
        .bind(input.id)
        .bind(input.name)
        .bind(input.role)
        .bind(input.description)
        .bind(input.prompt_template_ref)
        .bind(input.allowed_tools)
        .bind(input.skill_tags)
        .bind(input.boundaries)
        .bind(input.enabled)
        .bind(input.employment_status)
        .execute(pool)
        .await?;
    get_agent_definition(pool, input.id).await
}

pub async fn get_agent_definition(
    pool: &SqlitePool,
    id: &str,
) -> Result<AgentDefinition, AppError> {
    sqlx::query("SELECT id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status,created_at,updated_at FROM agent_definitions WHERE id=?")
        .bind(id)
        .map(row_to_agent_definition)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent {id} not found")))
}

pub async fn update_agent_definition(
    pool: &SqlitePool,
    patch: UpdateAgentDefinitionPatch<'_>,
) -> Result<AgentDefinition, AppError> {
    let existing = get_agent_definition(pool, patch.id).await?;
    let existing_allowed_tools =
        serde_json::to_string(&existing.allowed_tools).unwrap_or_else(|_| "[]".to_string());
    let existing_skill_tags =
        serde_json::to_string(&existing.skill_tags).unwrap_or_else(|_| "[]".to_string());
    let existing_boundaries =
        serde_json::to_string(&existing.boundaries).unwrap_or_else(|_| "{}".to_string());

    sqlx::query(
        "UPDATE agent_definitions SET name=?, role=?, description=?, prompt_template_ref=?, allowed_tools=?, skill_tags=?, boundaries=?, enabled=?, employment_status=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(patch.name.unwrap_or(&existing.name))
    .bind(patch.role.unwrap_or(&existing.role))
    .bind(patch.description.unwrap_or(&existing.description))
    .bind(patch.prompt_template_ref.unwrap_or(&existing.prompt_template_ref))
    .bind(patch.allowed_tools.unwrap_or(&existing_allowed_tools))
    .bind(patch.skill_tags.unwrap_or(&existing_skill_tags))
    .bind(patch.boundaries.unwrap_or(&existing_boundaries))
    .bind(patch.enabled.unwrap_or(existing.enabled))
    .bind(patch.employment_status.unwrap_or(&existing.employment_status))
    .bind(patch.id)
    .execute(pool)
    .await?;

    get_agent_definition(pool, patch.id).await
}

pub struct UpdateAgentDefinitionPatch<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub role: Option<&'a str>,
    pub description: Option<&'a str>,
    pub prompt_template_ref: Option<&'a str>,
    pub allowed_tools: Option<&'a str>,
    pub skill_tags: Option<&'a str>,
    pub boundaries: Option<&'a str>,
    pub enabled: Option<bool>,
    pub employment_status: Option<&'a str>,
}

pub async fn delete_agent_definition(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_definitions WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

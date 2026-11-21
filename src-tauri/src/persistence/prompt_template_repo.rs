use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub role: String,
    pub template_text: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn validate(name: &str, role: &str, text: &str) -> Result<(), AppError> {
    if name.trim().is_empty() || name.chars().count() > 200 || role.trim().is_empty() || role.chars().count() > 100 || text.trim().is_empty() || text.chars().count() > 1000 {
        return Err(AppError::Validation("Template requires a name (up to 200 characters), role (up to 100), and instructions (up to 1000)".into()));
    }
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<PromptTemplate>, AppError> {
    let items: Vec<PromptTemplate> = sqlx::query_as("SELECT id,name,role,template_text,version,created_at,updated_at FROM prompt_templates ORDER BY name,id LIMIT 1001").fetch_all(pool).await?;
    if items.len() > 1000 { return Err(AppError::Validation("More than 1000 prompt templates exist; reduce the list before managing templates".into())); }
    Ok(items)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PromptTemplate, AppError> {
    sqlx::query_as("SELECT id,name,role,template_text,version,created_at,updated_at FROM prompt_templates WHERE id=?")
        .bind(id).fetch_optional(pool).await?.ok_or_else(|| AppError::Validation("Prompt template not found; select an existing template".into()))
}

pub async fn create(pool: &SqlitePool, name: &str, role: &str, text: &str) -> Result<PromptTemplate, AppError> {
    validate(name, role, text)?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query_as("INSERT INTO prompt_templates(id,name,role,template_text) SELECT ?,?,?,? WHERE (SELECT count(*) FROM prompt_templates) < 1000 RETURNING id,name,role,template_text,version,created_at,updated_at")
        .bind(id).bind(name.trim()).bind(role.trim()).bind(text).fetch_optional(pool).await?
        .ok_or_else(|| AppError::Validation("At most 1000 prompt templates are supported".into()))
}

pub async fn update(pool: &SqlitePool, id: &str, expected_version: i64, name: &str, role: &str, text: &str) -> Result<PromptTemplate, AppError> {
    validate(name, role, text)?;
    sqlx::query_as("UPDATE prompt_templates SET name=?,role=?,template_text=?,version=version+1,updated_at=datetime('now') WHERE id=? AND version=? RETURNING id,name,role,template_text,version,created_at,updated_at")
        .bind(name.trim()).bind(role.trim()).bind(text).bind(id).bind(expected_version).fetch_optional(pool).await?
        .ok_or_else(|| AppError::Validation("Prompt template changed or was removed; refresh before saving".into()))
}

pub async fn delete(pool: &SqlitePool, id: &str, expected_version: i64) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM prompt_templates WHERE id=? AND version=? AND NOT EXISTS (SELECT 1 FROM agent_definitions WHERE prompt_template_ref='template:' || prompt_templates.id)")
        .bind(id).bind(expected_version).execute(pool).await?;
    if result.rows_affected() != 1 { return Err(AppError::Validation("Prompt template changed, was removed, or is assigned to an agent".into())); }
    Ok(())
}

/// Legacy references were inert metadata; only explicit template: references opt in.
pub async fn resolve(pool: &SqlitePool, reference: &str, agent_role: &str) -> Result<Option<String>, AppError> {
    let Some(id) = reference.strip_prefix("template:") else { return Ok(None); };
    let template = get(pool, id).await?;
    if template.role != "*" && template.role != agent_role { return Err(AppError::Validation("Prompt template role does not match agent role".into())); }
    validate(&template.name, &template.role, &template.template_text)?;
    Ok(Some(template.template_text))
}

use crate::domain::agent::{AgentSkillLink, Skill, TeamSkillLink};
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn list_skills(pool: &SqlitePool) -> Result<Vec<Skill>, AppError> {
    sqlx::query_as::<_, Skill>(
        "SELECT id,name,category,description,instructions,enabled,created_at,updated_at FROM skills ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn create_skill(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    category: &str,
    description: &str,
    instructions: &str,
    enabled: bool,
) -> Result<Skill, AppError> {
    sqlx::query("INSERT INTO skills (id,name,category,description,instructions,enabled) VALUES (?,?,?,?,?,?)")
        .bind(id)
        .bind(name)
        .bind(category)
        .bind(description)
        .bind(instructions)
        .bind(enabled)
        .execute(pool)
        .await?;
    get_skill(pool, id).await
}

pub async fn get_skill(pool: &SqlitePool, id: &str) -> Result<Skill, AppError> {
    sqlx::query_as::<_, Skill>("SELECT id,name,category,description,instructions,enabled,created_at,updated_at FROM skills WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Skill {id} not found")))
}

pub async fn update_skill(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    category: Option<&str>,
    description: Option<&str>,
    instructions: Option<&str>,
    enabled: Option<bool>,
) -> Result<Skill, AppError> {
    let existing = get_skill(pool, id).await?;
    sqlx::query(
        "UPDATE skills SET name=?, category=?, description=?, instructions=?, enabled=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(name.unwrap_or(&existing.name))
    .bind(category.unwrap_or(&existing.category))
    .bind(description.unwrap_or(&existing.description))
    .bind(instructions.unwrap_or(&existing.instructions))
    .bind(enabled.unwrap_or(existing.enabled))
    .bind(id)
    .execute(pool)
    .await?;
    get_skill(pool, id).await
}

pub async fn delete_skill(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM skills WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_agent_skill_links(pool: &SqlitePool) -> Result<Vec<AgentSkillLink>, AppError> {
    sqlx::query_as::<_, AgentSkillLink>(
        "SELECT id,agent_id,skill_id,proficiency,created_at FROM agent_skill_links ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn link_skill_to_agent(
    pool: &SqlitePool,
    id: &str,
    agent_id: &str,
    skill_id: &str,
    proficiency: &str,
) -> Result<AgentSkillLink, AppError> {
    sqlx::query(
        "INSERT OR REPLACE INTO agent_skill_links (id,agent_id,skill_id,proficiency,created_at) VALUES (?,?,?,?,datetime('now'))",
    )
    .bind(id)
    .bind(agent_id)
    .bind(skill_id)
    .bind(proficiency)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentSkillLink>(
        "SELECT id,agent_id,skill_id,proficiency,created_at FROM agent_skill_links WHERE agent_id=? AND skill_id=? LIMIT 1",
    )
    .bind(agent_id)
    .bind(skill_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn unlink_skill_from_agent(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_skill_links WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_team_skill_links(pool: &SqlitePool) -> Result<Vec<TeamSkillLink>, AppError> {
    sqlx::query_as::<_, TeamSkillLink>(
        "SELECT id,team_id,skill_id,created_at FROM team_skill_links ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn link_skill_to_team(
    pool: &SqlitePool,
    id: &str,
    team_id: &str,
    skill_id: &str,
) -> Result<TeamSkillLink, AppError> {
    sqlx::query(
        "INSERT OR REPLACE INTO team_skill_links (id,team_id,skill_id,created_at) VALUES (?,?,?,datetime('now'))",
    )
    .bind(id)
    .bind(team_id)
    .bind(skill_id)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, TeamSkillLink>(
        "SELECT id,team_id,skill_id,created_at FROM team_skill_links WHERE team_id=? AND skill_id=? LIMIT 1",
    )
    .bind(team_id)
    .bind(skill_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn unlink_skill_from_team(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM team_skill_links WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

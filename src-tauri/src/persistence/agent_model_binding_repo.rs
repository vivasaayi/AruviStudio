use crate::domain::agent::AgentModelBinding;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn get_agent_model_bindings(
    pool: &SqlitePool,
    agent_id: &str,
) -> Result<Vec<AgentModelBinding>, AppError> {
    sqlx::query_as::<_, AgentModelBinding>(
        "SELECT id,agent_id,model_id,priority,created_at FROM agent_model_bindings WHERE agent_id=? ORDER BY priority ASC",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn list_agent_model_bindings(
    pool: &SqlitePool,
) -> Result<Vec<AgentModelBinding>, AppError> {
    sqlx::query_as::<_, AgentModelBinding>(
        "SELECT id,agent_id,model_id,priority,created_at FROM agent_model_bindings ORDER BY agent_id ASC, priority ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn delete_agent_model_bindings_for_agent(
    pool: &SqlitePool,
    agent_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_model_bindings WHERE agent_id=?")
        .bind(agent_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_agent_model_binding(
    pool: &SqlitePool,
    id: &str,
    agent_id: &str,
    model_id: &str,
    priority: i32,
) -> Result<AgentModelBinding, AppError> {
    sqlx::query(
        "INSERT INTO agent_model_bindings (id,agent_id,model_id,priority) VALUES (?,?,?,?)",
    )
    .bind(id)
    .bind(agent_id)
    .bind(model_id)
    .bind(priority)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentModelBinding>(
        "SELECT id,agent_id,model_id,priority,created_at FROM agent_model_bindings WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

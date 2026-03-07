use crate::domain::artifact::Artifact;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_artifact(
    pool: &SqlitePool,
    id: &str,
    work_item_id: &str,
    workflow_run_id: Option<&str>,
    agent_run_id: Option<&str>,
    artifact_type: &str,
    summary: &str,
    storage_path: &str,
) -> Result<Artifact, AppError> {
    sqlx::query_as::<_, Artifact>(
        "INSERT INTO artifacts (id,work_item_id,workflow_run_id,agent_run_id,artifact_type,storage_path,summary,created_at) 
         VALUES (?,?,?,?,?,?,?,datetime('now')) 
         RETURNING id,work_item_id,workflow_run_id,agent_run_id,artifact_type,storage_path,summary,content_type,size_bytes,created_at"
    )
    .bind(id)
    .bind(work_item_id)
    .bind(workflow_run_id)
    .bind(agent_run_id)
    .bind(artifact_type)
    .bind(storage_path)
    .bind(summary)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn list_work_item_artifacts(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Vec<Artifact>, AppError> {
    sqlx::query_as::<_, Artifact>("SELECT id,work_item_id,workflow_run_id,agent_run_id,artifact_type,storage_path,summary,content_type,size_bytes,created_at FROM artifacts WHERE work_item_id=? ORDER BY created_at DESC")
        .bind(work_item_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn get_artifact(pool: &SqlitePool, artifact_id: &str) -> Result<Artifact, AppError> {
    sqlx::query_as::<_, Artifact>(
        "SELECT id,work_item_id,workflow_run_id,agent_run_id,artifact_type,storage_path,summary,content_type,size_bytes,created_at
         FROM artifacts
         WHERE id=?",
    )
    .bind(artifact_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

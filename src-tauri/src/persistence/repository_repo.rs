use crate::domain::repository::Repository;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_repository(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    local_path: &str,
    remote_url: &str,
    default_branch: &str,
) -> Result<Repository, AppError> {
    sqlx::query_as::<_, Repository>("INSERT INTO repositories (id,name,local_path,remote_url,default_branch) VALUES (?,?,?,?,?) RETURNING id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at")
        .bind(id).bind(name).bind(local_path).bind(remote_url).bind(default_branch)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_repositories(pool: &SqlitePool) -> Result<Vec<Repository>, AppError> {
    sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories ORDER BY name")
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn delete_repository(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM repositories WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_repository(pool: &SqlitePool, id: &str) -> Result<Repository, AppError> {
    sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories WHERE id=?")
        .bind(id)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn attach_repository(
    pool: &SqlitePool,
    id: &str,
    scope_type: &str,
    scope_id: &str,
    repository_id: &str,
    is_default: bool,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO repository_attachments (id,scope_type,scope_id,repository_id,is_default) VALUES (?,?,?,?,?)")
        .bind(id).bind(scope_type).bind(scope_id).bind(repository_id).bind(is_default)
        .execute(pool).await?;
    Ok(())
}

pub async fn resolve_repository_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Option<Repository>, AppError> {
    // Priority: work item override -> module attachment -> product attachment
    let work_item = crate::persistence::work_item_repo::get_work_item(pool, work_item_id).await?;
    if let Some(ref repo_id) = work_item.repo_override_id {
        let repo = sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories WHERE id=?")
            .bind(repo_id)
            .fetch_optional(pool).await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }
    if let Some(ref module_id) = work_item.module_id {
        let repo = sqlx::query_as::<_, Repository>(
            "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at FROM repositories r JOIN repository_attachments ra ON r.id=ra.repository_id WHERE ra.scope_type='module' AND ra.scope_id=? AND ra.is_default=1 LIMIT 1")
            .bind(module_id)
            .fetch_optional(pool).await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }
    let product_id = &work_item.product_id;
    let repo = sqlx::query_as::<_, Repository>(
        "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at FROM repositories r JOIN repository_attachments ra ON r.id=ra.repository_id WHERE ra.scope_type='product' AND ra.scope_id=? AND ra.is_default=1 LIMIT 1")
        .bind(product_id)
        .fetch_optional(pool).await?;
    Ok(repo)
}

pub async fn resolve_repository_for_scope(
    pool: &SqlitePool,
    product_id: Option<&str>,
    module_id: Option<&str>,
) -> Result<Option<Repository>, AppError> {
    if let Some(module_id) = module_id {
        let repo = sqlx::query_as::<_, Repository>(
            "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at
             FROM repositories r
             JOIN repository_attachments ra ON r.id=ra.repository_id
             WHERE ra.scope_type='module' AND ra.scope_id=? AND ra.is_default=1
             LIMIT 1",
        )
        .bind(module_id)
        .fetch_optional(pool)
        .await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }

    if let Some(product_id) = product_id {
        let repo = sqlx::query_as::<_, Repository>(
            "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at
             FROM repositories r
             JOIN repository_attachments ra ON r.id=ra.repository_id
             WHERE ra.scope_type='product' AND ra.scope_id=? AND ra.is_default=1
             LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }

    Ok(None)
}

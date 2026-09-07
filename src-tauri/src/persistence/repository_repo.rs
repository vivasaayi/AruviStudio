use crate::domain::repository::{Repository, RepositoryAttachment};
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

pub async fn find_repository_by_local_path(
    pool: &SqlitePool,
    local_path: &str,
) -> Result<Option<Repository>, AppError> {
    sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories WHERE local_path=? ORDER BY created_at,id LIMIT 1")
        .bind(local_path)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn update_repository(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    local_path: &str,
    remote_url: &str,
    default_branch: &str,
) -> Result<Repository, AppError> {
    sqlx::query_as::<_, Repository>(
        "UPDATE repositories
         SET name=?, local_path=?, remote_url=?, default_branch=?, updated_at=datetime('now')
         WHERE id=?
         RETURNING id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at",
    )
    .bind(name)
    .bind(local_path)
    .bind(remote_url)
    .bind(default_branch)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
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
    let mut transaction = pool.begin().await?;
    if is_default {
        sqlx::query(
            "UPDATE repository_attachments SET is_default=0 WHERE scope_type=? AND scope_id=?",
        )
        .bind(scope_type)
        .bind(scope_id)
        .execute(&mut *transaction)
        .await?;
    }
    let has_default: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repository_attachments WHERE scope_type=? AND scope_id=? AND is_default=1)")
        .bind(scope_type)
        .bind(scope_id)
        .fetch_one(&mut *transaction)
        .await?;
    let effective_default = is_default || !has_default;
    sqlx::query("INSERT INTO repository_attachments (id,scope_type,scope_id,repository_id,is_default) VALUES (?,?,?,?,?) ON CONFLICT(scope_type,scope_id,repository_id) DO UPDATE SET is_default=CASE WHEN excluded.is_default=1 THEN 1 ELSE repository_attachments.is_default END")
        .bind(id).bind(scope_type).bind(scope_id).bind(repository_id).bind(effective_default)
        .execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn list_repository_attachments(
    pool: &SqlitePool,
    scope_type: &str,
    scope_id: &str,
) -> Result<Vec<RepositoryAttachment>, AppError> {
    sqlx::query_as::<_, RepositoryAttachment>("SELECT id,scope_type,scope_id,repository_id,is_default,created_at FROM repository_attachments WHERE scope_type=? AND scope_id=? ORDER BY is_default DESC,created_at,id")
        .bind(scope_type)
        .bind(scope_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

pub async fn list_all_repository_attachments(
    pool: &SqlitePool,
) -> Result<Vec<RepositoryAttachment>, AppError> {
    sqlx::query_as::<_, RepositoryAttachment>("SELECT id,scope_type,scope_id,repository_id,is_default,created_at FROM repository_attachments ORDER BY scope_type,scope_id,is_default DESC,created_at,id")
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

pub async fn detach_repository(
    pool: &SqlitePool,
    scope_type: &str,
    scope_id: &str,
    repository_id: &str,
) -> Result<(), AppError> {
    let mut transaction = pool.begin().await?;
    let was_default: bool = sqlx::query_scalar("SELECT COALESCE(MAX(is_default),0) FROM repository_attachments WHERE scope_type=? AND scope_id=? AND repository_id=?")
        .bind(scope_type).bind(scope_id).bind(repository_id)
        .fetch_one(&mut *transaction).await?;
    sqlx::query(
        "DELETE FROM repository_attachments WHERE scope_type=? AND scope_id=? AND repository_id=?",
    )
    .bind(scope_type)
    .bind(scope_id)
    .bind(repository_id)
    .execute(&mut *transaction)
    .await?;
    if was_default {
        sqlx::query("UPDATE repository_attachments SET is_default=1 WHERE id=(SELECT id FROM repository_attachments WHERE scope_type=? AND scope_id=? ORDER BY created_at,id LIMIT 1)")
            .bind(scope_type).bind(scope_id)
            .execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn resolve_repository_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Option<Repository>, AppError> {
    // Priority: explicit work item override -> active work item repo -> product_area attachment -> product attachment
    let work_item = crate::persistence::work_item_repo::get_work_item(pool, work_item_id).await?;
    if let Some(ref repo_id) = work_item.repo_override_id {
        let repo = sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories WHERE id=?")
            .bind(repo_id)
            .fetch_optional(pool).await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }
    if let Some(ref repo_id) = work_item.active_repo_id {
        let repo = sqlx::query_as::<_, Repository>("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories WHERE id=?")
            .bind(repo_id)
            .fetch_optional(pool).await?;
        if repo.is_some() {
            return Ok(repo);
        }
    }
    if let Some(ref product_area_id) = work_item.product_area_id {
        let repo = sqlx::query_as::<_, Repository>(
            "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at FROM repositories r JOIN repository_attachments ra ON r.id=ra.repository_id WHERE ra.scope_type='product_area' AND ra.scope_id=? AND ra.is_default=1 LIMIT 1")
            .bind(product_area_id)
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
    product_area_id: Option<&str>,
) -> Result<Option<Repository>, AppError> {
    if let Some(product_area_id) = product_area_id {
        let repo = sqlx::query_as::<_, Repository>(
            "SELECT r.id,r.name,r.local_path,r.remote_url,r.default_branch,r.auth_profile,r.created_at,r.updated_at
             FROM repositories r
             JOIN repository_attachments ra ON r.id=ra.repository_id
             WHERE ra.scope_type='product_area' AND ra.scope_id=? AND ra.is_default=1
             LIMIT 1",
        )
        .bind(product_area_id)
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

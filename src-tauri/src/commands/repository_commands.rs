use crate::domain::repository::{Repository, RepositoryTreeNode};
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::services::repo_service;
use crate::state::AppState;
use std::process::Command;
use tauri::State;

pub(crate) mod exports;
pub mod workspace;
pub(crate) use workspace::create_local_workspace_for_scope;

#[tauri::command]
pub async fn register_repository(
    state: State<'_, AppState>,
    name: String,
    local_path: String,
    remote_url: String,
    default_branch: String,
) -> Result<Repository, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    repository_repo::create_repository(
        &state.db,
        &id,
        &name,
        &local_path,
        &remote_url,
        &default_branch,
    )
    .await
}

#[tauri::command]
pub async fn update_repository(
    state: State<'_, AppState>,
    id: String,
    name: String,
    local_path: String,
    remote_url: String,
    default_branch: String,
) -> Result<Repository, AppError> {
    let trimmed_name = name.trim();
    let trimmed_path = local_path.trim();
    let trimmed_branch = default_branch.trim();

    if trimmed_name.is_empty() {
        return Err(AppError::Validation(
            "Workspace name is required".to_string(),
        ));
    }
    if trimmed_path.is_empty() {
        return Err(AppError::Validation("Local path is required".to_string()));
    }
    if trimmed_branch.is_empty() {
        return Err(AppError::Validation(
            "Default branch is required".to_string(),
        ));
    }

    let existing = repository_repo::get_repository(&state.db, &id).await?;
    let repository = repository_repo::update_repository(
        &state.db,
        &id,
        trimmed_name,
        trimmed_path,
        remote_url.trim(),
        trimmed_branch,
    )
    .await?;

    if existing.default_branch != repository.default_branch {
        sqlx::query(
            "UPDATE work_items
             SET branch_name=?, updated_at=datetime('now')
             WHERE active_repo_id=?
               AND (branch_name IS NULL OR branch_name=?)",
        )
        .bind(&repository.default_branch)
        .bind(&repository.id)
        .bind(&existing.default_branch)
        .execute(&state.db)
        .await?;
    }

    Ok(repository)
}

#[tauri::command]
pub async fn list_repositories(state: State<'_, AppState>) -> Result<Vec<Repository>, AppError> {
    repository_repo::list_repositories(&state.db).await
}

#[tauri::command]
pub async fn delete_repository(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    repository_repo::delete_repository(&state.db, &id).await
}

#[tauri::command]
pub async fn attach_repository(
    state: State<'_, AppState>,
    scope_type: String,
    scope_id: String,
    repository_id: String,
    is_default: bool,
) -> Result<(), AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    repository_repo::attach_repository(
        &state.db,
        &id,
        &scope_type,
        &scope_id,
        &repository_id,
        is_default,
    )
    .await
}

#[tauri::command]
pub async fn resolve_repository_for_work_item(
    state: State<'_, AppState>,
    work_item_id: String,
) -> Result<Option<Repository>, AppError> {
    repository_repo::resolve_repository_for_work_item(&state.db, &work_item_id).await
}

#[tauri::command]
pub async fn resolve_repository_for_scope(
    state: State<'_, AppState>,
    product_id: Option<String>,
    product_area_id: Option<String>,
) -> Result<Option<Repository>, AppError> {
    repository_repo::resolve_repository_for_scope(
        &state.db,
        product_id.as_deref(),
        product_area_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn browse_for_repository_path() -> Result<Option<String>, AppError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"POSIX path of (choose folder with prompt "Select repository folder")"#)
        .output()
        .map_err(|error| AppError::Validation(format!("Failed to open folder picker: {error}")))?;

    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), AppError> {
    let status = Command::new("open")
        .arg("-R")
        .arg(&path)
        .status()
        .map_err(|error| {
            AppError::Validation(format!("Failed to reveal path in Finder: {error}"))
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Finder could not reveal the requested path".to_string(),
        ))
    }
}

#[tauri::command]
pub async fn list_repository_tree(
    state: State<'_, AppState>,
    repository_id: String,
    include_hidden: Option<bool>,
    max_depth: Option<i64>,
) -> Result<Vec<RepositoryTreeNode>, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    let depth = max_depth.map(|value| value.clamp(1, 32) as usize);
    repo_service::list_repository_tree(
        &repository.local_path,
        include_hidden.unwrap_or(false),
        depth,
    )
}

#[tauri::command]
pub async fn read_repository_file(
    state: State<'_, AppState>,
    repository_id: String,
    relative_path: String,
) -> Result<String, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::read_repository_file(&repository.local_path, &relative_path)
}

#[tauri::command]
pub async fn write_repository_file(
    state: State<'_, AppState>,
    repository_id: String,
    relative_path: String,
    content: String,
) -> Result<(), AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::write_repository_file(&repository.local_path, &relative_path, &content)
}

#[tauri::command]
pub async fn get_repository_file_sha256(
    state: State<'_, AppState>,
    repository_id: String,
    relative_path: String,
) -> Result<String, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::get_repository_file_sha256(&repository.local_path, &relative_path)
}

#[tauri::command]
pub async fn apply_repository_patch(
    state: State<'_, AppState>,
    repository_id: String,
    relative_path: String,
    patch: String,
    base_sha256: Option<String>,
) -> Result<String, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::apply_repository_patch(
        &repository.local_path,
        &relative_path,
        &patch,
        base_sha256.as_deref(),
    )
}

#[cfg(test)]
mod tests;

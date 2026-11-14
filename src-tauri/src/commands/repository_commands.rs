use crate::domain::repository::{Repository, RepositoryAttachment, RepositoryTreeNode};
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::services::repo_service;
use crate::state::AppState;
use std::path::{Component, Path, PathBuf};
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
    register_repository_for_state(&state, name, local_path, remote_url, default_branch).await
}

pub(crate) async fn register_repository_for_state(
    state: &AppState,
    name: String,
    local_path: String,
    remote_url: String,
    default_branch: String,
) -> Result<Repository, AppError> {
    let local_path = normalize_repository_path(&local_path)?;
    if let Some(existing) =
        repository_repo::find_repository_by_local_path(&state.db, &local_path).await?
    {
        return Ok(existing);
    }
    for existing in repository_repo::list_repositories(&state.db).await? {
        if normalize_repository_path(&existing.local_path)
            .ok()
            .as_deref()
            == Some(local_path.as_str())
        {
            return Ok(existing);
        }
    }
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

fn normalize_repository_path(raw: &str) -> Result<String, AppError> {
    let path = Path::new(raw.trim());
    if !path.is_absolute() {
        return Err(AppError::Validation(
            "workspace path must be absolute".into(),
        ));
    }
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| {
        path.components()
            .fold(PathBuf::new(), |mut output, component| {
                match component {
                    Component::ParentDir => {
                        output.pop();
                    }
                    Component::CurDir => {}
                    other => output.push(other.as_os_str()),
                }
                output
            })
    });
    normalized
        .into_os_string()
        .into_string()
        .map_err(|_| AppError::Validation("workspace path must be valid UTF-8".into()))
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
    let normalized_path = normalize_repository_path(trimmed_path)?;
    for other in repository_repo::list_repositories(&state.db).await? {
        if other.id != id
            && normalize_repository_path(&other.local_path).ok().as_deref()
                == Some(normalized_path.as_str())
        {
            return Err(AppError::Validation(
                "Another workspace is already registered for this local path".into(),
            ));
        }
    }

    let existing = repository_repo::get_repository(&state.db, &id).await?;
    let repository = repository_repo::update_repository(
        &state.db,
        &id,
        trimmed_name,
        &normalized_path,
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
    validate_attachment_scope(&scope_type, &scope_id)?;
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

fn validate_attachment_scope(scope_type: &str, scope_id: &str) -> Result<(), AppError> {
    if !matches!(scope_type, "product" | "product_area") {
        return Err(AppError::Validation(
            "workspace scope must be product or product_area".into(),
        ));
    }
    if scope_id.trim().is_empty() {
        return Err(AppError::Validation(
            "workspace scope ID is required".into(),
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_repository_attachments(
    state: State<'_, AppState>,
    scope_type: String,
    scope_id: String,
) -> Result<Vec<RepositoryAttachment>, AppError> {
    validate_attachment_scope(&scope_type, &scope_id)?;
    repository_repo::list_repository_attachments(&state.db, &scope_type, &scope_id).await
}

#[tauri::command]
pub async fn list_all_repository_attachments(
    state: State<'_, AppState>,
) -> Result<Vec<RepositoryAttachment>, AppError> {
    repository_repo::list_all_repository_attachments(&state.db).await
}

#[tauri::command]
pub async fn detach_repository(
    state: State<'_, AppState>,
    scope_type: String,
    scope_id: String,
    repository_id: String,
) -> Result<(), AppError> {
    validate_attachment_scope(&scope_type, &scope_id)?;
    repository_repo::detach_repository(&state.db, &scope_type, &scope_id, &repository_id).await
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

#[tauri::command]
pub async fn matching_repository_files(
    state: State<'_, AppState>, repository_id: String, relative_path: String, open_paths: Vec<String>,
) -> Result<Vec<String>, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    crate::services::repo_file_service::matching_repository_files(&repository.local_path, &relative_path, &open_paths)
}

#[tauri::command]
pub async fn preview_repository_patch(
    state: State<'_, AppState>, repository_id: String, relative_path: String, patch: String, base_sha256: Option<String>,
) -> Result<crate::services::repo_file_service::RepositoryPatchPreview, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::preview_repository_patch(&repository.local_path, &relative_path, &patch, base_sha256.as_deref())
}

#[tauri::command]
pub async fn apply_reviewed_repository_patch(
    state: State<'_, AppState>, repository_id: String, relative_path: String, patch: String, base_sha256: String, expected_exists: bool,
) -> Result<String, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::apply_reviewed_repository_patch(&repository.local_path, &relative_path, &patch, &base_sha256, expected_exists)
}

#[cfg(test)]
mod tests;

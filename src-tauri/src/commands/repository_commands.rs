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
    let mut transaction = state.db.begin_with("BEGIN IMMEDIATE").await?;
    let all: Vec<Repository> = sqlx::query_as("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories")
        .fetch_all(&mut *transaction).await?;
    if let Some(existing) = all.into_iter().find(|entry| normalize_repository_path(&entry.local_path).ok().as_deref() == Some(local_path.as_str())) {
        transaction.commit().await?;
        return Ok(existing);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let repository = sqlx::query_as::<_, Repository>("INSERT INTO repositories (id,name,local_path,remote_url,default_branch) VALUES (?,?,?,?,?) RETURNING id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at")
        .bind(id).bind(&name).bind(&local_path).bind(&remote_url).bind(&default_branch)
        .fetch_one(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(repository)
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
    update_repository_for_state(&state, id, name, local_path, remote_url, default_branch, None)
        .await?
        .ok_or_else(|| AppError::Validation("Workspace changed or was removed".into()))
}

/// Update the workspace and inherited work-item branches under one write lock.
pub(crate) async fn update_repository_for_state(
    state: &AppState,
    id: String,
    name: String,
    local_path: String,
    remote_url: String,
    default_branch: String,
    expected: Option<Repository>,
) -> Result<Option<Repository>, AppError> {
    let trimmed_name = name.trim();
    let trimmed_path = local_path.trim();
    let trimmed_branch = default_branch.trim();
    if trimmed_name.is_empty() { return Err(AppError::Validation("Workspace name is required".into())); }
    if trimmed_path.is_empty() { return Err(AppError::Validation("Local path is required".into())); }
    if trimmed_branch.is_empty() { return Err(AppError::Validation("Default branch is required".into())); }
    let normalized_path = normalize_repository_path(trimmed_path)?;
    let mut transaction = state.db.begin_with("BEGIN IMMEDIATE").await?;
    let all: Vec<Repository> = sqlx::query_as("SELECT id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at FROM repositories")
        .fetch_all(&mut *transaction).await?;
    let Some(existing) = all.iter().find(|row| row.id == id) else { return Ok(None); };
    if expected.as_ref().is_some_and(|snapshot| snapshot.id != existing.id || snapshot.name != existing.name || snapshot.local_path != existing.local_path || snapshot.remote_url != existing.remote_url || snapshot.default_branch != existing.default_branch) {
        return Ok(None);
    }
    if all.iter().any(|other| other.id != id && normalize_repository_path(&other.local_path).ok().as_deref() == Some(normalized_path.as_str())) {
        return Err(AppError::Validation("Another workspace is already registered for this local path".into()));
    }
    if existing.local_path != normalized_path || existing.default_branch != trimmed_branch || existing.remote_url != remote_url.trim() {
        let active: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work_items w WHERE (w.active_repo_id=? OR w.repo_override_id=? OR EXISTS (SELECT 1 FROM repository_attachments a WHERE a.repository_id=? AND a.is_default=1 AND ((a.scope_type='product' AND a.scope_id=w.product_id) OR (a.scope_type='product_area' AND a.scope_id=w.product_area_id)))) AND (EXISTS (SELECT 1 FROM workflow_runs r WHERE r.work_item_id=w.id AND r.status IN ('running','paused') AND r.current_stage NOT IN ('done','failed','cancelled')) OR EXISTS (SELECT 1 FROM external_cli_runs c WHERE c.work_item_id=w.id AND c.status='running')))")
            .bind(&id).bind(&id).bind(&id).fetch_one(&mut *transaction).await?;
        if active != 0 { return Err(AppError::Validation("Workspace is used by an active workflow or CLI run".into())); }
    }
    let repository: Repository = sqlx::query_as("UPDATE repositories SET name=?,local_path=?,remote_url=?,default_branch=?,updated_at=datetime('now') WHERE id=? RETURNING id,name,local_path,remote_url,default_branch,auth_profile,created_at,updated_at")
        .bind(trimmed_name).bind(&normalized_path).bind(remote_url.trim()).bind(trimmed_branch).bind(&id)
        .fetch_one(&mut *transaction).await?;
    if existing.default_branch != repository.default_branch {
        sqlx::query("UPDATE work_items SET branch_name=?,updated_at=datetime('now') WHERE active_repo_id=? AND (branch_name IS NULL OR branch_name=?)")
            .bind(&repository.default_branch).bind(&repository.id).bind(&existing.default_branch)
            .execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(Some(repository))
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

// Hold the SQLite write reservation while checking the registered root and
// performing filesystem I/O, so cooperating Studio processes cannot redirect
// the workspace during a guarded operation.
pub(crate) async fn lock_repository_root<'a>(state: &'a AppState, repository_id: &str, expected_root: Option<&str>) -> Result<(sqlx::Transaction<'a, sqlx::Sqlite>, String), AppError> {
    let mut transaction = state.db.begin_with("BEGIN IMMEDIATE").await?;
    let root: Option<String> = sqlx::query_scalar("SELECT local_path FROM repositories WHERE id=?")
        .bind(repository_id).fetch_optional(&mut *transaction).await?;
    let root = root.ok_or_else(|| AppError::NotFound("Workspace not found".into()))?;
    if expected_root.is_some_and(|expected| expected != root) {
        return Err(AppError::Validation("Workspace path changed. Reopen the workspace before writing".into()));
    }
    Ok((transaction, root))
}

#[derive(serde::Serialize)]
pub struct RepositoryFileSnapshot { pub content: String, pub workspace_root: String }

#[tauri::command]
pub async fn get_repository_root_snapshot(state: State<'_, AppState>, repository_id: String) -> Result<String, AppError> {
    let (transaction, root) = lock_repository_root(&state, &repository_id, None).await?;
    transaction.commit().await?;
    Ok(root)
}

#[tauri::command]
pub async fn read_repository_file_snapshot(state: State<'_, AppState>, repository_id: String, relative_path: String, expected_root: Option<String>) -> Result<RepositoryFileSnapshot, AppError> {
    let (transaction, root) = lock_repository_root(&state, &repository_id, expected_root.as_deref()).await?;
    let content = repo_service::read_repository_file(&root, &relative_path)?;
    transaction.commit().await?;
    Ok(RepositoryFileSnapshot { content, workspace_root: root })
}

#[tauri::command]
pub async fn write_repository_file_guarded(state: State<'_, AppState>, repository_id: String, relative_path: String, expected_root: String, expected_content: String, content: String) -> Result<(), AppError> {
    let (transaction, root) = lock_repository_root(&state, &repository_id, Some(&expected_root)).await?;
    let current = repo_service::read_repository_file(&root, &relative_path)?;
    if current != expected_content {
        return Err(AppError::Validation("File changed since it was opened. Copy or discard your edits, then reopen the file.".into()));
    }
    repo_service::write_repository_file(&root, &relative_path, &content)?;
    transaction.commit().await?;
    Ok(())
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
    state: State<'_, AppState>, repository_id: String, relative_path: String, open_paths: Vec<String>, open_repository_id: Option<String>, expected_root: Option<String>, open_expected_root: Option<String>,
) -> Result<Vec<String>, AppError> {
    let (mut transaction, root) = lock_repository_root(&state, &repository_id, expected_root.as_deref()).await?;
    let open_root = match open_repository_id {
        Some(id) if id != repository_id => sqlx::query_scalar::<_, String>("SELECT local_path FROM repositories WHERE id=?")
            .bind(id).fetch_optional(&mut *transaction).await?.ok_or_else(|| AppError::NotFound("Workspace not found".into()))?,
        _ => root.clone(),
    };
    if open_expected_root.as_deref().is_some_and(|expected| expected != open_root) {
        return Err(AppError::Validation("An open file's workspace path changed. Reopen the workspace before applying a patch".into()));
    }
    let matches = crate::services::repo_file_service::matching_repository_files(&root, &relative_path, &open_root, &open_paths)?;
    transaction.commit().await?;
    Ok(matches)
}

#[tauri::command]
pub async fn preview_repository_patch(
    state: State<'_, AppState>, repository_id: String, relative_path: String, patch: String, base_sha256: Option<String>,
) -> Result<crate::services::repo_file_service::RepositoryPatchPreview, AppError> {
    let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
    repo_service::preview_repository_patch(&repository.local_path, &relative_path, &patch, base_sha256.as_deref())
}

#[derive(serde::Serialize)]
pub struct RepositoryPatchSnapshot {
    #[serde(flatten)]
    pub preview: crate::services::repo_file_service::RepositoryPatchPreview,
    pub workspace_root: String,
}

#[tauri::command]
pub async fn preview_repository_patch_snapshot(
    state: State<'_, AppState>, repository_id: String, relative_path: String, patch: String, base_sha256: Option<String>, expected_root: Option<String>,
) -> Result<RepositoryPatchSnapshot, AppError> {
    let (transaction, root) = lock_repository_root(&state, &repository_id, expected_root.as_deref()).await?;
    let preview = repo_service::preview_repository_patch(&root, &relative_path, &patch, base_sha256.as_deref())?;
    transaction.commit().await?;
    Ok(RepositoryPatchSnapshot { preview, workspace_root: root })
}

#[tauri::command]
pub async fn apply_reviewed_repository_patch(
    state: State<'_, AppState>, repository_id: String, relative_path: String, patch: String, base_sha256: String, expected_exists: bool, expected_root: Option<String>,
) -> Result<String, AppError> {
    let (transaction, root) = lock_repository_root(&state, &repository_id, expected_root.as_deref()).await?;
    let result = repo_service::apply_reviewed_repository_patch(&root, &relative_path, &patch, &base_sha256, expected_exists)?;
    transaction.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests;

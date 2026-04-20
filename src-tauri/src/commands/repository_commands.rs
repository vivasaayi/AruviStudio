use crate::domain::repository::{Repository, RepositoryTreeNode};
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::services::repo_service;
use crate::state::AppState;
use directories::UserDirs;
use git2::{Repository as GitRepository, Signature};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::State;
use tracing::info;

#[derive(serde::Serialize)]
pub struct WorkspaceProvisionResult {
    pub repository: Repository,
    pub created_path: String,
    pub attached_scope_type: String,
    pub attached_scope_id: String,
}

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
    module_id: Option<String>,
) -> Result<Option<Repository>, AppError> {
    repository_repo::resolve_repository_for_scope(
        &state.db,
        product_id.as_deref(),
        module_id.as_deref(),
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
pub async fn export_product_overview_html(
    file_name: String,
    html: String,
) -> Result<String, AppError> {
    let user_dirs = UserDirs::new().ok_or_else(|| {
        AppError::Validation("Could not determine a writable user documents directory".to_string())
    })?;

    let documents_dir = user_dirs.document_dir().ok_or_else(|| {
        AppError::Validation("Could not determine a writable user documents directory".to_string())
    })?;

    let export_dir = documents_dir.join("AruviStudio").join("exports");
    fs::create_dir_all(&export_dir)?;

    let safe_name = sanitize_export_file_name(&file_name);
    let destination = export_dir.join(safe_name);
    fs::write(&destination, html)?;

    Ok(destination.to_string_lossy().to_string())
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
#[allow(non_snake_case)]
pub async fn create_local_workspace(
    state: State<'_, AppState>,
    product_id: Option<String>,
    productId: Option<String>,
    module_id: Option<String>,
    moduleId: Option<String>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    preferred_path: Option<String>,
    preferredPath: Option<String>,
) -> Result<WorkspaceProvisionResult, AppError> {
    create_local_workspace_for_scope(
        state.inner(),
        product_id.or(productId),
        module_id.or(moduleId),
        work_item_id.or(workItemId),
        preferred_path.or(preferredPath),
    )
    .await
}

pub(crate) async fn create_local_workspace_for_scope(
    state: &AppState,
    product_id: Option<String>,
    module_id: Option<String>,
    work_item_id: Option<String>,
    preferred_path: Option<String>,
) -> Result<WorkspaceProvisionResult, AppError> {
    let scope: (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    ) = if let Some(work_item_id) = work_item_id.as_deref() {
        let work_item =
            crate::persistence::work_item_repo::get_work_item(&state.db, work_item_id).await?;
        let product_id = work_item.product_id.clone().ok_or_else(|| {
            AppError::Validation("Selected work item has no product scope".to_string())
        })?;
        let product = crate::persistence::product_repo::get_product(&state.db, &product_id).await?;
        let module_name = if let Some(module_id) = work_item.module_id.as_deref() {
            Some(resolve_module_name(&state.db, module_id).await?)
        } else {
            None
        };
        (
            product_id,
            work_item.module_id.clone(),
            product.name,
            module_name,
            Some(work_item.id),
        )
    } else {
        let product_id =
            product_id.ok_or_else(|| AppError::Validation("missing product id".to_string()))?;
        let product = crate::persistence::product_repo::get_product(&state.db, &product_id).await?;
        let module_name = if let Some(module_id) = module_id.as_deref() {
            Some(resolve_module_name(&state.db, module_id).await?)
        } else {
            None
        };
        (product_id, module_id, product.name, module_name, None)
    };

    let (product_id, module_id, product_name, module_name, maybe_work_item_id) = scope;

    let workspace_root: PathBuf = preferred_path
        .map(PathBuf::from)
        .unwrap_or_else(default_workspace_root);
    let folder_name = module_name
        .as_ref()
        .map(|module| format!("{}-{}", slugify(&product_name), slugify(module)))
        .unwrap_or_else(|| slugify(&product_name));
    let workspace_path = workspace_root.join(folder_name);
    std::fs::create_dir_all(&workspace_path)?;

    let readme = format!("# {}\n\nWorkspace prepared by AruviStudio.\n", product_name);
    let gitignore = "node_modules/\ndist/\nbuild/\ncoverage/\n.env\n.DS_Store\n";
    std::fs::write(workspace_path.join("README.md"), readme)?;
    std::fs::write(workspace_path.join(".gitignore"), gitignore)?;
    std::fs::create_dir_all(workspace_path.join("tests"))?;
    std::fs::write(workspace_path.join("tests/.gitkeep"), "")?;
    std::fs::create_dir_all(workspace_path.join("src"))?;
    std::fs::write(workspace_path.join("src/.gitkeep"), "")?;

    ensure_git_repository(&workspace_path, &product_name)?;

    let local_path = workspace_path.to_string_lossy().to_string();
    let default_branch = "main".to_string();
    let existing_repo = repository_repo::list_repositories(&state.db)
        .await?
        .into_iter()
        .find(|repo| normalize_path(&repo.local_path) == normalize_path(&local_path));

    let repository = if let Some(existing_repo) = existing_repo {
        existing_repo
    } else {
        let repo_id = uuid::Uuid::new_v4().to_string();
        repository_repo::create_repository(
            &state.db,
            &repo_id,
            &product_name,
            &local_path,
            "",
            &default_branch,
        )
        .await?
    };

    let (attached_scope_type, attached_scope_id) = if let Some(module_id) = module_id.as_deref() {
        ("module".to_string(), module_id.to_string())
    } else {
        ("product".to_string(), product_id.clone())
    };

    sqlx::query(
        "DELETE FROM repository_attachments WHERE scope_type = ? AND scope_id = ? AND is_default = 1",
    )
    .bind(&attached_scope_type)
    .bind(&attached_scope_id)
    .execute(&state.db)
    .await?;

    let attachment_id = uuid::Uuid::new_v4().to_string();
    repository_repo::attach_repository(
        &state.db,
        &attachment_id,
        &attached_scope_type,
        &attached_scope_id,
        &repository.id,
        true,
    )
    .await?;

    if let Some(work_item_id) = maybe_work_item_id.as_deref() {
        sqlx::query("UPDATE work_items SET active_repo_id=?, branch_name='main', updated_at=datetime('now') WHERE id=?")
            .bind(&repository.id)
            .bind(work_item_id)
            .execute(&state.db)
            .await?;
    }

    info!(
        repository_id = %repository.id,
        local_path = %local_path,
        scope_type = %attached_scope_type,
        scope_id = %attached_scope_id,
        "create_local_workspace succeeded"
    );

    Ok(WorkspaceProvisionResult {
        repository,
        created_path: local_path,
        attached_scope_type,
        attached_scope_id,
    })
}

fn sanitize_export_file_name(file_name: &str) -> String {
    let mut sanitized: String = file_name
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => character,
            _ => '-',
        })
        .collect();

    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }

    sanitized = sanitized.trim_matches('-').to_string();

    if sanitized.is_empty() {
        "product-overview.html".to_string()
    } else if sanitized.ends_with(".html") {
        sanitized
    } else {
        format!("{sanitized}.html")
    }
}

fn default_workspace_root() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("AruviStudioWorkspaces")
}

async fn resolve_module_name(db: &sqlx::SqlitePool, module_id: &str) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>("SELECT name FROM modules WHERE id = ?")
        .bind(module_id)
        .fetch_one(db)
        .await
        .map_err(|error| error.into())
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if normalized == '-' {
            if !last_dash && !out.is_empty() {
                out.push('-');
            }
            last_dash = true;
        } else {
            out.push(normalized);
            last_dash = false;
        }
    }
    out.trim_matches('-').to_string()
}

fn normalize_path(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn ensure_git_repository(workspace_path: &PathBuf, product_name: &str) -> Result<(), AppError> {
    let repo = if workspace_path.join(".git").exists() {
        GitRepository::open(workspace_path)?
    } else {
        GitRepository::init(workspace_path)?
    };

    let mut index = repo.index()?;
    index.add_path(std::path::Path::new("README.md"))?;
    index.add_path(std::path::Path::new(".gitignore"))?;
    index.add_path(std::path::Path::new("tests/.gitkeep"))?;
    index.add_path(std::path::Path::new("src/.gitkeep"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    if repo.head().is_err() {
        let signature = Signature::now("AruviStudio", "local@aruvi.studio")
            .or_else(|_| repo.signature())
            .map_err(AppError::Git)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Initialize workspace for {}", product_name),
            &tree,
            &[],
        )?;
    }

    Ok(())
}

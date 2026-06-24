use crate::domain::repository::Repository;
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::state::AppState;
use git2::{Repository as GitRepository, Signature};
use std::path::PathBuf;
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
pub async fn create_local_workspace(
    state: State<'_, AppState>,
    product_id: Option<String>,
    product_area_id: Option<String>,
    work_item_id: Option<String>,
    preferred_path: Option<String>,
) -> Result<WorkspaceProvisionResult, AppError> {
    create_local_workspace_for_scope(
        state.inner(),
        product_id,
        product_area_id,
        work_item_id,
        preferred_path,
    )
    .await
}

pub(crate) async fn create_local_workspace_for_scope(
    state: &AppState,
    product_id: Option<String>,
    product_area_id: Option<String>,
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
        let product_area_name = if let Some(product_area_id) = work_item.product_area_id.as_deref()
        {
            Some(resolve_product_area_name(&state.db, product_area_id).await?)
        } else {
            None
        };
        (
            product_id,
            work_item.product_area_id.clone(),
            product.name,
            product_area_name,
            Some(work_item.id),
        )
    } else {
        let product_id =
            product_id.ok_or_else(|| AppError::Validation("missing product id".to_string()))?;
        let product = crate::persistence::product_repo::get_product(&state.db, &product_id).await?;
        let product_area_name = if let Some(product_area_id) = product_area_id.as_deref() {
            Some(resolve_product_area_name(&state.db, product_area_id).await?)
        } else {
            None
        };
        (
            product_id,
            product_area_id,
            product.name,
            product_area_name,
            None,
        )
    };

    let (product_id, product_area_id, product_name, product_area_name, maybe_work_item_id) = scope;

    let workspace_root: PathBuf = preferred_path
        .map(PathBuf::from)
        .unwrap_or_else(default_workspace_root);
    let folder_name = product_area_name
        .as_ref()
        .map(|product_area| format!("{}-{}", slugify(&product_name), slugify(product_area)))
        .unwrap_or_else(|| slugify(&product_name));
    let workspace_path = workspace_root.join(folder_name);
    std::fs::create_dir_all(&workspace_path)?;

    let readme = format!("# {}\n\nWorkspace prepared by AruviStudio.\n", product_name);
    let gitignore = "node_product_areas/\ndist/\nbuild/\ncoverage/\n.env\n.DS_Store\n";
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

    let (attached_scope_type, attached_scope_id) =
        if let Some(product_area_id) = product_area_id.as_deref() {
            ("product_area".to_string(), product_area_id.to_string())
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

fn default_workspace_root() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("AruviStudioWorkspaces")
}

async fn resolve_product_area_name(
    db: &sqlx::SqlitePool,
    product_area_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>("SELECT name FROM product_areas WHERE id = ?")
        .bind(product_area_id)
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

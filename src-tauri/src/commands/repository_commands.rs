use crate::domain::repository::{Repository, RepositoryTreeNode};
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::services::repo_service;
use crate::state::AppState;
use chrono::Utc;
use directories::UserDirs;
use git2::{Repository as GitRepository, Signature};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookExportTocItem {
    pub id: String,
    pub title: String,
    pub level: i32,
}

#[derive(Clone, Debug)]
struct EpubNavNode {
    item: BookExportTocItem,
    children: Vec<EpubNavNode>,
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
    let export_dir = export_documents_dir()?;
    let safe_name =
        sanitize_export_file_name_with_extension(&file_name, "product-overview", "html");
    let destination = export_dir.join(safe_name);
    fs::write(&destination, html)?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn export_product_overview_epub(
    file_name: String,
    title: String,
    html: String,
    toc_items: Vec<BookExportTocItem>,
    author: Option<String>,
    language: Option<String>,
) -> Result<String, AppError> {
    let export_dir = export_documents_dir()?;
    let safe_name = sanitize_export_file_name_with_extension(&file_name, "product-book", "epub");
    let destination = export_dir.join(safe_name);
    if destination.exists() {
        fs::remove_file(&destination)?;
    }

    let temp_root = std::env::temp_dir().join(format!(
        "aruvi-epub-export-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&temp_root)?;

    let result = export_epub_archive(
        &temp_root,
        &destination,
        &title,
        &html,
        &toc_items,
        author.as_deref().unwrap_or("Aruvi Studio"),
        language.as_deref().unwrap_or("en"),
    );

    let cleanup_result = fs::remove_dir_all(&temp_root);
    if let Err(error) = cleanup_result {
        info!("Failed to cleanup temporary EPUB export directory: {error}");
    }

    result?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn export_product_overview_pdf(
    file_name: String,
    html: String,
    page_width: String,
    page_height: String,
    margin_top: String,
    margin_right: String,
    margin_bottom: String,
    margin_left: String,
    header_title: String,
    header_right: Option<String>,
) -> Result<String, AppError> {
    let export_dir = export_documents_dir()?;
    let safe_name = sanitize_export_file_name_with_extension(&file_name, "product-book", "pdf");
    let destination = export_dir.join(safe_name);

    let temp_root = std::env::temp_dir().join(format!(
        "aruvi-pdf-export-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&temp_root)?;
    let source_path = temp_root.join("book.html");
    fs::write(&source_path, html)?;

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| AppError::Internal("Could not resolve workspace root".to_string()))?
        .to_path_buf();
    let script_path = workspace_root.join("scripts").join("export-book-pdf.mjs");
    if !script_path.exists() {
        return Err(AppError::Validation(format!(
            "PDF export script is missing at {}",
            script_path.to_string_lossy()
        )));
    }

    let output = Command::new("node")
        .current_dir(&workspace_root)
        .arg(&script_path)
        .arg(&source_path)
        .arg(&destination)
        .arg(page_width)
        .arg(page_height)
        .arg(margin_top)
        .arg(margin_right)
        .arg(margin_bottom)
        .arg(margin_left)
        .arg(header_title)
        .arg(header_right.unwrap_or_else(|| "Aruvi Studio Book".to_string()))
        .output()
        .map_err(|error| {
            AppError::Validation(format!(
                "Failed to launch the PDF export renderer. Ensure Node.js is installed: {error}"
            ))
        })?;

    let cleanup_result = fs::remove_dir_all(&temp_root);
    if let Err(error) = cleanup_result {
        info!("Failed to cleanup temporary PDF export directory: {error}");
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(AppError::Validation(format!(
            "Playwright PDF export failed. Ensure npm dependencies are installed. {}",
            detail
        )));
    }

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

fn export_documents_dir() -> Result<PathBuf, AppError> {
    let user_dirs = UserDirs::new().ok_or_else(|| {
        AppError::Validation("Could not determine a writable user documents directory".to_string())
    })?;

    let documents_dir = user_dirs.document_dir().ok_or_else(|| {
        AppError::Validation("Could not determine a writable user documents directory".to_string())
    })?;

    let export_dir = documents_dir.join("AruviStudio").join("exports");
    fs::create_dir_all(&export_dir)?;
    Ok(export_dir)
}

fn sanitize_export_file_name_with_extension(
    file_name: &str,
    default_stem: &str,
    extension: &str,
) -> String {
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
        format!("{default_stem}.{extension}")
    } else if sanitized.ends_with(&format!(".{extension}")) {
        sanitized
    } else {
        format!("{sanitized}.{extension}")
    }
}

fn export_epub_archive(
    temp_root: &Path,
    destination: &Path,
    title: &str,
    html: &str,
    toc_items: &[BookExportTocItem],
    author: &str,
    language: &str,
) -> Result<(), AppError> {
    let meta_inf_dir = temp_root.join("META-INF");
    let oebps_dir = temp_root.join("OEBPS");
    fs::create_dir_all(&meta_inf_dir)?;
    fs::create_dir_all(&oebps_dir)?;

    fs::write(temp_root.join("mimetype"), "application/epub+zip")?;
    fs::write(
        meta_inf_dir.join("container.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
    )?;

    let now = Utc::now();
    let identifier = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    fs::write(oebps_dir.join("book.xhtml"), html)?;
    fs::write(
        oebps_dir.join("nav.xhtml"),
        build_epub_nav_document(title, toc_items, language),
    )?;
    fs::write(
        oebps_dir.join("content.opf"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">{}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:creator>{}</dc:creator>
    <dc:language>{}</dc:language>
    <meta property="dcterms:modified">{}</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="book" href="book.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="nav"/>
    <itemref idref="book"/>
  </spine>
</package>"#,
            escape_xml(identifier.as_str()),
            escape_xml(title),
            escape_xml(author),
            escape_xml(language),
            now.format("%Y-%m-%dT%H:%M:%SZ")
        ),
    )?;

    let first_status = Command::new("zip")
        .current_dir(temp_root)
        .arg("-X0")
        .arg(destination)
        .arg("mimetype")
        .status()
        .map_err(|error| {
            AppError::Validation(format!(
                "Failed to invoke zip while building EPUB. Ensure the system zip utility is available: {error}"
            ))
        })?;

    if !first_status.success() {
        return Err(AppError::Validation(
            "The zip utility failed while writing the EPUB mimetype entry.".to_string(),
        ));
    }

    let second_status = Command::new("zip")
        .current_dir(temp_root)
        .arg("-Xr9D")
        .arg(destination)
        .arg("META-INF")
        .arg("OEBPS")
        .status()
        .map_err(|error| {
            AppError::Validation(format!(
                "Failed to invoke zip while finalizing the EPUB archive: {error}"
            ))
        })?;

    if !second_status.success() {
        return Err(AppError::Validation(
            "The zip utility failed while finalizing the EPUB archive.".to_string(),
        ));
    }

    Ok(())
}

fn build_epub_nav_document(title: &str, toc_items: &[BookExportTocItem], language: &str) -> String {
    let nav_tree = build_epub_nav_tree(toc_items);
    let items = render_epub_nav_nodes(&nav_tree, 3);

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="{language}">
  <head>
    <title>{title}</title>
    <style>
      body {{
        margin: 0;
        padding: 1.4rem 1.2rem;
        font-family: Georgia, "Times New Roman", serif;
        color: #1f2733;
      }}

      h1 {{
        margin: 0 0 1rem;
        font-size: 1.6rem;
      }}

      nav ol {{
        margin: 0;
        padding-left: 1.2rem;
      }}

      nav li {{
        margin: 0.35rem 0;
      }}

      nav li ol {{
        margin-top: 0.2rem;
      }}

      nav a {{
        color: inherit;
        text-decoration: none;
      }}
    </style>
  </head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>{title}</h1>
      <ol>
{items}
      </ol>
    </nav>
  </body>
</html>"#,
        language = escape_xml(language),
        title = escape_xml(title),
        items = items
    )
}

fn build_epub_nav_tree(items: &[BookExportTocItem]) -> Vec<EpubNavNode> {
    let mut index = 0;
    build_epub_nav_nodes(items, &mut index, 0)
}

fn build_epub_nav_nodes(
    items: &[BookExportTocItem],
    index: &mut usize,
    level: i32,
) -> Vec<EpubNavNode> {
    let mut nodes: Vec<EpubNavNode> = Vec::new();

    while *index < items.len() {
        let item = &items[*index];
        if item.level < level {
            break;
        }

        if item.level > level {
            if let Some(last) = nodes.last_mut() {
                last.children = build_epub_nav_nodes(items, index, item.level);
                continue;
            }
            *index += 1;
            continue;
        }

        let current_level = item.level;
        let mut node = EpubNavNode {
            item: item.clone(),
            children: Vec::new(),
        };
        *index += 1;

        if *index < items.len() && items[*index].level > current_level {
            node.children = build_epub_nav_nodes(items, index, items[*index].level);
        }

        nodes.push(node);
    }

    nodes
}

fn render_epub_nav_nodes(nodes: &[EpubNavNode], depth: usize) -> String {
    nodes.iter()
        .map(|node| {
            let indent = "  ".repeat(depth);
            let children = if node.children.is_empty() {
                String::new()
            } else {
                format!(
                    "\n{indent}  <ol>\n{}\n{indent}  </ol>",
                    render_epub_nav_nodes(&node.children, depth + 2)
                )
            };

            format!(
                r#"{indent}<li><a href="book.xhtml#{id}">{title}</a>{children}</li>"#,
                indent = indent,
                id = escape_xml(&node.item.id),
                title = escape_xml(&node.item.title),
                children = children
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

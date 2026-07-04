use crate::error::AppError;
use directories::UserDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

mod epub;
use epub::export_epub_archive;
pub use epub::BookExportTocItem;

#[derive(Clone, Debug, Deserialize)]
pub struct ExportProductOverviewPdfCommand {
    #[serde(alias = "fileName")]
    pub file_name: String,
    pub html: String,
    #[serde(alias = "pageWidth")]
    pub page_width: String,
    #[serde(alias = "pageHeight")]
    pub page_height: String,
    #[serde(alias = "marginTop")]
    pub margin_top: String,
    #[serde(alias = "marginRight")]
    pub margin_right: String,
    #[serde(alias = "marginBottom")]
    pub margin_bottom: String,
    #[serde(alias = "marginLeft")]
    pub margin_left: String,
    #[serde(alias = "headerTitle")]
    pub header_title: String,
    #[serde(alias = "headerRight")]
    pub header_right: Option<String>,
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

    let temp_root =
        std::env::temp_dir().join(format!("aruvi-epub-export-{}", uuid::Uuid::new_v4()));
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
pub async fn export_product_overview_pdf(
    request: ExportProductOverviewPdfCommand,
) -> Result<String, AppError> {
    let export_dir = export_documents_dir()?;
    let safe_name =
        sanitize_export_file_name_with_extension(&request.file_name, "product-book", "pdf");
    let destination = export_dir.join(safe_name);

    let temp_root = std::env::temp_dir().join(format!("aruvi-pdf-export-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_root)?;
    let source_path = temp_root.join("book.html");
    fs::write(&source_path, request.html)?;

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
        .arg(request.page_width)
        .arg(request.page_height)
        .arg(request.margin_top)
        .arg(request.margin_right)
        .arg(request.margin_bottom)
        .arg(request.margin_left)
        .arg(request.header_title)
        .arg(
            request
                .header_right
                .unwrap_or_else(|| "Aruvi Studio Book".to_string()),
        )
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

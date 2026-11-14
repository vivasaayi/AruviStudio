use crate::error::AppError;
use directories::UserDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::services::product_book_epub::render_epub_archive_with_images;
pub use crate::services::product_book_epub::BookExportTocItem;

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
    let bytes = tokio::task::spawn_blocking(move || {
        let (html, images) = crate::services::product_book_images::prepare_book_images(&html, crate::services::product_book_images::ImageTarget::Epub)?;
        render_epub_archive_with_images(&title, &html, &toc_items, author.as_deref().unwrap_or("Aruvi Studio"), language.as_deref().unwrap_or("en"), &images)
    }).await.map_err(|error| AppError::Internal(format!("EPUB export task failed: {error}")))??;
    fs::write(&destination, bytes)?;

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

    let bytes = render_desktop_pdf(request).await?;
    fs::write(&destination, bytes)?;

    Ok(destination.to_string_lossy().to_string())
}

async fn render_desktop_pdf(request: ExportProductOverviewPdfCommand) -> Result<Vec<u8>, AppError> {
    let trim = serde_json::json!({"pageWidth":request.page_width,"pageHeight":request.page_height,
        "marginTop":request.margin_top,"marginRight":request.margin_right,"marginBottom":request.margin_bottom,
        "marginLeft":request.margin_left,"label":request.header_right.unwrap_or_else(|| "Aruvi Studio Book".into())});
    let html = request.html;
    let (html, _) = tokio::task::spawn_blocking(move || crate::services::product_book_images::prepare_book_images(&html, crate::services::product_book_images::ImageTarget::Pdf)).await
        .map_err(|error| AppError::Internal(format!("PDF image task failed: {error}")))??;
    crate::services::product_book_pdf::render_pdf(&html, &request.header_title, &trim).await
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf_request(html: String) -> ExportProductOverviewPdfCommand {
        ExportProductOverviewPdfCommand { file_name: "fixture.pdf".into(), html,
            page_width: "6in".into(), page_height: "9in".into(), margin_top: "0.75in".into(),
            margin_right: "0.72in".into(), margin_bottom: "0.85in".into(), margin_left: "0.82in".into(),
            header_title: "Fixture".into(), header_right: Some("Trade".into()) }
    }

    #[tokio::test]
    async fn desktop_pdf_rejects_remote_images_before_rendering() {
        let request = pdf_request("<html><head></head><body><img src=\"https://example.invalid/image.png\"></body></html>".into());
        assert!(render_desktop_pdf(request).await.is_err());
    }

    #[tokio::test]
    #[ignore = "requires configured Chromium and Playwright"]
    async fn desktop_pdf_uses_guarded_renderer() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/private", listener.local_addr().unwrap());
        let html = format!("<html><head><style>@import url('{url}');</style></head><body><script>fetch('{url}')</script><p>Safe fixture</p></body></html>");
        let bytes = render_desktop_pdf(pdf_request(html)).await.unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        if let Ok(path) = std::env::var("ARUVI_DESKTOP_PDF_TEST_OUTPUT") { std::fs::write(path, &bytes).unwrap(); }
        assert!(tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept()).await.is_err());
    }
}

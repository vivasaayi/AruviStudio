use crate::error::AppError;
use crate::services::product_book_epub::{render_epub_archive_with_images, BookExportTocItem};
use crate::services::product_book_images::{prepare_book_images, ImageTarget};
use crate::services::product_book_renderer::{render_product_book_for_product, ProductBookFormat, ProductBookRenderMode, ProductBookTrim};
use sqlx::SqlitePool;
use std::sync::{Arc, OnceLock};

const MAX_ARTIFACT_BYTES: usize = 24 * 1024 * 1024;
static ARTIFACT_SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

#[derive(Clone, Copy)]
pub enum ProductBookArtifactFormat { DocsHtml, BookHtml, Epub, Pdf }

impl ProductBookArtifactFormat {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "docs_html" => Ok(Self::DocsHtml),
            "book_html" => Ok(Self::BookHtml),
            "epub" => Ok(Self::Epub),
            "pdf" => Ok(Self::Pdf),
            _ => Err(AppError::Validation("format must be docs_html, book_html, epub or pdf".into())),
        }
    }
}

pub fn parse_trim(value: &str) -> Result<ProductBookTrim, AppError> {
    match value {
        "trade-6x9" => Ok(ProductBookTrim::Trade),
        "a5" => Ok(ProductBookTrim::A5),
        "us-letter" => Ok(ProductBookTrim::UsLetter),
        _ => Err(AppError::Validation("trim_preset must be trade-6x9, a5 or us-letter".into())),
    }
}

pub struct ProductBookArtifact {
    pub filename: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

pub async fn build_product_book_artifact(pool: &SqlitePool, product_id: &str, format: ProductBookArtifactFormat, trim: ProductBookTrim) -> Result<ProductBookArtifact, AppError> {
    let permit = ARTIFACT_SLOTS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(2)))
        .clone().acquire_owned().await.map_err(|_| AppError::Internal("Product Book export unavailable".into()))?;
    let (book_format, mode, suffix, content_type) = match format {
        ProductBookArtifactFormat::DocsHtml => (ProductBookFormat::Docs, ProductBookRenderMode::Web, "overview.html", "text/html; charset=utf-8"),
        ProductBookArtifactFormat::BookHtml => (ProductBookFormat::Book, ProductBookRenderMode::Web, "book.html", "text/html; charset=utf-8"),
        ProductBookArtifactFormat::Epub => (ProductBookFormat::Book, ProductBookRenderMode::Epub, "book.epub", "application/epub+zip"),
        ProductBookArtifactFormat::Pdf => (ProductBookFormat::Book, ProductBookRenderMode::Print, "book.pdf", "application/pdf"),
    };
    let rendered = render_product_book_for_product(pool, product_id, book_format, mode, trim).await?;
    if matches!(format, ProductBookArtifactFormat::Pdf) {
        let name = rendered["productName"].as_str().ok_or_else(|| AppError::Internal("Product Book title missing".into()))?;
        let html = rendered["html"].as_str().ok_or_else(|| AppError::Internal("Product Book HTML missing".into()))?;
        let input = html.to_owned();
        let (html, images) = tokio::task::spawn_blocking(move || prepare_book_images(&input, ImageTarget::Pdf)).await
            .map_err(|e| AppError::Internal(format!("Product Book image task failed: {e}")))??;
        drop(images);
        let bytes = crate::services::product_book_pdf::render_pdf(&html, name, &rendered["trimPreset"]).await?;
        drop(permit);
        return Ok(ProductBookArtifact { filename: format!("{}-{suffix}", filename_slug(name)), content_type, bytes });
    }
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let name = rendered["productName"].as_str().ok_or_else(|| AppError::Internal("Product Book title missing".into()))?;
        let html = rendered["html"].as_str().ok_or_else(|| AppError::Internal("Product Book HTML missing".into()))?;
        let bytes = if matches!(format, ProductBookArtifactFormat::Epub) {
            let (html, images) = prepare_book_images(html, ImageTarget::Epub)?;
            let toc: Vec<BookExportTocItem> = serde_json::from_value(rendered["tocItems"].clone())?;
            render_epub_archive_with_images(name, &html, &toc, "Aruvi Studio", "en", &images)?
        } else { html.as_bytes().to_vec() };
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(AppError::Validation("Product Book artifact exceeds the 24 MiB limit".into()));
        }
        Ok(ProductBookArtifact { filename: format!("{}-{suffix}", filename_slug(name)), content_type, bytes })
    }).await.map_err(|e| AppError::Internal(format!("Product Book export task failed: {e}")))?
}

fn filename_slug(name: &str) -> String {
    let mut result = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() { result.push(c.to_ascii_lowercase()); }
        else if !result.ends_with('-') && !result.is_empty() { result.push('-'); }
        if result.len() >= 80 { break; }
    }
    let result = result.trim_matches('-');
    if result.is_empty() { "product".into() } else { result.into() }
}

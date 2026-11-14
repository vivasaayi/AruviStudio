use crate::error::AppError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{ImageFormat, ImageReader, Limits};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::OnceLock;

const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 12 * 1024 * 1024;
const MAX_IMAGE_REFERENCES: usize = 32;

pub struct BookImage {
    pub filename: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy)]
pub enum ImageTarget { Epub, Pdf }

/// Rewrite self-contained raster images only. Files and remote URLs are never
/// fetched here: repository-scoped resolution needs explicit provenance.
pub fn prepare_book_images(html: &str, target: ImageTarget) -> Result<(String, Vec<BookImage>), AppError> {
    static TAG: OnceLock<Regex> = OnceLock::new();
    static SRC: OnceLock<Regex> = OnceLock::new();
    let tags = TAG.get_or_init(|| Regex::new(r"(?is)<img\b[^>]*>").unwrap());
    let source = SRC.get_or_init(|| Regex::new(r#"(?i)(?:^|\s)src="([^"]*)""#).unwrap());
    let mut output = String::with_capacity(html.len());
    let mut images = Vec::<BookImage>::new();
    let mut seen = HashMap::<[u8; 32], usize>::new();
    let mut last = 0;
    let mut total_bytes = 0_usize;
    for (count, tag) in tags.find_iter(html).enumerate() {
        if count >= MAX_IMAGE_REFERENCES { return Err(AppError::Validation("Product Book has too many images".into())); }
        let capture = source.captures(tag.as_str()).ok_or_else(|| AppError::Validation("Product Book image source is invalid".into()))?;
        let matched = capture.get(1).unwrap();
        let (media_type, format, suffix, encoded) = parse_source(matched.as_str())?;
        if encoded.len() > ((MAX_IMAGE_BYTES + 2) / 3) * 4 { return Err(AppError::Validation("Product Book image exceeds 4 MiB".into())); }
        let bytes = STANDARD.decode(encoded).map_err(|_| AppError::Validation("Product Book image has invalid base64".into()))?;
        if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES { return Err(AppError::Validation("Product Book image exceeds 4 MiB or is empty".into())); }
        if image::guess_format(&bytes).ok() != Some(format) { return Err(AppError::Validation("Product Book image type does not match its contents".into())); }
        if format == ImageFormat::Png && is_animated_png(&bytes) {
            return Err(AppError::Validation("Animated PNG images are not yet supported in Product Book exports".into()));
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let index = if let Some(index) = seen.get(&digest) { *index } else {
            total_bytes = total_bytes.saturating_add(bytes.len());
            if total_bytes > MAX_TOTAL_BYTES { return Err(AppError::Validation("Product Book images exceed 12 MiB total".into())); }
            let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
            let mut limits = Limits::default();
            limits.max_image_width = Some(2048);
            limits.max_image_height = Some(2048);
            limits.max_alloc = Some(32 * 1024 * 1024);
            reader.limits(limits);
            reader.decode().map_err(|_| AppError::Validation("Product Book image is corrupt or exceeds the 2048-pixel limit".into()))?;
            let hash = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
            let index = images.len();
            images.push(BookImage { filename: format!("{hash}.{suffix}"), media_type, bytes });
            seen.insert(digest, index);
            index
        };
        let replacement = match target {
            ImageTarget::Epub => format!("images/{}", images[index].filename),
            ImageTarget::Pdf => format!("data:{};base64,{}", images[index].media_type, STANDARD.encode(&images[index].bytes)),
        };
        let start = tag.start() + matched.start();
        let end = tag.start() + matched.end();
        output.push_str(&html[last..start]);
        output.push_str(&replacement);
        last = end;
    }
    output.push_str(&html[last..]);
    Ok((output, images))
}

fn is_animated_png(bytes: &[u8]) -> bool {
    let mut offset = 8_usize;
    while offset.saturating_add(12) <= bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        if &bytes[offset + 4..offset + 8] == b"acTL" { return true; }
        let Some(next) = offset.checked_add(size).and_then(|n| n.checked_add(12)) else { break; };
        if next > bytes.len() { break; }
        offset = next;
    }
    false
}

fn parse_source(source: &str) -> Result<(&'static str, ImageFormat, &'static str, &str), AppError> {
    let (header, encoded) = source.split_once(',').ok_or_else(|| AppError::Validation("Only embedded PNG and JPEG images are currently supported in Product Book exports".into()))?;
    if header.eq_ignore_ascii_case("data:image/png;base64") { Ok(("image/png", ImageFormat::Png, "png", encoded)) }
    else if header.eq_ignore_ascii_case("data:image/jpeg;base64") { Ok(("image/jpeg", ImageFormat::Jpeg, "jpg", encoded)) }
    else { Err(AppError::Validation("Only embedded PNG and JPEG images are currently supported in Product Book exports".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn fixture(format: ImageFormat, width: u32) -> String {
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(width, 2).write_to(&mut output, format).unwrap();
        let mime = if format == ImageFormat::Png { "image/png" } else { "image/jpeg" };
        format!("data:{mime};base64,{}", STANDARD.encode(output.into_inner()))
    }

    #[test]
    fn embedded_images_are_validated_deduplicated_and_packaged() {
        let png = fixture(ImageFormat::Png, 2);
        let jpeg = fixture(ImageFormat::Jpeg, 2);
        let html = format!("<html><head></head><body><img src=\"{png}\" /><img src=\"{png}\" /><img src=\"{jpeg}\" /></body></html>");
        let (epub_html, images) = prepare_book_images(&html, ImageTarget::Epub).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(epub_html.matches(&images[0].filename).count(), 2);
        assert!(!epub_html.contains("data:image"));
        let archive = crate::services::product_book_epub::render_epub_archive_with_images("Book", &epub_html, &[], "Author", "en", &images).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(archive)).unwrap();
        let mut manifest = String::new();
        zip.by_name("OEBPS/content.opf").unwrap().read_to_string(&mut manifest).unwrap();
        for image in &images {
            assert!(manifest.contains(&format!("images/{}", image.filename)));
            assert!(manifest.contains(image.media_type));
            let mut bytes = Vec::new();
            zip.by_name(&format!("OEBPS/images/{}", image.filename)).unwrap().read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, image.bytes);
        }
        let (pdf_html, _) = prepare_book_images(&html, ImageTarget::Pdf).unwrap();
        assert!(pdf_html.contains(&png));
        assert!(pdf_html.contains(&jpeg));
    }

    #[test]
    fn rejects_external_bad_oversized_and_mismatched_images() {
        let png = fixture(ImageFormat::Png, 2);
        for source in ["https://example.com/image.png".to_string(), "images/example.png".into(), "data:image/png;base64,%%%".into(), png.replace("image/png", "image/jpeg"), fixture(ImageFormat::Png, 2049)] {
            let html = format!("<img src=\"{source}\" />");
            assert!(prepare_book_images(&html, ImageTarget::Epub).is_err(), "source should be rejected");
        }
        assert!(prepare_book_images("<img />", ImageTarget::Epub).is_err());
        assert!(prepare_book_images("<img data-src=\"data:image/png;base64,AAAA\" />", ImageTarget::Epub).is_err());
        let repeated = format!("<img src=\"{png}\" />").repeat(MAX_IMAGE_REFERENCES + 1);
        assert!(prepare_book_images(&repeated, ImageTarget::Epub).is_err());
        let mut animated = STANDARD.decode(png.split_once(',').unwrap().1).unwrap();
        animated.splice(33..33, [0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(is_animated_png(&animated));
        let animated_source = format!("<img src=\"data:image/png;base64,{}\" />", STANDARD.encode(animated));
        assert!(prepare_book_images(&animated_source, ImageTarget::Epub).is_err());
    }
}

use crate::error::AppError;
use chrono::Utc;
use serde::Deserialize;
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

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

/// Package already-rendered XHTML as an EPUB 3 artifact without host utilities
/// or filesystem paths. The caller is responsible for rendering valid XHTML.
#[cfg(test)]
pub(crate) fn render_epub_archive(
    title: &str,
    html: &str,
    toc_items: &[BookExportTocItem],
    author: &str,
    language: &str,
) -> Result<Vec<u8>, AppError> {
    render_epub_archive_with_images(title, html, toc_items, author, language, &[])
}

pub(crate) fn render_epub_archive_with_images(
    title: &str,
    html: &str,
    toc_items: &[BookExportTocItem],
    author: &str,
    language: &str,
    images: &[crate::services::product_book_images::BookImage],
) -> Result<Vec<u8>, AppError> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let zip_error = |error| AppError::Internal(format!("Failed to create EPUB archive: {error}"));
    // EPUB requires the uncompressed mimetype as the first entry, without an
    // extra field. ZipWriter emits no extras for these simple file options.
    archive.start_file("mimetype", stored).map_err(zip_error)?;
    archive.write_all(b"application/epub+zip")?;
    archive.start_file("META-INF/container.xml", compressed).map_err(zip_error)?;
    archive.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#)?;
    archive.start_file("OEBPS/book.xhtml", compressed).map_err(zip_error)?;
    archive.write_all(html.as_bytes())?;
    archive.start_file("OEBPS/nav.xhtml", compressed).map_err(zip_error)?;
    archive.write_all(build_epub_nav_document(title, toc_items, language).as_bytes())?;
    let mut image_manifest = String::new();
    for (index, image) in images.iter().enumerate() {
        archive.start_file(format!("OEBPS/images/{}", image.filename), compressed).map_err(zip_error)?;
        archive.write_all(&image.bytes)?;
        image_manifest.push_str(&format!("    <item id=\"image-{index}\" href=\"images/{}\" media-type=\"{}\"/>\n", image.filename, image.media_type));
    }
    let identifier = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let metadata = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
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
{}
  </manifest>
  <spine><itemref idref="nav"/><itemref idref="book"/></spine>
</package>"#,
        escape_xml(&identifier), escape_xml(title), escape_xml(author),
        escape_xml(language), Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), image_manifest);
    archive.start_file("OEBPS/content.opf", compressed).map_err(zip_error)?;
    archive.write_all(metadata.as_bytes())?;
    Ok(archive.finish().map_err(zip_error)?.into_inner())
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
    nodes
        .iter()
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
    value.chars().filter(|&c| matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[tokio::test]
    #[ignore = "requires a fixture captured from the desktop Product Book UI"]
    async fn desktop_renderer_fixture_produces_real_epub() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fixture {
            title: String,
            html: String,
            toc_items: Vec<BookExportTocItem>,
            author: String,
            language: String,
        }
        let fixture_path = std::env::var("ARUVI_EPUB_FIXTURE_PATH").expect("fixture path required");
        let output_path = std::env::var("ARUVI_EPUB_TEST_OUTPUT").expect("output path required");
        let fixture: Fixture = serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
        let bytes = render_epub_archive(&fixture.title, &fixture.html, &fixture.toc_items, &fixture.author, &fixture.language).unwrap();
        std::fs::write(output_path, bytes).unwrap();
    }

    #[test]
    fn epub_is_a_readable_archive_with_required_order_and_escaped_navigation() {
        let html = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body><h1 id=\"part\">Book</h1><h2 id=\"sub\">Story</h2></body></html>";
        let items = vec![
            BookExportTocItem { id: "part".into(), title: "Part & <one>".into(), level: 0 },
            BookExportTocItem { id: "sub".into(), title: "Story 'two'".into(), level: 1 },
        ];
        let bytes = render_epub_archive("Title & <book>\u{0}", html, &items, "Author & Co", "en").unwrap();
        // The first local file header must name mimetype, use STORE, and have
        // no extra field, as required by EPUB's OCF container specification.
        assert_eq!(&bytes[0..4], b"PK\x03\x04");
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 0);
        assert_eq!(u16::from_le_bytes([bytes[28], bytes[29]]), 0);
        assert_eq!(&bytes[30..38], b"mimetype");
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(zip.len(), 5);
        let mut mimetype = String::new();
        zip.by_index(0).unwrap().read_to_string(&mut mimetype).unwrap();
        assert_eq!(mimetype, "application/epub+zip");
        let mut book = String::new();
        zip.by_name("OEBPS/book.xhtml").unwrap().read_to_string(&mut book).unwrap();
        assert_eq!(book, html);
        let mut nav = String::new();
        zip.by_name("OEBPS/nav.xhtml").unwrap().read_to_string(&mut nav).unwrap();
        assert!(nav.contains("Part &amp; &lt;one&gt;"));
        assert!(nav.contains("book.xhtml#part"));
        assert!(nav.contains("book.xhtml#sub"));
        assert!(nav.contains("Story &apos;two&apos;"));
        let mut metadata = String::new();
        zip.by_name("OEBPS/content.opf").unwrap().read_to_string(&mut metadata).unwrap();
        assert!(metadata.contains("Title &amp; &lt;book&gt;"));
        assert!(!metadata.contains('\u{0}'));
        assert!(metadata.contains("Author &amp; Co"));
        assert!(zip.by_name("META-INF/container.xml").is_ok());
    }
}

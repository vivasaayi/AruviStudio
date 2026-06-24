use crate::error::AppError;
use chrono::Utc;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::Command;

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

pub(super) fn export_epub_archive(
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
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

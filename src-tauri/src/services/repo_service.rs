use std::fs;
use std::path::{Component, Path};

use crate::domain::repository::RepositoryTreeNode;
use crate::error::AppError;
pub use crate::services::repo_file_service::{
    apply_repository_patch, get_repository_file_sha256, read_repository_file, write_repository_file,
};
use crate::services::repo_path_service::canonicalize_repo_root;

const MAX_TREE_DEPTH: usize = 12;
const MAX_TREE_ENTRIES: usize = 10_000;
const EXCLUDED_DIRECTORIES: [&str; 7] = [
    ".git",
    "node_product_areas",
    "dist",
    "build",
    "target",
    ".next",
    ".turbo",
];

pub fn list_repository_tree(
    repo_root: &str,
    include_hidden: bool,
    max_depth: Option<usize>,
) -> Result<Vec<RepositoryTreeNode>, AppError> {
    let root = canonicalize_repo_root(repo_root)?;
    let depth_limit = max_depth.unwrap_or(MAX_TREE_DEPTH).clamp(1, MAX_TREE_DEPTH);
    let mut node_count = 0usize;
    build_directory_nodes(
        &root,
        &root,
        0,
        depth_limit,
        include_hidden,
        &mut node_count,
    )
}

fn build_directory_nodes(
    root: &Path,
    directory: &Path,
    depth: usize,
    depth_limit: usize,
    include_hidden: bool,
    node_count: &mut usize,
) -> Result<Vec<RepositoryTreeNode>, AppError> {
    if depth > depth_limit || *node_count >= MAX_TREE_ENTRIES {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, std::io::Error>>()?;

    entries.sort_by(|left, right| {
        let left_is_dir = left.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let right_is_dir = right.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        right_is_dir.cmp(&left_is_dir).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });

    let mut result = Vec::new();
    for entry in entries {
        if *node_count >= MAX_TREE_ENTRIES {
            break;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type()?;
        let path = entry.path();
        let relative_path = to_unix_relative_path(root, &path)?;

        if file_type.is_dir() {
            if EXCLUDED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            *node_count += 1;
            let children = if depth < depth_limit {
                build_directory_nodes(
                    root,
                    &path,
                    depth + 1,
                    depth_limit,
                    include_hidden,
                    node_count,
                )?
            } else {
                Vec::new()
            };
            result.push(RepositoryTreeNode {
                name,
                relative_path,
                node_type: "directory".to_string(),
                size_bytes: None,
                children,
            });
            continue;
        }

        if file_type.is_file() {
            *node_count += 1;
            let size_bytes = entry.metadata().ok().map(|meta| meta.len());
            result.push(RepositoryTreeNode {
                name,
                relative_path,
                node_type: "file".to_string(),
                size_bytes,
                children: Vec::new(),
            });
        }
    }

    Ok(result)
}

fn to_unix_relative_path(root: &Path, path: &Path) -> Result<String, AppError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        AppError::Validation(format!(
            "Path is outside repository boundary: {}",
            path.display()
        ))
    })?;
    let joined = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    Ok(joined)
}

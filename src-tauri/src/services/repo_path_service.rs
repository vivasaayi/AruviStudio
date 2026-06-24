use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

pub(crate) fn canonicalize_repo_root(repo_root: &str) -> Result<PathBuf, AppError> {
    let root = PathBuf::from(repo_root);
    if !root.exists() || !root.is_dir() {
        return Err(AppError::Validation(format!(
            "Repository path is invalid or not a directory: {}",
            repo_root
        )));
    }
    Ok(fs::canonicalize(root)?)
}

pub(crate) fn resolve_repository_relative_path(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    let normalized = relative_path.replace('\\', "/");
    let input = Path::new(&normalized);
    if input.is_absolute() {
        return Err(AppError::Validation(format!(
            "Absolute paths are not allowed: {}",
            relative_path
        )));
    }

    let mut clean = PathBuf::new();
    for component in input.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => clean.push(part),
            _ => {
                return Err(AppError::Validation(format!(
                    "Path traversal is not allowed: {}",
                    relative_path
                )));
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(AppError::Validation(
            "Relative path cannot be empty".to_string(),
        ));
    }

    Ok(root.join(clean))
}

pub(crate) fn canonicalize_nearest_existing_ancestor(path: &Path) -> Result<PathBuf, AppError> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return Ok(fs::canonicalize(candidate)?);
        }
        current = candidate.parent();
    }

    Err(AppError::Validation(
        "Unable to resolve a valid parent path for file operation".to_string(),
    ))
}

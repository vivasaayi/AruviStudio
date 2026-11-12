use crate::domain::repository::Repository;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::fs;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct WorkItemWorkspace {
    pub work_item_id: String,
    pub base_path: PathBuf,
    pub repo_path: PathBuf,
}

impl WorkItemWorkspace {
    fn resolve_relative_repo_path(&self, relative_path: &str) -> Result<PathBuf, AppError> {
        let normalized = relative_path.replace('\\', "/");
        let candidate = Path::new(&normalized);
        if candidate.is_absolute() {
            return Err(AppError::Validation(format!(
                "Absolute paths are not allowed: {}",
                relative_path
            )));
        }

        let mut clean = PathBuf::new();
        for component in candidate.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::Normal(value) => clean.push(value),
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
                "Empty relative path is not allowed".to_string(),
            ));
        }

        Ok(self.repo_path.join(clean))
    }

    /// Create a new isolated workspace for a work item
    pub async fn create(
        work_item: &WorkItem,
        workflow_run_id: &str,
        repo: &Repository,
        workspace_base_path: &Path,
    ) -> Result<Self, AppError> {
        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run_id, repo_path = %repo.local_path, "Creating work item workspace");
        let workspace_id = Uuid::new_v4().to_string();
        let base_path = workspace_base_path.join(&workspace_id);

        // Create workspace directory
        fs::create_dir_all(&base_path).await
            .map_err(|e| {
                error!(work_item_id = %work_item.id, workspace_id = %workspace_id, error = %e, "Failed to create workspace directory");
                AppError::Io(e)
            })?;

        // Copy repository to workspace
        let repo_path = base_path.join("repo");
        Self::copy_repository(&repo.local_path, &repo_path).await?;

        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run_id, workspace_path = ?base_path, "Successfully created work item workspace");
        Ok(Self {
            work_item_id: work_item.id.clone(),
            base_path,
            repo_path,
        })
    }

    /// Copy repository to workspace
    async fn copy_repository(source: &str, destination: &PathBuf) -> Result<(), AppError> {
        info!(source = %source, destination = ?destination, "Copying repository to workspace");
        // For now, create a git worktree. In production, this should be more sophisticated
        // to handle large repositories efficiently
        fs::create_dir_all(destination).await
            .map_err(|e| {
                error!(source = %source, destination = ?destination, error = %e, "Failed to create destination directory");
                AppError::Io(e)
            })?;

        // TODO: Implement proper git worktree or shallow clone
        // For MVP, just copy the directory
        Self::copy_dir_recursive(source, destination).await?;
        info!(source = %source, destination = ?destination, "Successfully copied repository to workspace");
        Ok(())
    }

    /// Recursively copy a directory
    fn copy_dir_recursive<'a>(
        src: &'a str,
        dst: &'a PathBuf,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            debug!(src = %src, dst = ?dst, "Starting recursive directory copy");
            let src_path = Path::new(src);
            let dst_path = dst;

            if !src_path.exists() {
                error!(src = %src, "Source path does not exist");
                return Err(AppError::Internal(format!(
                    "Source path does not exist: {}",
                    src
                )));
            }

            if src_path.is_dir() {
                debug!(src = %src, "Source is a directory, creating destination directory");
                fs::create_dir_all(dst_path).await
                    .map_err(|e| {
                        error!(src = %src, dst = ?dst, error = %e, "Failed to create destination directory");
                        AppError::Io(e)
                    })?;

                let mut entries = fs::read_dir(src_path).await.map_err(|e| {
                    error!(src = %src, error = %e, "Failed to read source directory");
                    AppError::Io(e)
                })?;

                while let Some(entry) = entries.next_entry().await.map_err(|e| {
                    error!(src = %src, error = %e, "Failed to read directory entry");
                    AppError::Io(e)
                })? {
                    let entry_path = entry.path();
                    // Never follow repository symlinks into files outside the copied workspace.
                    if entry.file_type().await?.is_symlink() { continue; }
                    let file_name = entry_path.file_name().unwrap();
                    let dst_entry = dst_path.join(file_name);

                    if entry_path.is_dir() {
                        // Skip .git directory for workspace isolation
                        if file_name != ".git" {
                            debug!(entry = ?entry_path, "Recursively copying subdirectory");
                            Self::copy_dir_recursive(&entry_path.to_string_lossy(), &dst_entry)
                                .await?;
                        } else {
                            debug!(entry = ?entry_path, "Skipping .git directory");
                        }
                    } else {
                        debug!(entry = ?entry_path, dst = ?dst_entry, "Copying file");
                        fs::copy(&entry_path, &dst_entry).await
                            .map_err(|e| {
                                error!(src = ?entry_path, dst = ?dst_entry, error = %e, "Failed to copy file");
                                AppError::Io(e)
                            })?;
                    }
                }
            } else {
                debug!(src = %src, dst = ?dst, "Source is a file, copying directly");
                fs::copy(src_path, dst_path).await.map_err(|e| {
                    error!(src = %src, dst = ?dst, error = %e, "Failed to copy file");
                    AppError::Io(e)
                })?;
            }

            debug!(src = %src, dst = ?dst, "Completed recursive directory copy");
            Ok(())
        })
    }

    /// Get the path to a file in the workspace
    pub fn get_file_path(&self, relative_path: &str) -> PathBuf {
        debug!(work_item_id = %self.work_item_id, relative_path = %relative_path, "Getting file path in workspace");
        self.repo_path.join(relative_path)
    }

    /// Read a file from the workspace
    pub async fn read_file(&self, relative_path: &str) -> Result<String, AppError> {
        debug!(work_item_id = %self.work_item_id, relative_path = %relative_path, "Reading file from workspace");
        let path = self.get_file_path(relative_path);
        fs::read_to_string(&path).await
            .map_err(|e| {
                error!(work_item_id = %self.work_item_id, path = ?path, error = %e, "Failed to read file");
                AppError::Internal(format!("Failed to read file {}: {}", path.display(), e))
            })
    }

    /// Write a file to the workspace
    pub async fn write_file(&self, relative_path: &str, content: &str) -> Result<(), AppError> {
        let path = self.resolve_relative_repo_path(relative_path)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(AppError::Io)?;
        }

        fs::write(&path, content).await.map_err(|e| {
            AppError::Internal(format!("Failed to write file {}: {}", path.display(), e))
        })
    }

    pub async fn sync_files_back(
        &self,
        target_repo_path: &str,
        relative_paths: &[String],
        previous_files: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), AppError> {
        let target_root = Path::new(target_repo_path);
        if !target_root.is_dir() { return Err(AppError::Validation("Target repository path is unavailable".to_string())); }
        let git = git2::Repository::open(target_root).ok();
        let mut files = Vec::new();
        // Preflight every destination before writing anything, including existing dirty files.
        for relative_path in relative_paths {
            let relative = safe_relative_path(relative_path)?;
            let source = confined_file_path(&self.repo_path, &relative, false)?;
            let destination = confined_file_path(target_root, &relative, true)?;
            if let Some(repo) = &git {
                match repo.status_file(&relative) {
                    Ok(status) if status.is_empty() => {},
                    Err(error) if error.code() == git2::ErrorCode::NotFound && !destination.exists() => {},
                    Ok(status) if status.intersects(git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED | git2::Status::INDEX_DELETED | git2::Status::INDEX_RENAMED | git2::Status::INDEX_TYPECHANGE | git2::Status::CONFLICTED) => return Err(AppError::Validation(format!("Cannot overwrite a staged repository file: {}", relative.display()))),
                    _ => {
                        let name = relative.to_str().ok_or_else(|| AppError::Validation("Repository path must be UTF-8".to_string()))?;
                        let matches_previous = previous_files.get(name).is_some_and(|expected| {
                            crate::services::workflow_delivery_manifest::file_digest(target_root, &relative).is_ok_and(|actual| &actual == expected)
                        });
                        if !matches_previous { return Err(AppError::Validation(format!("Cannot overwrite a changed or untracked repository file: {}", relative.display()))); }
                    },
                }
            } else if destination.exists() {
                return Err(AppError::Validation(format!("Cannot safely overwrite an existing file outside a Git repository: {}", relative.display())));
            }
            files.push((relative, source, destination));
        }
        for (relative, source, destination) in files {
            if let Some(parent) = destination.parent() { fs::create_dir_all(parent).await?; }
            // Recheck after creating directories. Non-cooperating filesystem writers may still race.
            confined_file_path(target_root, &relative, true)?;
            fs::copy(&source, &destination).await.map_err(|error| AppError::Internal(format!("Failed to sync file {} back to repository: {}", relative.display(), error)))?;
        }

        Ok(())
    }

    /// Clean up the workspace
    pub async fn cleanup(&self) -> Result<(), AppError> {
        // In production, this should be more careful about what gets deleted
        fs::remove_dir_all(&self.base_path)
            .await
            .map_err(AppError::Io)?;
        Ok(())
    }
}

/// Validate an agent-supplied path before using it in the workspace or repository.
pub(crate) fn safe_relative_path(value: &str) -> Result<PathBuf, AppError> {
    if value.contains('\n') || value.contains('\r') || value.contains('\0') { return Err(AppError::Validation("Invalid repository file path".to_string())); }
    let normalized = value.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute() { return Err(AppError::Validation("Absolute repository paths are not allowed".to_string())); }
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) if part != ".git" => clean.push(part),
            std::path::Component::CurDir => {},
            _ => return Err(AppError::Validation("Invalid repository file path".to_string())),
        }
    }
    if clean.as_os_str().is_empty() { return Err(AppError::Validation("Empty repository file path".to_string())); }
    Ok(clean)
}

/// Reject symlinked components and non-regular files. Existing parents are checked even when
/// the final file does not exist. This does not lock out unrelated processes changing the tree.
pub(crate) fn confined_file_path(root: &Path, relative: &Path, allow_missing: bool) -> Result<PathBuf, AppError> {
    let root = std::fs::canonicalize(root)?;
    let mut current = root;
    let count = relative.components().count();
    for (index, component) in relative.components().enumerate() {
        let std::path::Component::Normal(part) = component else { return Err(AppError::Validation("Invalid repository file path".to_string())); };
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() { return Err(AppError::Validation(format!("Symlinked repository path is not allowed: {}", relative.display()))); }
                if index + 1 < count && !metadata.is_dir() { return Err(AppError::Validation("Repository path parent is not a directory".to_string())); }
                if index + 1 == count {
                    if !metadata.is_file() { return Err(AppError::Validation("Repository path is not a regular file".to_string())); }
                    #[cfg(unix)] { use std::os::unix::fs::MetadataExt; if metadata.nlink() > 1 { return Err(AppError::Validation("Hard-linked repository files are not supported".to_string())); } }
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_missing => {},
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

#[cfg(test)]
mod delivery_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn sync_rejects_existing_untracked_and_preflights_all_files() {
        let root = std::env::temp_dir().join(format!("aruvi-sync-{}", uuid::Uuid::new_v4()));
        let target = root.join("target"); let source = root.join("workspace");
        std::fs::create_dir_all(&target).unwrap(); std::fs::create_dir_all(&source).unwrap();
        git2::Repository::init(&target).unwrap();
        std::fs::write(target.join("existing.txt"), "user content").unwrap();
        std::fs::write(source.join("existing.txt"), "agent content").unwrap();
        std::fs::write(source.join("new.txt"), "new content").unwrap();
        let workspace = WorkItemWorkspace { work_item_id: "fixture".into(), base_path: root.clone(), repo_path: source };
        let result = workspace.sync_files_back(target.to_str().unwrap(), &["new.txt".into(), "existing.txt".into()], &BTreeMap::new()).await;
        assert!(result.is_err());
        assert!(!target.join("new.txt").exists());
        assert_eq!(std::fs::read_to_string(target.join("existing.txt")).unwrap(), "user content");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sync_and_copy_do_not_follow_symlinks() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("aruvi-sync-link-{}", uuid::Uuid::new_v4()));
        let target = root.join("target"); let source = root.join("workspace"); let outside = root.join("outside");
        std::fs::create_dir_all(&target).unwrap(); std::fs::create_dir_all(source.join("dir")).unwrap(); std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(source.join("dir/file.txt"), "agent content").unwrap();
        symlink(&outside, target.join("dir")).unwrap();
        let workspace = WorkItemWorkspace { work_item_id: "fixture".into(), base_path: root.clone(), repo_path: source.clone() };
        assert!(workspace.sync_files_back(target.to_str().unwrap(), &["dir/file.txt".into()], &BTreeMap::new()).await.is_err());
        assert!(!outside.join("file.txt").exists());
        symlink(&outside, source.join("outside-link")).unwrap();
        let copy = root.join("copy"); WorkItemWorkspace::copy_repository(source.to_str().unwrap(), &copy).await.unwrap();
        assert!(!copy.join("outside-link").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

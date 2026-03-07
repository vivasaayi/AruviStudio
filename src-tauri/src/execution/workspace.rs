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
    pub workflow_run_id: String,
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
            workflow_run_id: workflow_run_id.to_string(),
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

    /// Apply code changes to the workspace
    pub async fn apply_code_changes(&self, changes: &str) -> Result<(), AppError> {
        info!(work_item_id = %self.work_item_id, workflow_run_id = %self.workflow_run_id, "Applying code changes to workspace");
        // Parse the changes and apply them
        // This is a simplified implementation - in production, this would parse
        // structured output from the coding agent

        // For now, assume changes are in a simple format
        // TODO: Implement proper change parsing and application

        // Create a changes directory to track modifications
        let changes_dir = self.base_path.join("changes");
        fs::create_dir_all(&changes_dir).await
            .map_err(|e| {
                error!(work_item_id = %self.work_item_id, changes_dir = ?changes_dir, error = %e, "Failed to create changes directory");
                AppError::Io(e)
            })?;

        // Write changes summary
        let changes_file = changes_dir.join("changes.txt");
        fs::write(&changes_file, changes).await
            .map_err(|e| {
                error!(work_item_id = %self.work_item_id, changes_file = ?changes_file, error = %e, "Failed to write changes file");
                AppError::Io(e)
            })?;

        info!(work_item_id = %self.work_item_id, workflow_run_id = %self.workflow_run_id, changes_file = ?changes_file, "Successfully applied code changes");
        Ok(())
    }

    /// Get the path to a file in the workspace
    pub fn get_file_path(&self, relative_path: &str) -> PathBuf {
        debug!(work_item_id = %self.work_item_id, relative_path = %relative_path, "Getting file path in workspace");
        self.repo_path.join(relative_path)
    }

    /// Check if a file exists in the workspace
    pub async fn file_exists(&self, relative_path: &str) -> bool {
        let path = self.get_file_path(relative_path);
        let exists = path.exists();
        debug!(work_item_id = %self.work_item_id, path = ?path, exists = exists, "Checked file existence");
        exists
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
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(e))?;
        }

        fs::write(&path, content).await.map_err(|e| {
            AppError::Internal(format!("Failed to write file {}: {}", path.display(), e))
        })
    }

    pub async fn sync_files_back(
        &self,
        target_repo_path: &str,
        relative_paths: &[String],
    ) -> Result<(), AppError> {
        let target_root = Path::new(target_repo_path);
        if !target_root.exists() {
            return Err(AppError::Validation(format!(
                "Target repository path does not exist: {}",
                target_repo_path
            )));
        }

        for relative_path in relative_paths {
            let source_path = self.resolve_relative_repo_path(relative_path)?;
            let destination_path = target_root.join(relative_path);
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::copy(&source_path, &destination_path)
                .await
                .map_err(|error| {
                    AppError::Internal(format!(
                        "Failed to sync file {} back to repository: {}",
                        relative_path, error
                    ))
                })?;
        }

        Ok(())
    }

    /// Clean up the workspace
    pub async fn cleanup(&self) -> Result<(), AppError> {
        // In production, this should be more careful about what gets deleted
        fs::remove_dir_all(&self.base_path)
            .await
            .map_err(|e| AppError::Io(e))?;
        Ok(())
    }
}

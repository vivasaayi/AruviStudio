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
    /// Create a new isolated workspace for a work item
    pub async fn create(
        work_item: &WorkItem,
        workflow_run_id: &str,
        repo: &Repository,
        workspace_base_path: &Path,
    ) -> Result<Self, AppError> {
        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run_id, repo_path = %repo.local_path, "Creating work item workspace");
        let workspace_id = Uuid::new_v4().to_string();
        let (base_path, repo_path) = Self::create_workspace_copy(&repo.local_path, workspace_base_path, &workspace_id).await?;

        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run_id, workspace_path = ?base_path, "Successfully created work item workspace");
        Ok(Self {
            work_item_id: work_item.id.clone(),
            base_path,
            repo_path,
        })
    }

    async fn create_workspace_copy(source: &str, workspace_base_path: &Path, workspace_id: &str) -> Result<(PathBuf, PathBuf), AppError> {
        fs::create_dir_all(workspace_base_path).await?;
        let base_path = workspace_base_path.join(workspace_id);
        // Exclusive creation ensures a collision is never mistaken for a directory we own.
        fs::create_dir(&base_path).await?;
        let repo_path = base_path.join("repo");
        if let Err(copy_error) = Self::copy_repository(source, &repo_path).await {
            if let Err(cleanup_error) = fs::remove_dir_all(&base_path).await {
                error!(workspace_path = ?base_path, error = %cleanup_error, "Failed to remove partial workspace after copy failure");
            }
            return Err(copy_error);
        }
        Ok((base_path, repo_path))
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

    /// Read a file from the workspace
    pub async fn read_file(&self, relative_path: &str) -> Result<String, AppError> {
        debug!(work_item_id = %self.work_item_id, relative_path = %relative_path, "Reading file from workspace");
        let relative = safe_relative_path(relative_path)?;
        let missing = || AppError::NotFound(format!("Repository file does not exist: {}", relative.display()));
        let path = confined_file_path(&self.repo_path, &relative, false).map_err(|error| match error {
            AppError::Io(ref source) if source.kind() == std::io::ErrorKind::NotFound => missing(),
            other => other,
        })?;
        fs::read_to_string(&path).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound { return missing(); }
            error!(work_item_id = %self.work_item_id, path = ?path, error = %error, "Failed to read file");
            AppError::Internal(format!("Failed to read repository file {}", relative.display()))
        })
    }

    /// Write a file to the workspace
    pub async fn write_file(&self, relative_path: &str, content: &str) -> Result<(), AppError> {
        let relative = safe_relative_path(relative_path)?;
        let root = self.repo_path.clone();
        let content = content.to_owned();
        tokio::task::spawn_blocking(move || write_confined_file(&root, &relative, content.as_bytes()))
            .await.map_err(|error| AppError::Internal(format!("Workspace write task failed: {error}")))?
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

// Use directory descriptors on Unix so a symlink substituted after validation cannot
// redirect an agent write. Do not truncate until the opened file has been checked.
#[cfg(unix)]
fn write_confined_file(root: &Path, relative: &Path, content: &[u8]) -> Result<(), AppError> {
    use std::ffi::CString;
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let mut directory = std::fs::OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)?;
    let mut parts = relative.components().peekable();
    while let Some(component) = parts.next() {
        let std::path::Component::Normal(part) = component else {
            return Err(AppError::Validation("Invalid repository file path".to_string()));
        };
        let name = CString::new(part.as_bytes()).map_err(|_| AppError::Validation("Invalid repository file path".to_string()))?;
        if parts.peek().is_some() {
            // SAFETY: the directory descriptor and name are valid for this call.
            let result = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o777) };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::AlreadyExists { return Err(error.into()); }
            }
            // SAFETY: openat returns a new descriptor owned by the File below.
            let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
            if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
            directory = unsafe { std::fs::File::from_raw_fd(fd) };
        } else {
            // SAFETY: openat returns a new descriptor owned by the File below.
            let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK, 0o666) };
            if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
            let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(AppError::Validation("Workspace destination must be a regular file without hard links".to_string()));
            }
            file.set_len(0)?;
            file.write_all(content)?;
            return Ok(());
        }
    }
    Err(AppError::Validation("Empty repository file path".to_string()))
}

#[cfg(not(unix))]
fn write_confined_file(root: &Path, relative: &Path, content: &[u8]) -> Result<(), AppError> {
    let path = confined_file_path(root, relative, true)?;
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let path = confined_file_path(root, relative, true)?;
    std::fs::write(path, content)?;
    Ok(())
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
    async fn failed_workspace_copy_removes_only_the_new_directory() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("workspaces");
        let existing = base.join("existing");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("sentinel"), "untouched").unwrap();
        let missing = root.path().join("missing");
        assert!(WorkItemWorkspace::create_workspace_copy(missing.to_str().unwrap(), &base, "new").await.is_err());
        assert!(!base.join("new").exists());
        assert_eq!(std::fs::read_to_string(existing.join("sentinel")).unwrap(), "untouched");
        assert!(WorkItemWorkspace::create_workspace_copy(missing.to_str().unwrap(), &base, "existing").await.is_err());
        assert_eq!(std::fs::read_to_string(existing.join("sentinel")).unwrap(), "untouched");
        let source = root.path().join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("fixture"), "copied").unwrap();
        let (workspace, repo) = WorkItemWorkspace::create_workspace_copy(source.to_str().unwrap(), &base, "success").await.unwrap();
        assert_eq!(workspace, base.join("success"));
        assert_eq!(std::fs::read_to_string(repo.join("fixture")).unwrap(), "copied");
    }

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

    #[tokio::test]
    async fn workspace_reads_confined_files_and_reports_missing_relative_path() {
        let root = std::env::temp_dir().join(format!("aruvi-read-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("visible.txt"), "content").unwrap();
        let workspace = WorkItemWorkspace { work_item_id: "fixture".into(), base_path: root.clone(), repo_path: root.clone() };
        assert_eq!(workspace.read_file("visible.txt").await.unwrap(), "content");
        let error = workspace.read_file("new/file.txt").await.unwrap_err().to_string();
        assert!(error.contains("Repository file does not exist: new/file.txt"));
        assert!(!error.contains(root.to_str().unwrap()));
        assert!(workspace.read_file("../outside").await.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_read_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("aruvi-read-link-{}", uuid::Uuid::new_v4()));
        let repo = root.join("repo"); std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(root.join("secret.txt"), "private").unwrap();
        symlink(root.join("secret.txt"), repo.join("link.txt")).unwrap();
        let workspace = WorkItemWorkspace { work_item_id: "fixture".into(), base_path: root.clone(), repo_path: repo };
        assert!(workspace.read_file("link.txt").await.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_write_rejects_links_without_modifying_their_targets() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("target"), "original").unwrap();
        symlink(&outside, repo.join("linked-dir")).unwrap();
        symlink(outside.join("target"), repo.join("linked-file")).unwrap();
        std::fs::hard_link(outside.join("target"), repo.join("hard-link")).unwrap();
        let workspace = WorkItemWorkspace { work_item_id: "fixture".into(), base_path: root.path().to_owned(), repo_path: repo.clone() };

        for path in ["linked-dir/target", "linked-file", "hard-link", "../outside/target", ".git/config"] {
            assert!(workspace.write_file(path, "changed").await.is_err(), "write should reject {path}");
            assert_eq!(std::fs::read_to_string(outside.join("target")).unwrap(), "original");
        }
        workspace.write_file("new/nested/file.txt", "created").await.unwrap();
        workspace.write_file("new/nested/file.txt", "updated").await.unwrap();
        assert_eq!(std::fs::read_to_string(repo.join("new/nested/file.txt")).unwrap(), "updated");
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

// Git operations - branch creation, diff generation, commits via git2

use crate::error::AppError;
use git2::Repository;
use tracing::{error, info, warn};

pub struct GitOperations;

impl GitOperations {
    /// Stage all changes in the working directory
    pub fn stage_all_changes(repo_path: &str) -> Result<(), AppError> {
        info!(repo_path = %repo_path, "Staging all changes");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        let mut index = repo.index().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get index");
            AppError::Internal(format!("Failed to get index: {}", e))
        })?;

        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| {
                error!(repo_path = %repo_path, error = %e, "Failed to stage files");
                AppError::Internal(format!("Failed to stage files: {}", e))
            })?;

        index.write().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to write index");
            AppError::Internal(format!("Failed to write index: {}", e))
        })?;

        info!(repo_path = %repo_path, "Successfully staged all changes");
        Ok(())
    }

    /// Create a commit with the given message
    pub fn create_commit(repo_path: &str, message: &str) -> Result<String, AppError> {
        info!(repo_path = %repo_path, message = %message, "Creating commit");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        let mut index = repo.index().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get index");
            AppError::Internal(format!("Failed to get index: {}", e))
        })?;

        // Check if there are any changes to commit
        let diff = repo.diff_index_to_workdir(None, None).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to check for changes");
            AppError::Internal(format!("Failed to check for changes: {}", e))
        })?;

        if diff.deltas().len() == 0 {
            warn!(repo_path = %repo_path, "No changes to commit");
            return Err(AppError::Internal("No changes to commit".to_string()));
        }

        // Stage changes
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| {
                error!(repo_path = %repo_path, error = %e, "Failed to stage files");
                AppError::Internal(format!("Failed to stage files: {}", e))
            })?;
        index.write().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to write index");
            AppError::Internal(format!("Failed to write index: {}", e))
        })?;

        let tree_id = index.write_tree().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to write tree");
            AppError::Internal(format!("Failed to write tree: {}", e))
        })?;
        let tree = repo.find_tree(tree_id).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to find tree");
            AppError::Internal(format!("Failed to find tree: {}", e))
        })?;

        // Get parent commit
        let parent_commit = if let Ok(head) = repo.head() {
            Some(head.peel_to_commit().map_err(|e| {
                error!(repo_path = %repo_path, error = %e, "Failed to get parent commit");
                AppError::Internal(format!("Failed to get parent commit: {}", e))
            })?)
        } else {
            None
        };

        let parents = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();

        // Create commit
        let signature = repo.signature().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get signature");
            AppError::Internal(format!("Failed to get signature: {}", e))
        })?;

        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        ).map_err(|e| {
            error!(repo_path = %repo_path, message = %message, error = %e, "Failed to create commit");
            AppError::Internal(format!("Failed to create commit: {}", e))
        })?;

        info!(repo_path = %repo_path, commit_id = %commit_id, message = %message, "Successfully created commit");
        Ok(commit_id.to_string())
    }

    /// Push the current branch to remote
    pub fn push_to_remote(
        repo_path: &str,
        remote_name: &str,
        branch_name: &str,
    ) -> Result<(), AppError> {
        info!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, "Pushing to remote");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        // Find the remote
        let mut remote = repo.find_remote(remote_name)
            .map_err(|e| {
                error!(repo_path = %repo_path, remote_name = %remote_name, error = %e, "Failed to find remote");
                AppError::Internal(format!("Failed to find remote: {}", e))
            })?;

        // Push the branch
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote.push(&[&refspec], None)
            .map_err(|e| {
                error!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, refspec = %refspec, error = %e, "Failed to push");
                AppError::Internal(format!("Failed to push: {}", e))
            })?;

        info!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, "Successfully pushed to remote");
        Ok(())
    }
}

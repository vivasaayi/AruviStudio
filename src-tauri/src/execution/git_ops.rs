// Git operations - branch creation, diff generation, commits via git2

use crate::domain::repository::Repository as RepoDomain;
use crate::error::AppError;
use git2::{BranchType, DiffFormat, DiffOptions, Repository};
use std::path::Path;
use tracing::{debug, error, info, warn};

pub struct GitOperations;

impl GitOperations {
    /// Create a new branch for the work item
    pub fn create_work_item_branch(repo_path: &str, branch_name: &str) -> Result<(), AppError> {
        info!(repo_path = %repo_path, branch_name = %branch_name, "Creating work item branch");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        // Get the current HEAD
        let head = repo.head().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get HEAD");
            AppError::Internal(format!("Failed to get HEAD: {}", e))
        })?;
        let head_commit = head.peel_to_commit().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get HEAD commit");
            AppError::Internal(format!("Failed to get HEAD commit: {}", e))
        })?;

        // Create the new branch
        repo.branch(branch_name, &head_commit, false)
            .map_err(|e| {
                error!(repo_path = %repo_path, branch_name = %branch_name, error = %e, "Failed to create branch");
                AppError::Internal(format!("Failed to create branch: {}", e))
            })?;

        info!(repo_path = %repo_path, branch_name = %branch_name, "Successfully created work item branch");
        Ok(())
    }

    /// Generate diff between current branch and base branch
    pub fn generate_diff(repo_path: &str, base_branch: &str) -> Result<String, AppError> {
        info!(repo_path = %repo_path, base_branch = %base_branch, "Generating diff");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        // Get the base branch
        let base_ref = repo.find_branch(base_branch, BranchType::Local)
            .map_err(|e| {
                error!(repo_path = %repo_path, base_branch = %base_branch, error = %e, "Failed to find base branch");
                AppError::Internal(format!("Failed to find base branch: {}", e))
            })?;
        let base_commit = base_ref.get().peel_to_commit()
            .map_err(|e| {
                error!(repo_path = %repo_path, base_branch = %base_branch, error = %e, "Failed to get base commit");
                AppError::Internal(format!("Failed to get base commit: {}", e))
            })?;

        // Get current HEAD
        let head_commit = repo.head()?.peel_to_commit().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get HEAD commit");
            AppError::Internal(format!("Failed to get HEAD commit: {}", e))
        })?;

        // Generate diff
        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(
            Some(&base_commit.tree()?),
            Some(&head_commit.tree()?),
            Some(&mut diff_opts),
        ).map_err(|e| {
            error!(repo_path = %repo_path, base_branch = %base_branch, error = %e, "Failed to generate diff");
            AppError::Internal(format!("Failed to generate diff: {}", e))
        })?;

        let mut diff_text = String::new();
        diff.print(DiffFormat::Patch, |delta, hunk, line| {
            match line.origin() {
                '+' => diff_text.push('+'),
                '-' => diff_text.push('-'),
                ' ' => diff_text.push(' '),
                _ => {}
            }
            diff_text.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|e| AppError::Internal(format!("Failed to format diff: {}", e)))?;

        Ok(diff_text)
    }

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

    /// Check if repository has uncommitted changes
    pub fn has_uncommitted_changes(repo_path: &str) -> Result<bool, AppError> {
        debug!(repo_path = %repo_path, "Checking for uncommitted changes");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        let diff = repo.diff_index_to_workdir(None, None).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to check for changes");
            AppError::Internal(format!("Failed to check for changes: {}", e))
        })?;

        let has_changes = diff.deltas().len() > 0;
        debug!(repo_path = %repo_path, has_changes = has_changes, "Checked for uncommitted changes");
        Ok(has_changes)
    }

    /// Get current branch name
    pub fn get_current_branch(repo_path: &str) -> Result<String, AppError> {
        debug!(repo_path = %repo_path, "Getting current branch");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        let head = repo.head().map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to get HEAD");
            AppError::Internal(format!("Failed to get HEAD: {}", e))
        })?;

        let branch_name = head.shorthand().ok_or_else(|| {
            error!(repo_path = %repo_path, "Not on a branch");
            AppError::Internal("Not on a branch".to_string())
        })?;

        debug!(repo_path = %repo_path, branch_name = %branch_name, "Got current branch");
        Ok(branch_name.to_string())
    }
}

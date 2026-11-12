// Git operations - branch creation, diff generation, commits via git2

use crate::error::AppError;
use git2::Repository;
use tracing::{error, info};

pub struct GitOperations;

impl GitOperations {
    /// Commit only files recorded for this workflow. Existing staged changes are never included.
    pub(crate) fn create_workflow_commit(
        repo_path: &str,
        branch_name: &str,
        manifest: &crate::services::workflow_delivery_manifest::DeliveryManifest,
        message: &str,
    ) -> Result<String, AppError> {
        use crate::execution::workspace::{confined_file_path, safe_relative_path};
        use sha2::{Digest, Sha256};
        let repo = Repository::open(repo_path)?;
        let head = repo.head().map_err(|_| AppError::Validation("A checked-out branch with an existing commit is required for delivery".to_string()))?;
        let expected_ref = format!("refs/heads/{branch_name}");
        if !git2::Reference::is_valid_name(&expected_ref) || head.name() != Some(expected_ref.as_str()) {
            return Err(AppError::Validation("The configured delivery branch is not the checked-out branch".to_string()));
        }
        let parent = head.peel_to_commit()?;
        if manifest.base_head.as_deref() != Some(parent.id().to_string().as_str()) {
            return Err(AppError::Validation("Repository HEAD changed since coding; review before delivery".to_string()));
        }
        if manifest.files.is_empty() { return Err(AppError::Validation("No workflow-generated files are recorded for delivery".to_string())); }
        repo.find_remote("origin").map_err(|_| AppError::Validation("Repository has no origin remote".to_string()))?;
        let signature = repo.signature().map_err(|_| AppError::Validation("Configure the repository Git user name and email before delivery".to_string()))?;
        let mut index = repo.index()?;
        if index.has_conflicts() { return Err(AppError::Validation("Resolve Git index conflicts before delivery".to_string())); }
        let parent_tree = parent.tree()?;
        if repo.diff_tree_to_index(Some(&parent_tree), Some(&index), None)?.deltas().len() != 0 {
            return Err(AppError::Validation("Repository has staged changes; unstage or commit them before workflow delivery".to_string()));
        }
        for (name, expected_hash) in &manifest.files {
            let relative = safe_relative_path(name)?;
            confined_file_path(std::path::Path::new(repo_path), &relative, false)?;
            let actual_hash = crate::services::workflow_delivery_manifest::file_digest(std::path::Path::new(repo_path), &relative)?;
            if &actual_hash != expected_hash { return Err(AppError::Validation(format!("Workflow file changed after coding: {}", relative.display()))); }
            if repo.status_file(&relative).is_ok_and(|status| status.contains(git2::Status::IGNORED)) {
                return Err(AppError::Validation(format!("Workflow file is ignored by Git: {}", relative.display())));
            }
            index.add_path(&relative)?;
            let entry = index.get_path(&relative, 0).ok_or_else(|| AppError::Internal("Unable to stage workflow file".to_string()))?;
            let blob = repo.find_blob(entry.id)?;
            if format!("{:x}", Sha256::digest(blob.content())) != *expected_hash {
                return Err(AppError::Validation(format!("Workflow file changed while staging: {}", relative.display())));
            }
        }
        let tree_id = index.write_tree()?;
        if tree_id == parent_tree.id() { return Err(AppError::Validation("Workflow files contain no changes to deliver".to_string())); }
        if repo.head()?.target() != Some(parent.id()) { return Err(AppError::Validation("Repository HEAD changed during delivery".to_string())); }
        index.write()?;
        let tree = repo.find_tree(tree_id)?;
        let commit = repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])?;
        Ok(commit.to_string())
    }

    /// Push the current branch to remote
    pub fn push_to_remote(
        repo_path: &str,
        remote_name: &str,
        branch_name: &str,
        expected_commit: &str,
    ) -> Result<(), AppError> {
        info!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, "Pushing to remote");
        let repo = Repository::open(repo_path).map_err(|e| {
            error!(repo_path = %repo_path, error = %e, "Failed to open repository");
            AppError::Internal(format!("Failed to open repository: {}", e))
        })?;

        let reference = format!("refs/heads/{branch_name}");
        let head = repo.head().map_err(|_| AppError::Validation("Delivery branch is not checked out".to_string()))?;
        if !git2::Reference::is_valid_name(&reference) || head.name() != Some(reference.as_str()) || head.target().map(|id| id.to_string()).as_deref() != Some(expected_commit) {
            return Err(AppError::Validation("Delivery branch or commit changed before push".to_string()));
        }
        // Find the remote
        let mut remote = repo.find_remote(remote_name)
            .map_err(|e| {
                error!(repo_path = %repo_path, remote_name = %remote_name, error = %e, "Failed to find remote");
                AppError::Internal(format!("Failed to find remote: {}", e))
            })?;

        // Push the branch
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.push_update_reference(|_, status| match status {
            Some(message) => Err(git2::Error::from_str(message)),
            None => Ok(()),
        });
        let mut options = git2::PushOptions::new();
        options.remote_callbacks(callbacks);
        remote.push(&[&refspec], Some(&mut options))
            .map_err(|e| {
                error!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, refspec = %refspec, error = %e, "Failed to push");
                AppError::Internal(format!("Failed to push: {}", e))
            })?;

        info!(repo_path = %repo_path, remote_name = %remote_name, branch_name = %branch_name, "Successfully pushed to remote");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workflow_delivery_manifest::{file_digest, DeliveryManifest};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn fixture() -> (PathBuf, String, DeliveryManifest) {
        let root = std::env::temp_dir().join(format!("aruvi-git-delivery-{}", uuid::Uuid::new_v4()));
        let work = root.join("work"); let bare = root.join("origin.git");
        std::fs::create_dir_all(&work).unwrap();
        git2::Repository::init_bare(&bare).unwrap();
        let repo = git2::Repository::init(&work).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        repo.remote("origin", bare.to_str().unwrap()).unwrap();
        let mut config = repo.config().unwrap(); config.set_str("user.name", "Fixture").unwrap(); config.set_str("user.email", "fixture@example.invalid").unwrap();
        std::fs::write(work.join("base.txt"), "base\n").unwrap();
        let mut index = repo.index().unwrap(); index.add_path(Path::new("base.txt")).unwrap(); index.write().unwrap();
        let tree_id = index.write_tree().unwrap(); let tree = repo.find_tree(tree_id).unwrap(); let signature = repo.signature().unwrap();
        let head = repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap().to_string();
        std::fs::write(work.join("generated.txt"), "workflow output\n").unwrap();
        let hash = file_digest(&work, Path::new("generated.txt")).unwrap();
        let manifest = DeliveryManifest { repository_id: "repo".into(), base_head: Some(head.clone()), files: BTreeMap::from([("generated.txt".into(), hash)]), commit: None };
        (root, head, manifest)
    }

    #[test]
    fn workflow_commit_includes_only_recorded_files() {
        let (root, _, manifest) = fixture(); let work = root.join("work");
        std::fs::write(work.join("base.txt"), "user edit\n").unwrap();
        std::fs::write(work.join("unrelated.txt"), "user file\n").unwrap();
        let id = GitOperations::create_workflow_commit(work.to_str().unwrap(), "main", &manifest, "workflow").unwrap();
        let repo = Repository::open(&work).unwrap(); let commit = repo.find_commit(git2::Oid::from_str(&id).unwrap()).unwrap(); let tree = commit.tree().unwrap();
        assert!(tree.get_name("generated.txt").is_some());
        assert!(tree.get_name("unrelated.txt").is_none());
        let base = tree.get_name("base.txt").unwrap().to_object(&repo).unwrap().into_blob().unwrap();
        assert_eq!(base.content(), b"base\n");
        assert_eq!(std::fs::read_to_string(work.join("base.txt")).unwrap(), "user edit\n");
        assert_eq!(std::fs::read_to_string(work.join("unrelated.txt")).unwrap(), "user file\n");
        GitOperations::push_to_remote(work.to_str().unwrap(), "origin", "main", &id).unwrap();
        let remote = Repository::open_bare(root.join("origin.git")).unwrap();
        assert_eq!(remote.find_reference("refs/heads/main").unwrap().target().unwrap().to_string(), id);
        drop(base); drop(tree); drop(commit); drop(repo); drop(remote); std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delivery_rejects_staged_changes_modified_files_and_wrong_branch() {
        let (root, original, manifest) = fixture(); let work = root.join("work"); let repo = Repository::open(&work).unwrap();
        std::fs::write(work.join("unrelated.txt"), "staged\n").unwrap();
        let mut index = repo.index().unwrap(); index.add_path(Path::new("unrelated.txt")).unwrap(); index.write().unwrap();
        assert!(GitOperations::create_workflow_commit(work.to_str().unwrap(), "main", &manifest, "workflow").is_err());
        assert!(repo.index().unwrap().get_path(Path::new("unrelated.txt"), 0).is_some());
        repo.reset_default(Some(&repo.head().unwrap().peel(git2::ObjectType::Commit).unwrap()), ["unrelated.txt"]).unwrap();
        std::fs::write(work.join("generated.txt"), "changed by user\n").unwrap();
        assert!(GitOperations::create_workflow_commit(work.to_str().unwrap(), "main", &manifest, "workflow").is_err());
        assert!(GitOperations::create_workflow_commit(work.to_str().unwrap(), "other", &manifest, "workflow").is_err());
        assert_eq!(repo.head().unwrap().target().unwrap().to_string(), original);
        drop(index); drop(repo); std::fs::remove_dir_all(root).unwrap();
    }
}

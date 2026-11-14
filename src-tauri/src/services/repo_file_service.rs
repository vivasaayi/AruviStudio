use std::fs;

use crate::error::AppError;
use crate::services::repo_patch_service::apply_unified_patch_text;
use crate::services::repo_path_service::{
    canonicalize_nearest_existing_ancestor, canonicalize_repo_root,
    resolve_repository_relative_path,
};
use sha2::{Digest, Sha256};

const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;

pub fn read_repository_file(repo_root: &str, relative_path: &str) -> Result<String, AppError> {
    let root = canonicalize_repo_root(repo_root)?;
    let candidate = resolve_repository_relative_path(&root, relative_path)?;

    if !candidate.exists() {
        return Err(AppError::NotFound(format!(
            "File not found in repository: {}",
            relative_path
        )));
    }
    if !candidate.is_file() {
        return Err(AppError::Validation(format!(
            "Path is not a file: {}",
            relative_path
        )));
    }

    let canonical_file = fs::canonicalize(&candidate)?;
    if !canonical_file.starts_with(&root) {
        return Err(AppError::Validation(format!(
            "Resolved file is outside repository boundary: {}",
            relative_path
        )));
    }

    let metadata = fs::metadata(&canonical_file)?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(AppError::Validation(format!(
            "File too large to open in IDE (> {} bytes): {}",
            MAX_FILE_BYTES, relative_path
        )));
    }

    let bytes = fs::read(&canonical_file)?;
    let content = String::from_utf8(bytes).map_err(|_| {
        AppError::Validation(format!(
            "Only UTF-8 text files are supported in IDE: {}",
            relative_path
        ))
    })?;
    Ok(content)
}

pub fn write_repository_file(
    repo_root: &str,
    relative_path: &str,
    content: &str,
) -> Result<(), AppError> {
    if content.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(format!(
            "File content exceeds IDE save limit (> {} bytes): {}",
            MAX_FILE_BYTES, relative_path
        )));
    }

    let root = canonicalize_repo_root(repo_root)?;
    let candidate = resolve_repository_relative_path(&root, relative_path)?;

    let parent = candidate.parent().ok_or_else(|| {
        AppError::Validation(format!(
            "Cannot resolve parent directory for file: {}",
            relative_path
        ))
    })?;
    let canonical_parent = canonicalize_nearest_existing_ancestor(parent)?;
    if !canonical_parent.starts_with(&root) {
        return Err(AppError::Validation(format!(
            "Target path is outside repository boundary: {}",
            relative_path
        )));
    }

    if candidate.exists() {
        let canonical_target = fs::canonicalize(&candidate)?;
        if !canonical_target.starts_with(&root) {
            return Err(AppError::Validation(format!(
                "Resolved file is outside repository boundary: {}",
                relative_path
            )));
        }
    }

    fs::create_dir_all(parent)?;
    fs::write(candidate, content.as_bytes())?;
    Ok(())
}

pub fn get_repository_file_sha256(
    repo_root: &str,
    relative_path: &str,
) -> Result<String, AppError> {
    let root = canonicalize_repo_root(repo_root)?;
    let candidate = resolve_repository_relative_path(&root, relative_path)?;
    if !candidate.exists() || !candidate.is_file() {
        return Err(AppError::NotFound(format!(
            "File not found in repository: {}",
            relative_path
        )));
    }
    let canonical_file = fs::canonicalize(candidate)?;
    if !canonical_file.starts_with(&root) {
        return Err(AppError::Validation(format!(
            "Resolved file is outside repository boundary: {}",
            relative_path
        )));
    }
    let bytes = fs::read(canonical_file)?;
    Ok(sha256_hex(&bytes))
}

#[derive(serde::Serialize)]
pub struct RepositoryPatchPreview {
    pub exists: bool,
    pub base_sha256: String,
    pub result_sha256: String,
    pub result_content: String,
}

/// Identify open editor paths referring to the same file as a patch target.
pub fn matching_repository_files(repo_root: &str, relative_path: &str, open_paths: &[String]) -> Result<Vec<String>, AppError> {
    if open_paths.len() > 1024 { return Err(AppError::Validation("Too many open files to compare".into())); }
    let root = canonicalize_repo_root(repo_root)?;
    let target = resolve_repository_relative_path(&root, relative_path)?;
    let target_exists = target.try_exists()?;
    let mut matches = Vec::new();
    for path in open_paths {
        let candidate = resolve_repository_relative_path(&root, path)?;
        if candidate == target {
            matches.push(path.clone());
        } else if target_exists && candidate.try_exists()? && same_file::is_same_file(&target, &candidate)? {
            matches.push(path.clone());
        }
    }
    Ok(matches)
}

pub fn preview_repository_patch(
    repo_root: &str,
    relative_path: &str,
    patch: &str,
    base_sha256: Option<&str>,
) -> Result<RepositoryPatchPreview, AppError> {
    let (_, preview) = prepare_repository_patch(repo_root, relative_path, patch, base_sha256)?;
    Ok(preview)
}

pub fn apply_repository_patch(
    repo_root: &str,
    relative_path: &str,
    patch: &str,
    base_sha256: Option<&str>,
) -> Result<String, AppError> {
    let (candidate, preview) =
        prepare_repository_patch(repo_root, relative_path, patch, base_sha256)?;
    if let Some(parent) = candidate.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(candidate, preview.result_content.as_bytes())?;
    Ok(preview.result_sha256)
}

pub fn apply_reviewed_repository_patch(
    repo_root: &str,
    relative_path: &str,
    patch: &str,
    base_sha256: &str,
    expected_exists: bool,
) -> Result<String, AppError> {
    if base_sha256.len() != 64 || !base_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::Validation("A valid reviewed file hash is required".to_string()));
    }
    let (candidate, preview) = prepare_repository_patch(repo_root, relative_path, patch, Some(base_sha256))?;
    if preview.exists != expected_exists {
        return Err(AppError::Validation("Patch target existence changed after review".to_string()));
    }
    if let Some(parent) = candidate.parent() { fs::create_dir_all(parent)?; }
    fs::write(candidate, preview.result_content.as_bytes())?;
    Ok(preview.result_sha256)
}

fn prepare_repository_patch(
    repo_root: &str,
    relative_path: &str,
    patch: &str,
    base_sha256: Option<&str>,
) -> Result<(std::path::PathBuf, RepositoryPatchPreview), AppError> {
    let root = canonicalize_repo_root(repo_root)?;
    let candidate = resolve_repository_relative_path(&root, relative_path)?;
    let canonical_parent =
        canonicalize_nearest_existing_ancestor(candidate.parent().ok_or_else(|| {
            AppError::Validation(format!("Invalid target path for patch: {}", relative_path))
        })?)?;
    if !canonical_parent.starts_with(&root) {
        return Err(AppError::Validation(format!(
            "Target path is outside repository boundary: {}",
            relative_path
        )));
    }

    let exists = match fs::symlink_metadata(&candidate) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    let original_bytes = if exists {
        let canonical_target = fs::canonicalize(&candidate)?;
        if !canonical_target.starts_with(&root) {
            return Err(AppError::Validation(format!(
                "Resolved patch target is outside repository boundary: {}",
                relative_path
            )));
        }
        if fs::metadata(&canonical_target)?.len() > MAX_FILE_BYTES as u64 {
            return Err(AppError::Validation(
                "Patch target exceeds file size limit".to_string(),
            ));
        }
        fs::read(canonical_target)?
    } else {
        Vec::new()
    };

    let actual_hash = sha256_hex(&original_bytes);
    if let Some(expected_hash) = base_sha256 {
        if !expected_hash.eq_ignore_ascii_case(&actual_hash) {
            return Err(AppError::Validation(format!(
                "Patch precondition failed for {} (expected base_sha256={}, actual={})",
                relative_path, expected_hash, actual_hash
            )));
        }
    }

    let original_text = String::from_utf8(original_bytes).map_err(|_| {
        AppError::Validation(format!(
            "Only UTF-8 text files are supported for patch application: {}",
            relative_path
        ))
    })?;
    let patched = apply_unified_patch_text(&original_text, patch)?;

    if patched.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(
            "Patched file exceeds file size limit".to_string(),
        ));
    }
    Ok((
        candidate,
        RepositoryPatchPreview {
            exists,
            base_sha256: actual_hash,
            result_sha256: sha256_hex(patched.as_bytes()),
            result_content: patched,
        },
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        output.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{apply_repository_patch, get_repository_file_sha256, preview_repository_patch};
    use crate::error::AppError;
    use std::fs;
    use std::path::PathBuf;

    fn temp_repo_dir(test_name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aruvi_repo_service_{}_{}",
            test_name,
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp repo dir");
        dir
    }

    #[cfg(unix)]
    #[test]
    fn matches_open_files_by_identity_including_symlinks_and_hard_links() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("original.txt"), "original").unwrap();
        std::fs::hard_link(root.join("original.txt"), root.join("hard.txt")).unwrap();
        symlink("original.txt", root.join("link.txt")).unwrap();
        std::fs::write(root.join("other.txt"), "other").unwrap();
        let paths = ["original.txt", "hard.txt", "link.txt", "other.txt", "missing.txt"].map(String::from);
        let result = super::matching_repository_files(root.to_str().unwrap(), "hard.txt", &paths).unwrap();
        assert_eq!(result, vec!["original.txt", "hard.txt", "link.txt"]);
        let missing = super::matching_repository_files(root.to_str().unwrap(), "missing.txt", &paths).unwrap();
        assert_eq!(missing, vec!["missing.txt"]);
    }

    #[test]
    fn patch_preview_validates_without_writing() {
        let root = temp_repo_dir("preview");
        fs::write(root.join("file.txt"), "alpha\nbeta\n").unwrap();
        let hash = get_repository_file_sha256(&root.to_string_lossy(), "file.txt").unwrap();
        let preview = preview_repository_patch(
            &root.to_string_lossy(),
            "file.txt",
            "@@ -1,0 +2,1 @@\n+middle",
            Some(&hash),
        )
        .unwrap();
        assert!(preview.exists);
        assert_eq!(preview.base_sha256, hash);
        assert_eq!(preview.result_content, "alpha\nmiddle\nbeta\n");
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "alpha\nbeta\n"
        );
        assert!(preview_repository_patch(
            &root.to_string_lossy(),
            "file.txt",
            "@@ -1 +1 @@\n-alpha\n+changed",
            Some("wrong")
        )
        .is_err());
        let new = preview_repository_patch(
            &root.to_string_lossy(),
            "nested/new.txt",
            "@@ -0,0 +1 @@\n+new",
            None,
        )
        .unwrap();
        assert!(!new.exists);
        assert!(!root.join("nested").exists());
        assert!(preview_repository_patch(
            &root.to_string_lossy(),
            "../outside",
            "@@ -0,0 +1 @@\n+x",
            None
        )
        .is_err());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_patch_updates_existing_file_with_hash_guard() {
        let root = temp_repo_dir("existing");
        let file_path = root.join("file.txt");
        fs::write(&file_path, "alpha\nbeta\n").expect("failed to seed file");

        let before_hash =
            get_repository_file_sha256(&root.to_string_lossy(), "file.txt").expect("hash failed");
        let patch = "@@ -1,2 +1,2 @@\n-alpha\n+alpha2\n beta";
        let after_hash = apply_repository_patch(
            &root.to_string_lossy(),
            "file.txt",
            patch,
            Some(&before_hash),
        )
        .expect("patch apply failed");

        let updated = fs::read_to_string(&file_path).expect("failed to read updated file");
        assert_eq!(updated, "alpha2\nbeta\n");
        assert_ne!(before_hash, after_hash);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn malformed_patch_does_not_change_file() {
        let root = temp_repo_dir("malformed");
        let file_path = root.join("file.txt");
        fs::write(&file_path, "alpha\nbeta\n").unwrap();
        let hash = get_repository_file_sha256(&root.to_string_lossy(), "file.txt").unwrap();
        assert!(apply_repository_patch(
            &root.to_string_lossy(),
            "file.txt",
            "@@ -1,2 +1,1 @@\n-alpha\n+changed",
            Some(&hash),
        )
        .is_err());
        assert_eq!(fs::read_to_string(file_path).unwrap(), "alpha\nbeta\n");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_patch_rejects_hash_mismatch() {
        let root = temp_repo_dir("hash_mismatch");
        let file_path = root.join("file.txt");
        fs::write(&file_path, "alpha\nbeta\n").expect("failed to seed file");

        let error = apply_repository_patch(
            &root.to_string_lossy(),
            "file.txt",
            "@@ -1,1 +1,1 @@\n-alpha\n+alpha2",
            Some("deadbeef"),
        )
        .expect_err("expected precondition failure");

        match error {
            AppError::Validation(message) => {
                assert!(message.contains("Patch precondition failed"));
            }
            other => panic!("unexpected error variant: {other}"),
        }

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_patch_creates_new_file_from_hunk() {
        let root = temp_repo_dir("create_new");
        let patch = "@@ -0,0 +1,2 @@\n+line one\n+line two";

        apply_repository_patch(&root.to_string_lossy(), "new.txt", patch, None)
            .expect("patch apply failed");

        let created = fs::read_to_string(root.join("new.txt")).expect("failed to read new file");
        assert_eq!(created, "line one\nline two");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_patch_blocks_path_traversal() {
        let root = temp_repo_dir("traversal");
        let error = apply_repository_patch(
            &root.to_string_lossy(),
            "../outside.txt",
            "@@ -0,0 +1,1 @@\n+x",
            None,
        )
        .expect_err("expected traversal rejection");

        match error {
            AppError::Validation(message) => {
                assert!(message.contains("Path traversal is not allowed"));
            }
            other => panic!("unexpected error variant: {other}"),
        }

        fs::remove_dir_all(root).ok();
    }
}

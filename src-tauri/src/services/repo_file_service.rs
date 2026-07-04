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

pub fn apply_repository_patch(
    repo_root: &str,
    relative_path: &str,
    patch: &str,
    base_sha256: Option<&str>,
) -> Result<String, AppError> {
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

    let original_bytes = if candidate.exists() {
        let canonical_target = fs::canonicalize(&candidate)?;
        if !canonical_target.starts_with(&root) {
            return Err(AppError::Validation(format!(
                "Resolved patch target is outside repository boundary: {}",
                relative_path
            )));
        }
        fs::read(canonical_target)?
    } else {
        Vec::new()
    };

    if let Some(expected_hash) = base_sha256 {
        let actual_hash = sha256_hex(&original_bytes);
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

    if let Some(parent) = candidate.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&candidate, patched.as_bytes())?;
    Ok(sha256_hex(patched.as_bytes()))
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
    use super::{apply_repository_patch, get_repository_file_sha256};
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

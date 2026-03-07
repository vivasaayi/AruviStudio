use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::domain::repository::RepositoryTreeNode;
use crate::error::AppError;
use sha2::{Digest, Sha256};

const MAX_TREE_DEPTH: usize = 12;
const MAX_TREE_ENTRIES: usize = 10_000;
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
const EXCLUDED_DIRECTORIES: [&str; 7] = [
    ".git",
    "node_modules",
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
    if content.as_bytes().len() > MAX_FILE_BYTES {
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

fn canonicalize_repo_root(repo_root: &str) -> Result<PathBuf, AppError> {
    let root = PathBuf::from(repo_root);
    if !root.exists() || !root.is_dir() {
        return Err(AppError::Validation(format!(
            "Repository path is invalid or not a directory: {}",
            repo_root
        )));
    }
    Ok(fs::canonicalize(root)?)
}

fn resolve_repository_relative_path(root: &Path, relative_path: &str) -> Result<PathBuf, AppError> {
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

fn canonicalize_nearest_existing_ancestor(path: &Path) -> Result<PathBuf, AppError> {
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

fn apply_unified_patch_text(original: &str, patch: &str) -> Result<String, AppError> {
    let cleaned_patch = strip_patch_fences(patch);
    let mut patch_lines = cleaned_patch.lines().peekable();
    while let Some(line) = patch_lines.peek().copied() {
        if line.starts_with("@@") {
            break;
        }
        patch_lines.next();
    }

    let source = original
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let original_had_trailing_newline = original.ends_with('\n');
    let mut output: Vec<String> = Vec::new();
    let mut source_cursor: usize = 0;
    let mut encountered_hunk = false;

    while let Some(line) = patch_lines.next() {
        if !line.starts_with("@@") {
            continue;
        }
        encountered_hunk = true;
        let (old_start, _old_count) = parse_hunk_header(line)?;
        let target_cursor = old_start.saturating_sub(1);
        if target_cursor < source_cursor {
            return Err(AppError::Validation(
                "Patch hunk order is invalid or overlapping".to_string(),
            ));
        }

        output.extend_from_slice(&source[source_cursor..target_cursor.min(source.len())]);
        source_cursor = target_cursor.min(source.len());

        while let Some(next_line) = patch_lines.peek().copied() {
            if next_line.starts_with("@@") {
                break;
            }
            patch_lines.next();
            if next_line == r"\ No newline at end of file" {
                continue;
            }
            let mut chars = next_line.chars();
            let marker = chars
                .next()
                .ok_or_else(|| AppError::Validation("Malformed patch line".to_string()))?;
            let value = chars.as_str().to_string();
            match marker {
                ' ' => {
                    let source_line = source.get(source_cursor).ok_or_else(|| {
                        AppError::Validation("Patch context line exceeds source length".to_string())
                    })?;
                    if source_line != &value {
                        return Err(AppError::Validation(format!(
                            "Patch context mismatch. Expected '{}', found '{}'",
                            value, source_line
                        )));
                    }
                    output.push(source_line.clone());
                    source_cursor += 1;
                }
                '-' => {
                    let source_line = source.get(source_cursor).ok_or_else(|| {
                        AppError::Validation("Patch removal line exceeds source length".to_string())
                    })?;
                    if source_line != &value {
                        return Err(AppError::Validation(format!(
                            "Patch removal mismatch. Expected '{}', found '{}'",
                            value, source_line
                        )));
                    }
                    source_cursor += 1;
                }
                '+' => {
                    output.push(value);
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported patch marker '{}'",
                        marker
                    )));
                }
            }
        }
    }

    if !encountered_hunk {
        return Err(AppError::Validation(
            "Patch did not contain any unified diff hunks".to_string(),
        ));
    }

    output.extend_from_slice(&source[source_cursor..]);
    let mut rebuilt = output.join("\n");
    if original_had_trailing_newline && !rebuilt.ends_with('\n') {
        rebuilt.push('\n');
    }
    Ok(rebuilt)
}

fn strip_patch_fences(patch: &str) -> String {
    let trimmed = patch.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or_default();
    if !first.starts_with("```") {
        return trimmed.to_string();
    }
    let mut body: Vec<String> = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        body.push(line.to_string());
    }
    body.join("\n")
}

fn parse_hunk_header(header: &str) -> Result<(usize, usize), AppError> {
    // @@ -old_start,old_count +new_start,new_count @@
    let remainder = header
        .strip_prefix("@@")
        .ok_or_else(|| AppError::Validation("Malformed hunk header".to_string()))?;
    let closing = remainder
        .find("@@")
        .ok_or_else(|| AppError::Validation("Malformed hunk header".to_string()))?;
    let inner = remainder[..closing].trim();
    let mut parts = inner.split_whitespace();
    let old_part = parts
        .next()
        .ok_or_else(|| AppError::Validation("Malformed old range in hunk header".to_string()))?;
    if !old_part.starts_with('-') {
        return Err(AppError::Validation(
            "Malformed old range in hunk header".to_string(),
        ));
    }
    parse_hunk_range(&old_part[1..])
}

fn parse_hunk_range(range: &str) -> Result<(usize, usize), AppError> {
    let mut parts = range.split(',');
    let start = parts
        .next()
        .ok_or_else(|| AppError::Validation("Missing hunk start".to_string()))?
        .parse::<usize>()
        .map_err(|_| AppError::Validation("Invalid hunk start".to_string()))?;
    let count = parts
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| AppError::Validation("Invalid hunk count".to_string()))?
        .unwrap_or(1);
    Ok((start, count))
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

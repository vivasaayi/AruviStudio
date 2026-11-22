use crate::error::AppError;
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use std::{fs::File, io::Read, path::Path};

#[cfg(unix)]
pub(crate) fn open_context_file(root: &Option<File>, _canonical_root: &Path, relative: &Path) -> std::io::Result<File> {
    use std::os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::MetadataExt}};
    use std::path::Component;
    let components: Vec<_> = relative.components().collect();
    let mut directory = root.as_ref().ok_or(std::io::ErrorKind::InvalidInput)?.try_clone()?;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else { return Err(std::io::ErrorKind::InvalidInput.into()); };
        let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        let last = index + 1 == components.len();
        let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | if last { libc::O_NONBLOCK } else { libc::O_DIRECTORY };
        // SAFETY: directory and name remain valid during openat; the returned fd is owned once.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 { return Err(std::io::Error::last_os_error()); }
        let opened = unsafe { File::from_raw_fd(fd) };
        if last {
            let metadata = opened.metadata()?;
            if !metadata.is_file() || metadata.nlink() != 1 { return Err(std::io::ErrorKind::PermissionDenied.into()); }
            return Ok(opened);
        }
        directory = opened;
    }
    Err(std::io::ErrorKind::InvalidInput.into())
}

#[cfg(not(unix))]
pub(crate) fn open_context_file(_root: &Option<File>, canonical_root: &Path, relative: &Path) -> std::io::Result<File> {
    let path = canonical_root.join(relative);
    if !std::fs::symlink_metadata(&path)?.file_type().is_file() || !std::fs::canonicalize(&path)?.starts_with(canonical_root) {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    File::open(path)
}

pub(crate) fn build_repository_context(
    repo_path: &str,
    boundaries: &AgentExecutionBoundaries,
    context_budget_chars: usize,
) -> Result<String, AppError> {
    let repo_root = Path::new(repo_path);
    if !repo_root.exists() || !repo_root.is_dir() {
        return Ok(String::new());
    }
    let canonical_root = std::fs::canonicalize(repo_root)?;
    #[cfg(unix)]
    let root_handle = Some(File::open(&canonical_root)?);
    #[cfg(not(unix))]
    let root_handle: Option<File> = None;

    let max_repo_files_scanned = boundaries
        .max_repo_files_scanned
        .unwrap_or(600)
        .clamp(50, 5_000);
    let mut stack = vec![repo_root.to_path_buf()];
    let mut files: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    while let Some(dir) = stack.pop() {
        if !std::fs::canonicalize(&dir).is_ok_and(|path| path.starts_with(&canonical_root)) {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = match entry.file_type() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if file_type.is_symlink() { continue; }
            if file_type.is_dir() {
                if agent_execution_limits::ignored_repository_dir(file_name.as_str()) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if scanned >= max_repo_files_scanned {
                break;
            }
            scanned += 1;
            let rel = match path.strip_prefix(repo_root) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if !agent_execution_limits::is_repo_relative_path_allowed(&rel, boundaries) {
                continue;
            }
            if !agent_execution_limits::is_text_source_file(&rel) {
                continue;
            }
            if open_context_file(&root_handle, &canonical_root, Path::new(&rel)).is_err() {
                continue;
            }
            files.push(rel);
        }
        if scanned >= max_repo_files_scanned {
            break;
        }
    }

    files.sort();
    if files.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();
    output.push_str("Repository file manifest:\n");
    for file in files.iter().take(80) {
        output.push_str("- ");
        output.push_str(file);
        output.push('\n');
    }

    let max_files = agent_execution_limits::max_files_per_run(boundaries);
    let max_file_chars = agent_execution_limits::max_repo_snippet_chars(boundaries);
    let mut used_chars = output.len();
    let mut snippets_added = 0usize;
    let snippet_budget = context_budget_chars
        .saturating_sub(used_chars)
        .min((context_budget_chars / 4).max(2_500))
        .min(6_000);

    let prioritized_files = agent_execution_limits::prioritize_repository_files(&files);
    for rel in &prioritized_files {
        if snippets_added >= max_files || used_chars >= snippet_budget {
            break;
        }
        let file = match open_context_file(&root_handle, &canonical_root, Path::new(rel)) {
            Ok(file) => file,
            Err(_) => continue,
        };
        let mut bytes = Vec::new();
        if file.take(300_001).read_to_end(&mut bytes).is_err() || bytes.len() > 300_000 { continue; }
        let content = match String::from_utf8(bytes) { Ok(value) => value, Err(_) => continue };
        if content.trim().is_empty() {
            continue;
        }
        let remaining = snippet_budget.saturating_sub(used_chars);
        if remaining < 100 {
            break;
        }
        let take_chars = remaining.min(max_file_chars);
        let snippet = content.chars().take(take_chars).collect::<String>();
        output.push_str("\nFile Snippet:\nPath: ");
        output.push_str(rel);
        output.push('\n');
        output.push_str(&snippet);
        if snippet.len() < content.len() {
            output.push_str("\n...[truncated]...\n");
        } else {
            output.push('\n');
        }
        used_chars = output.len();
        snippets_added += 1;
    }

    Ok(output)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn repository_context_skips_external_symlink_files_and_directories() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&repo).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(repo.join("main.rs"), "fn safe() {}\n").unwrap();
        std::fs::write(outside.join("private.rs"), "EXTERNAL_PRIVATE_MARKER\n").unwrap();
        std::os::unix::fs::symlink(outside.join("private.rs"), repo.join("linked.rs")).unwrap();
        std::os::unix::fs::symlink(&outside, repo.join("linked-dir")).unwrap();
        std::fs::hard_link(outside.join("private.rs"), repo.join("hardlinked.rs")).unwrap();
        let context = build_repository_context(repo.to_str().unwrap(), &AgentExecutionBoundaries::default(), 12000).unwrap();
        assert!(context.contains("main.rs"));
        assert!(!context.contains("linked.rs"));
        assert!(!context.contains("linked-dir"));
        assert!(!context.contains("hardlinked.rs"));
        assert!(!context.contains("EXTERNAL_PRIVATE_MARKER"));
    }

    #[test]
    fn descriptor_read_rejects_replaced_directory_and_file() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(repo.join("source")).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("secret.rs"), "secret").unwrap();
        let root = Some(File::open(&repo).unwrap());
        std::fs::remove_dir(repo.join("source")).unwrap();
        std::os::unix::fs::symlink(&outside, repo.join("source")).unwrap();
        assert!(open_context_file(&root, &repo, Path::new("source/secret.rs")).is_err());
        std::os::unix::fs::symlink(outside.join("secret.rs"), repo.join("secret.rs")).unwrap();
        assert!(open_context_file(&root, &repo, Path::new("secret.rs")).is_err());
    }
}

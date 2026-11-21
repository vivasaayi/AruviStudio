//! Shared, bounded text reads for desktop, browser and MCP artifacts.
use crate::error::AppError;
use std::path::Path;
use tokio::io::AsyncReadExt;

pub(crate) const TEXT_LIMIT: u64 = 2 * 1024 * 1024;
pub(crate) const TEST_RESULT_LIMIT: u64 = 256 * 1024;

#[cfg(unix)]
fn open_beneath(base: &Path, relative: &Path) -> Result<std::fs::File, AppError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;

    let unavailable = || AppError::NotFound("Artifact not found".into());
    let base_name = CString::new(base.as_os_str().as_bytes()).map_err(|_| unavailable())?;
    // Anchor each lookup to the directory descriptor. A symlink swapped into
    // any component after canonicalization must not redirect the open.
    let raw = unsafe { libc::open(base_name.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW) };
    if raw < 0 { return Err(unavailable()); }
    let mut directory = unsafe { OwnedFd::from_raw_fd(raw) };
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        let Component::Normal(name) = part else { return Err(unavailable()); };
        let name = CString::new(name.as_bytes()).map_err(|_| unavailable())?;
        let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK
            | if parts.peek().is_some() { libc::O_DIRECTORY } else { 0 };
        let raw = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if raw < 0 { return Err(unavailable()); }
        directory = unsafe { OwnedFd::from_raw_fd(raw) };
    }
    Ok(directory.into())
}

pub(crate) async fn read_text(base: &Path, path: &Path, limit: u64) -> Result<String, AppError> {
    let unavailable = || AppError::NotFound("Artifact not found".into());
    let base = tokio::fs::canonicalize(base).await.map_err(|_| unavailable())?;
    let resolved = tokio::fs::canonicalize(path).await.map_err(|_| unavailable())?;
    let relative = resolved.strip_prefix(&base).map_err(|_| unavailable())?;
    #[cfg(unix)]
    let file = tokio::fs::File::from_std(open_beneath(&base, relative)?);
    #[cfg(not(unix))]
    let file = tokio::fs::File::open(resolved).await.map_err(|_| unavailable())?;
    match file.metadata().await {
        Ok(metadata) if metadata.is_file() && metadata.len() <= limit => (),
        _ => return Err(AppError::Validation("Artifact is unavailable or exceeds the text limit".into())),
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > limit {
        return Err(AppError::Validation("Artifact is unavailable or exceeds the text limit".into()));
    }
    String::from_utf8(bytes).map_err(|_| AppError::Validation("Artifact is not UTF-8 text".into()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn swapped_parent_symlink_cannot_redirect_artifact_open() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("artifacts");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(base.join("run")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(base.join("run/output"), "safe").unwrap();
        std::fs::write(outside.join("output"), "secret").unwrap();
        let resolved = std::fs::canonicalize(base.join("run/output")).unwrap();
        let canonical_base = std::fs::canonicalize(&base).unwrap();
        let relative = resolved.strip_prefix(&canonical_base).unwrap();
        std::fs::rename(base.join("run"), base.join("original-run")).unwrap();
        std::os::unix::fs::symlink(&outside, base.join("run")).unwrap();
        assert!(open_beneath(&canonical_base, relative).is_err());
    }
}

//! Shared, bounded text reads for desktop, browser and MCP artifacts.
use crate::error::AppError;
use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub(crate) const TEXT_LIMIT: u64 = 2 * 1024 * 1024;
pub(crate) const TEST_RESULT_LIMIT: u64 = 256 * 1024;
pub(crate) const PAGE_LIMIT: u64 = 64 * 1024;

#[derive(serde::Serialize)]
pub struct ArtifactPage {
    pub(crate) content: String,
    pub(crate) offset: u64,
    pub(crate) next_offset: Option<u64>,
    pub(crate) total_bytes: u64,
    pub(crate) version: String,
}

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

async fn open_confined(base: &Path, path: &Path) -> Result<tokio::fs::File, AppError> {
    let unavailable = || AppError::NotFound("Artifact not found".into());
    let base = tokio::fs::canonicalize(base).await.map_err(|_| unavailable())?;
    let resolved = tokio::fs::canonicalize(path).await.map_err(|_| unavailable())?;
    let relative = resolved.strip_prefix(&base).map_err(|_| unavailable())?;
    #[cfg(unix)]
    let file = tokio::fs::File::from_std(open_beneath(&base, relative)?);
    #[cfg(not(unix))]
    let file = tokio::fs::File::open(resolved).await.map_err(|_| unavailable())?;
    Ok(file)
}

pub(crate) async fn read_text(base: &Path, path: &Path, limit: u64) -> Result<String, AppError> {
    let file = open_confined(base, path).await?;
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

fn version(metadata: &std::fs::Metadata) -> Result<String, AppError> {
    let modified = metadata.modified().map_err(|_| AppError::Validation("Artifact version is unavailable".into()))?
        .duration_since(std::time::UNIX_EPOCH).map_err(|_| AppError::Validation("Artifact version is unavailable".into()))?;
    let mut hash = Sha256::new();
    hash.update(metadata.len().to_le_bytes());
    hash.update(modified.as_secs().to_le_bytes());
    hash.update(modified.subsec_nanos().to_le_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hash.update(metadata.dev().to_le_bytes());
        hash.update(metadata.ino().to_le_bytes());
        hash.update(metadata.ctime().to_le_bytes());
        hash.update(metadata.ctime_nsec().to_le_bytes());
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) async fn read_page(base: &Path, path: &Path, offset: u64, expected: Option<&str>) -> Result<ArtifactPage, AppError> {
    if offset > 0 && expected.is_none() {
        return Err(AppError::Validation("A version is required after the first artifact page".into()));
    }
    let mut file = open_confined(base, path).await?;
    let metadata = file.metadata().await?;
    if !metadata.is_file() { return Err(AppError::NotFound("Artifact not found".into())); }
    let current = version(&metadata)?;
    if expected.is_some_and(|expected| expected != current) {
        return Err(AppError::Validation("Artifact changed; restart reading from the first page".into()));
    }
    if offset > metadata.len() { return Err(AppError::Validation("Artifact offset is beyond the file".into())); }
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    let mut bytes = Vec::new();
    (&mut file).take(PAGE_LIMIT).read_to_end(&mut bytes).await?;
    if version(&file.metadata().await?)? != current {
        return Err(AppError::Validation("Artifact changed while reading; restart from the first page".into()));
    }
    let end = match std::str::from_utf8(&bytes) {
        Ok(_) => bytes.len(),
        Err(error) if error.error_len().is_none() && offset + (bytes.len() as u64) < metadata.len() && error.valid_up_to() > 0 => error.valid_up_to(),
        Err(_) => return Err(AppError::Validation("Artifact is not UTF-8 text or offset is not a character boundary".into())),
    };
    let content = String::from_utf8(bytes[..end].to_vec()).expect("UTF-8 validated above");
    let next = offset + end as u64;
    Ok(ArtifactPage { content, offset, next_offset: (next < metadata.len()).then_some(next), total_bytes: metadata.len(), version: current })
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

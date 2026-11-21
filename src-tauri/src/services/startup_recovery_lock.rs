//! Coordinate background-job recovery between Studio processes sharing a SQLite file.
use crate::error::AppError;
use sqlx::SqlitePool;

#[cfg(unix)]
pub(crate) async fn acquire(db: &SqlitePool) -> Result<(Option<std::fs::File>, bool), AppError> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    let databases: Vec<(i64, String, String)> =
        sqlx::query_as("PRAGMA database_list").fetch_all(db).await?;
    let path = databases
        .into_iter()
        .find(|(_, name, _)| name == "main")
        .map(|(_, _, path)| path)
        .unwrap_or_default();
    if path.is_empty() {
        return Ok((None, true));
    }
    let database = std::fs::canonicalize(path)?;
    let parent = database
        .parent()
        .ok_or_else(|| AppError::Internal("Database has no parent directory".into()))?;
    let filename = database
        .file_name()
        .ok_or_else(|| AppError::Internal("Database has no filename".into()))?;
    let mut name = std::ffi::OsString::from(".");
    name.push(filename);
    name.push(".startup-recovery-locks");
    let directory = parent.join(name);
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = std::fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata.mode() & 0o077 != 0 {
        return Err(AppError::Internal(
            "Startup recovery lock directory must be private and must not be a symlink".into(),
        ));
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(directory.join("lifecycle"))?;
    if !file.metadata()?.is_file() {
        return Err(AppError::Internal(
            "Startup recovery lock must be a regular file".into(),
        ));
    }
    loop {
        // The file is held by AppState for the lifetime of the process.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok((Some(file), true));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if error.kind() != std::io::ErrorKind::WouldBlock {
            return Err(error.into());
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } == 0 {
            return Ok((Some(file), false));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::WouldBlock
            && error.kind() != std::io::ErrorKind::Interrupted
        {
            return Err(error.into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

#[cfg(unix)]
pub(crate) fn finish_recovery(file: &Option<std::fs::File>) -> Result<(), AppError> {
    use std::os::fd::AsRawFd;
    if let Some(file) = file {
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH) } == 0 {
                break;
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }
    Ok(())
}

// Non-Unix platforms keep the legacy startup recovery behavior.
#[cfg(not(unix))]
pub(crate) async fn acquire(_db: &SqlitePool) -> Result<(Option<std::fs::File>, bool), AppError> {
    Ok((None, true))
}
#[cfg(not(unix))]
pub(crate) fn finish_recovery(_file: &Option<std::fs::File>) -> Result<(), AppError> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn shared_database_recovers_only_after_all_guards_close() {
        let dir = tempfile::tempdir().unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(dir.path().join("test.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let (first, leader) = acquire(&db).await.unwrap();
        assert!(leader);
        finish_recovery(&first).unwrap();
        let (second, leader) = acquire(&db).await.unwrap();
        assert!(!leader);
        drop(first);
        let (third, leader) = acquire(&db).await.unwrap();
        assert!(!leader);
        drop(second);
        drop(third);
        let (last, leader) = acquire(&db).await.unwrap();
        assert!(leader);
        finish_recovery(&last).unwrap();
    }

    #[tokio::test]
    async fn contender_takes_over_if_initial_recovery_owner_disappears() {
        let dir = tempfile::tempdir().unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(dir.path().join("test.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let (first, leader) = acquire(&db).await.unwrap();
        assert!(leader);
        let other = db.clone();
        let mut contender = tokio::spawn(async move { acquire(&other).await });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut contender)
                .await
                .is_err()
        );
        drop(first);
        let (guard, leader) = tokio::time::timeout(std::time::Duration::from_secs(2), contender)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(leader);
        finish_recovery(&guard).unwrap();
    }

    #[tokio::test]
    async fn rejects_symlinked_lock_directory() {
        let dir = tempfile::tempdir().unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(dir.path().join("test.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join(".test.db.startup-recovery-locks"))
            .unwrap();
        assert!(acquire(&db).await.is_err());
    }

    #[tokio::test]
    async fn rejects_symlinked_lock_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(dir.path().join("test.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let locks = dir.path().join(".test.db.startup-recovery-locks");
        std::fs::create_dir(&locks).unwrap();
        std::fs::set_permissions(&locks, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(dir.path().join("test.db"), locks.join("lifecycle")).unwrap();
        assert!(acquire(&db).await.is_err());
    }
}

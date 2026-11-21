use crate::error::AppError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

static OVERRIDE_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DatabaseOverrideExpected {
    pub path: serde_json::Value,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MigrationStatus {
    pub version: i64,
    pub description: String,
    pub success: bool,
    pub installed_on: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealth {
    pub applied_migrations: usize,
    pub latest_version: Option<i64>,
    pub migrations: Vec<MigrationStatus>,
}

pub async fn get_database_health(state: &AppState) -> Result<DatabaseHealth, AppError> {
    let migrations = sqlx::query_as::<_, MigrationStatus>(
        "SELECT version, description, success, datetime(installed_on, 'unixepoch') AS installed_on
         FROM _sqlx_migrations
         ORDER BY version ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let latest_version = migrations.last().map(|migration| migration.version);

    Ok(DatabaseHealth {
        applied_migrations: migrations.len(),
        latest_version,
        migrations,
    })
}

pub async fn get_active_database_path(state: &AppState) -> Result<String, AppError> {
    let rows = sqlx::query("PRAGMA database_list")
        .fetch_all(&state.db)
        .await?;
    let main_path = rows
        .iter()
        .find(|row| row.get::<String, _>("name") == "main")
        .map(|row| row.get::<String, _>("file"))
        .ok_or_else(|| {
            AppError::Internal("Unable to resolve active SQLite database path".to_string())
        })?;
    Ok(main_path)
}

pub async fn get_database_path_override(
    state: &AppState,
) -> Result<Option<String>, AppError> {
    read_override(&state.app_data_dir)
}

fn read_override(dir: &Path) -> Result<Option<String>, AppError> {
    let override_path = dir.join("db_override_path.txt");
    let value = match std::fs::read_to_string(&override_path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(value)
}

pub async fn set_database_path_override(
    state: &AppState,
    db_path: String,
) -> Result<(), AppError> {
    update_database_path_override(state, Some(db_path), None).await?;
    Ok(())
}

pub async fn clear_database_path_override(state: &AppState) -> Result<(), AppError> {
    update_database_path_override(state, None, None).await?;
    Ok(())
}

/// Returns false when an optional expected snapshot no longer matches.
pub(crate) async fn update_database_path_override(
    state: &AppState,
    path: Option<String>,
    expected: Option<DatabaseOverrideExpected>,
) -> Result<bool, AppError> {
    let path = match path {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() { return Err(AppError::Validation("Database path cannot be empty".into())); }
            if !Path::new(trimmed).is_absolute() { return Err(AppError::Validation("Database path must be an absolute path".into())); }
            Some(trimmed.to_owned())
        }
        None => None,
    };
    let expected = match expected {
        None => None,
        Some(snapshot) => Some(match snapshot.path {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) => Some(value),
            _ => return Err(AppError::Validation("Expected database override must be a string or null".into())),
        }),
    };
    let dir = state.app_data_dir.clone();
    tokio::task::spawn_blocking(move || write_override(&dir, path.as_deref(), expected.as_ref()))
        .await.map_err(|_| AppError::Internal("Database override update task failed".into()))?
}

fn write_override(dir: &Path, path: Option<&str>, expected: Option<&Option<String>>) -> Result<bool, AppError> {
    let _guard = OVERRIDE_WRITE_LOCK.lock().map_err(|_| AppError::Internal("Database override lock failed".into()))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    let _file_lock = {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let file = options.open(dir.join(".db_override_path.lock"))?;
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 { break; }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted { return Err(error.into()); }
        }
        file
    };
    if let Some(expected) = expected {
        if &read_override(dir)? != expected { return Ok(false); }
    }
    let target = dir.join("db_override_path.txt");
    match path {
        Some(value) => {
            let temporary = dir.join(format!(".db-override-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| -> Result<(), std::io::Error> {
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
                let mut file = options.open(&temporary)?;
                file.write_all(value.as_bytes())?;
                file.sync_all()?;
                std::fs::rename(&temporary, &target)?;
                Ok(())
            })();
            if result.is_err() { let _ = std::fs::remove_file(&temporary); }
            result?;
        }
        None => match std::fs::remove_file(target) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.into()),
        },
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_updates_are_atomic_and_leave_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_override(dir.path(), Some("/tmp/first.db"), Some(&None)).unwrap());
        assert!(!write_override(dir.path(), Some("/tmp/stale.db"), Some(&None)).unwrap());
        assert_eq!(read_override(dir.path()).unwrap().as_deref(), Some("/tmp/first.db"));
        assert!(write_override(dir.path(), None, Some(&Some("/tmp/first.db".into()))).unwrap());
        assert_eq!(read_override(dir.path()).unwrap(), None);
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().ends_with(".tmp")));
    }

    #[test]
    fn readers_observe_only_complete_override_values() {
        let dir = tempfile::tempdir().unwrap();
        let first = format!("/tmp/{}", "a".repeat(8192));
        let second = format!("/tmp/{}", "b".repeat(8192));
        write_override(dir.path(), Some(&first), None).unwrap();
        let path = dir.path().to_owned();
        let next = second.clone();
        let writer = std::thread::spawn(move || {
            for _ in 0..20 { write_override(&path, Some(&next), None).unwrap(); }
        });
        for _ in 0..50 {
            let current = read_override(dir.path()).unwrap().unwrap();
            assert!(current == first || current == second);
        }
        writer.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn database_override_writer_child() {
        let Ok(dir) = std::env::var("ARUVI_TEST_OVERRIDE_DIR") else { return; };
        let name = std::env::var("ARUVI_TEST_OVERRIDE_WRITER").unwrap();
        let dir = std::path::PathBuf::from(dir);
        std::fs::write(dir.join(format!("{name}.ready")), "").unwrap();
        let start = std::time::Instant::now();
        while !dir.join("start").exists() {
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let success = write_override(&dir, Some(&format!("/tmp/{name}.db")), Some(&None)).unwrap();
        std::fs::write(dir.join(format!("{name}.result")), if success { "won" } else { "conflict" }).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn guarded_updates_coordinate_across_processes() {
        use std::process::{Command, Stdio};
        let dir = tempfile::tempdir().unwrap();
        let spawn = |name: &str| Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "services::settings_database_service::tests::database_override_writer_child"])
            .env("ARUVI_TEST_OVERRIDE_DIR", dir.path()).env("ARUVI_TEST_OVERRIDE_WRITER", name)
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let mut first = spawn("first");
        let mut second = spawn("second");
        let start = std::time::Instant::now();
        while !(dir.path().join("first.ready").exists() && dir.path().join("second.ready").exists()) {
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::fs::write(dir.path().join("start"), "").unwrap();
        assert!(first.wait().unwrap().success());
        assert!(second.wait().unwrap().success());
        let results = [std::fs::read_to_string(dir.path().join("first.result")).unwrap(),
                       std::fs::read_to_string(dir.path().join("second.result")).unwrap()];
        assert_eq!(results.iter().filter(|result| *result == "won").count(), 1);
        assert_eq!(results.iter().filter(|result| *result == "conflict").count(), 1);
        assert!(matches!(read_override(dir.path()).unwrap().as_deref(), Some("/tmp/first.db" | "/tmp/second.db")));
    }
}


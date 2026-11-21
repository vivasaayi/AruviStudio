use crate::error::AppError;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

pub(crate) const TEST_COMMAND_KEY: &str = "workflow.test_argv";
pub(crate) fn product_command_key(product_id: &str) -> String {
    format!("{TEST_COMMAND_KEY}.product.{product_id}")
}

pub(crate) async fn resolve_command(pool: &sqlx::SqlitePool, product_id: &str) -> Result<(String, Vec<String>), AppError> {
    use crate::persistence::settings_repo::get_setting;
    let product = get_setting(pool, &product_command_key(product_id)).await?;
    let default = get_setting(pool, TEST_COMMAND_KEY).await?;
    resolve_values(product.as_deref(), default.as_deref())
}

fn resolve_values(product: Option<&str>, default: Option<&str>) -> Result<(String, Vec<String>), AppError> {
    if let Some(value) = product {
        if value == "[]" { return Ok(("disabled".into(), vec![])); }
        return Ok(("custom".into(), parse_command(value)?));
    }
    match default {
        Some(value) if !value.trim().is_empty() => Ok(("inherit".into(), parse_command(value)?)),
        _ => Ok(("inherit".into(), vec![])),
    }
}

/// Compare the effective command and write the product override under one SQLite
/// write lock. The optional snapshot preserves compatibility with older clients.
pub(crate) async fn set_product_command(
    pool: &sqlx::SqlitePool, product_id: &str, mode: &str, argv: &[String],
    expected: Option<(&str, &[String])>,
) -> Result<Option<(String, Vec<String>)>, AppError> {
    let new_value = match mode {
        "inherit" => None,
        "disabled" => Some("[]".to_owned()),
        "custom" => {
            let value = serde_json::to_string(argv)?;
            parse_command(&value)?;
            Some(value)
        }
        _ => return Err(AppError::Validation("Invalid test command mode".into())),
    };
    let key = product_command_key(product_id);
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let product: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
        .bind(&key).fetch_optional(&mut *tx).await?;
    let default: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
        .bind(TEST_COMMAND_KEY).fetch_optional(&mut *tx).await?;
    if let Some((expected_mode, expected_argv)) = expected {
        let current = resolve_values(product.as_deref(), default.as_deref())?;
        if current.0 != expected_mode || current.1 != expected_argv {
            return Ok(None);
        }
    }
    if let Some(value) = &new_value {
        sqlx::query("INSERT INTO settings (key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')")
            .bind(&key).bind(value).execute(&mut *tx).await?;
    } else {
        sqlx::query("DELETE FROM settings WHERE key=?").bind(&key).execute(&mut *tx).await?;
    }
    let result = resolve_values(new_value.as_deref(), default.as_deref())?;
    tx.commit().await?;
    Ok(Some(result))
}

const OUTPUT_LIMIT: usize = 16 * 1024;
const TEST_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) fn parse_command(value: &str) -> Result<Vec<String>, AppError> {
    let args: Vec<String> = serde_json::from_str(value)
        .map_err(|_| AppError::Validation("Test command must be a JSON array of strings".into()))?;
    if args.is_empty() || args.len() > 32 || args[0].trim().is_empty() || args.iter().any(|arg| arg.len() > 4096 || arg.contains('\0')) {
        return Err(AppError::Validation("Test command must contain an executable and at most 32 valid arguments".into()));
    }
    Ok(args)
}

async fn collect_output(mut reader: impl AsyncRead + Unpin) -> std::io::Result<String> {
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 { break; }
        let remaining = OUTPUT_LIMIT.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    let mut text = String::from_utf8_lossy(&kept).into_owned();
    if truncated { text.push_str("\n[output truncated]\n"); }
    Ok(text)
}

pub(crate) struct TestResult {
    pub success: bool,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

// Retain cleanup when the future is cancelled as well as when it completes.
// This only covers children that remain in the command's process group.
#[cfg(unix)]
struct ProcessGroupGuard(Option<i32>);

#[cfg(unix)]
impl ProcessGroupGuard {
    fn terminate(&mut self) {
        if let Some(id) = self.0.take() {
            unsafe { libc::kill(-id, libc::SIGKILL); }
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) { self.terminate(); }
}

// Keep the exited leader waitable until its group has been signalled. Reaping
// first would allow the numeric PID/group ID to be reused by another process.
#[cfg(any(target_os = "linux", target_os = "macos"))]
async fn wait_for_group_leader(id: i32) -> std::io::Result<()> {
    loop {
        let exited = {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let result = unsafe {
                libc::waitid(libc::P_PID, id as libc::id_t, &mut info,
                             libc::WEXITED | libc::WNOWAIT | libc::WNOHANG)
            };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted { continue; }
                return Err(error);
            }
            unsafe { info.si_pid() != 0 }
        };
        if exited { return Ok(()); }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub(crate) async fn run_command(cwd: &Path, args: &[String]) -> Result<TestResult, AppError> {
    run_command_with_timeout(cwd, args, TEST_TIMEOUT).await
}

async fn run_command_with_timeout(cwd: &Path, args: &[String], timeout: Duration) -> Result<TestResult, AppError> {
    let mut command = tokio::process::Command::new(&args[0]);
    command.args(&args[1..]).current_dir(cwd).env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()))
        .env("HOME", std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into()))
        .env("CI", "true").stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    let mut child = command.spawn().map_err(|error| AppError::Internal(format!("Unable to start configured test command: {error}")))?;
    #[cfg(unix)]
    let mut process_group = ProcessGroupGuard(child.id().and_then(|id| i32::try_from(id).ok()));
    let stdout = child.stdout.take().ok_or_else(|| AppError::Internal("Unable to capture test stdout".into()))?;
    let stderr = child.stderr.take().ok_or_else(|| AppError::Internal("Unable to capture test stderr".into()))?;
    let mut stdout_task = tokio::spawn(collect_output(stdout));
    let mut stderr_task = tokio::spawn(collect_output(stderr));
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let wait = async {
        let id = process_group.0.ok_or_else(|| std::io::Error::other("Missing command process ID"))?;
        wait_for_group_leader(id).await
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let wait = child.wait();
    let (status, timed_out) = match tokio::time::timeout(timeout, wait).await {
        Ok(result) => {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                if let Err(error) = result {
                    // If the leader is no longer waitable, the group ID may
                    // have been reused; do not signal an unrelated group.
                    process_group.0 = None;
                    return Err(error.into());
                }
                process_group.terminate();
                (Some(child.wait().await?), false)
            }
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            {
                #[cfg(unix)]
                process_group.terminate();
                (Some(result?), false)
            }
        }
        Err(_) => {
            #[cfg(unix)]
            process_group.terminate();
            let _ = child.kill().await;
            (None, true)
        }
    };
    async fn finish(task: &mut tokio::task::JoinHandle<std::io::Result<String>>) -> String {
        match tokio::time::timeout(Duration::from_secs(2), &mut *task).await {
            Ok(Ok(Ok(text))) => text,
            _ => { task.abort(); "[output unavailable or still open]".into() }
        }
    }
    let stdout = finish(&mut stdout_task).await;
    let stderr = finish(&mut stderr_task).await;
    Ok(TestResult { success: status.as_ref().is_some_and(|value| value.success()), timed_out,
        exit_code: status.and_then(|value| value.code()), stdout, stderr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_requires_an_argument_array() {
        assert!(parse_command("\"echo hi\"").is_err());
        assert!(parse_command("[]").is_err());
        assert_eq!(parse_command(r#"["echo","hello"]"#).unwrap(), vec!["echo", "hello"]);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn command_reports_exit_and_does_not_inherit_application_secrets() {
        let args = vec!["/bin/sh".into(), "-c".into(), "printf '%s' \"${ARUVI_MOBILE_API_TOKEN-unset}\"; exit 7".into()];
        let result = run_command(std::env::temp_dir().as_path(), &args).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(7));
        assert_eq!(result.stdout, "unset");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn command_times_out() {
        let args = vec!["/bin/sh".into(), "-c".into(), "sleep 10".into()];
        let result = run_command_with_timeout(std::env::temp_dir().as_path(), &args, Duration::from_millis(100)).await.unwrap();
        assert!(result.timed_out);
        assert!(!result.success);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn timed_out_command_does_not_leave_background_children() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("child-ran");
        let args = vec!["/bin/sh".into(), "-c".into(),
            "(sleep 1; printf leaked > \"$1\") & wait".into(),
            "workflow-test".into(), marker.to_string_lossy().into_owned()];
        let result = run_command_with_timeout(temp.path(), &args, Duration::from_millis(100)).await.unwrap();
        assert!(result.timed_out);
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(!marker.exists(), "background command continued after timeout");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn successful_command_does_not_leave_background_children() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("child-ran");
        let args = vec!["/bin/sh".into(), "-c".into(),
            "(sleep 0.3; printf leaked > \"$1\") & exit 0".into(),
            "workflow-test".into(), marker.to_string_lossy().into_owned()];
        let result = run_command(temp.path(), &args).await.unwrap();
        assert!(result.success);
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!marker.exists(), "background command continued after test exit");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn cancelled_command_does_not_leave_background_children() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("child-ran");
        let ready = temp.path().join("ready");
        let args = vec!["/bin/sh".into(), "-c".into(),
            "(sleep 1; printf leaked > \"$1\") & printf ready > \"$2\"; wait".into(),
            "workflow-test".into(), marker.to_string_lossy().into_owned(), ready.to_string_lossy().into_owned()];
        let cwd = temp.path().to_owned();
        let task = tokio::spawn(async move { run_command(&cwd, &args).await });
        tokio::time::timeout(Duration::from_secs(3), async {
            while !ready.exists() { tokio::time::sleep(Duration::from_millis(10)).await; }
        }).await.expect("command did not start");
        task.abort();
        let _ = task.await;
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(!marker.exists(), "background command continued after cancellation");
    }
}

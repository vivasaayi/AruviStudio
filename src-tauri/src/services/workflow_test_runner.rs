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
    if let Some(value) = get_setting(pool, &product_command_key(product_id)).await? {
        if value == "[]" { return Ok(("disabled".into(), vec![])); }
        return Ok(("custom".into(), parse_command(&value)?));
    }
    match get_setting(pool, TEST_COMMAND_KEY).await? {
        Some(value) if !value.trim().is_empty() => Ok(("inherit".into(), parse_command(&value)?)),
        _ => Ok(("inherit".into(), vec![])),
    }
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
    let process_group = child.id();
    let stdout = child.stdout.take().ok_or_else(|| AppError::Internal("Unable to capture test stdout".into()))?;
    let stderr = child.stderr.take().ok_or_else(|| AppError::Internal("Unable to capture test stderr".into()))?;
    let mut stdout_task = tokio::spawn(collect_output(stdout));
    let mut stderr_task = tokio::spawn(collect_output(stderr));
    let (status, timed_out) = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => (Some(result?), false),
        Err(_) => {
            #[cfg(unix)]
            if let Some(id) = process_group { unsafe { libc::kill(-(id as i32), libc::SIGKILL); } }
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
}

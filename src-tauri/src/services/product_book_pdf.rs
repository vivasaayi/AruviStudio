use crate::error::AppError;
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

const WORKER: &str = include_str!("../../resources/render-product-book-pdf.cjs");
const MAX_HTML_BYTES: usize = 12 * 1024 * 1024;
const MAX_PDF_BYTES: u64 = 24 * 1024 * 1024;

// Playwright traps SIGTERM and closes its separately grouped Chromium process.
// This guard also runs when a requesting client cancels the future.
#[cfg(unix)]
struct WorkerGuard(Option<u32>);
#[cfg(unix)]
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.0 { unsafe { libc::kill(-(pid as i32), libc::SIGTERM); } }
    }
}

pub async fn render_pdf(html: &str, name: &str, trim: &Value) -> Result<Vec<u8>, AppError> {
    if html.len() > MAX_HTML_BYTES {
        return Err(AppError::Validation("Product Book PDF source exceeds the 12 MiB limit".into()));
    }
    let temp = tempfile::Builder::new().prefix("aruvi-book-pdf-").tempdir()?;
    let node = std::env::var_os("ARUVI_PDF_NODE").unwrap_or_else(|| "/usr/local/bin/node".into());
    let browser = std::env::var_os("ARUVI_PDF_BROWSER").unwrap_or_else(|| "/usr/bin/chromium".into());
    let playwright = std::env::var_os("ARUVI_PDF_PLAYWRIGHT_PATH").unwrap_or_else(|| "/usr/local/lib/aruvi-pdf/playwright-core".into());
    let sandbox = std::env::var("ARUVI_PDF_SANDBOX").unwrap_or_else(|_| "1".into());
    if sandbox != "0" && sandbox != "1" { return Err(AppError::Validation("ARUVI_PDF_SANDBOX must be 0 or 1".into())); }
    let request = serde_json::to_vec(&serde_json::json!({"html": html, "name": name, "trim": trim}))?;
    let mut command = Command::new(node);
    command.arg("--max-old-space-size=256").arg("-e").arg(WORKER)
        .env_clear().env("PATH", "/usr/bin:/bin:/usr/local/bin")
        .env("HOME", temp.path()).env("TMPDIR", temp.path())
        .env("ARUVI_PDF_BROWSER", browser).env("ARUVI_PDF_PLAYWRIGHT_PATH", playwright)
        .env("ARUVI_PDF_SANDBOX", sandbox)
        .current_dir(temp.path()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(not(unix))]
    command.kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| AppError::Validation("Product Book PDF renderer is unavailable".into()))?;
    #[cfg(unix)]
    let mut guard = WorkerGuard(child.id());
    let mut stdin = child.stdin.take().ok_or_else(|| AppError::Internal("PDF worker input unavailable".into()))?;
    let stdout = child.stdout.take().ok_or_else(|| AppError::Internal("PDF worker output unavailable".into()))?;
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        let send = async move {
            stdin.write_all(&request).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, std::io::Error>(())
        };
        let receive = async {
            let mut bytes = Vec::new();
            stdout.take(MAX_PDF_BYTES + 1).read_to_end(&mut bytes).await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let (_, bytes) = tokio::try_join!(send, receive)
            .map_err(|_| AppError::Validation("Product Book PDF renderer failed".into()))?;
        if bytes.len() as u64 > MAX_PDF_BYTES { return Err(AppError::Validation("Product Book PDF exceeds the 24 MiB limit".into())); }
        let status = child.wait().await?;
        #[cfg(unix)]
        { guard.0 = None; }
        if !status.success() || !bytes.starts_with(b"%PDF-") {
            return Err(AppError::Validation("Product Book PDF rendering failed".into()));
        }
        Ok(bytes)
    }).await.map_err(|_| AppError::Validation("Product Book PDF rendering timed out".into()))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_oversized_pdf_input_before_starting_worker() {
        let result = render_pdf(&"x".repeat(MAX_HTML_BYTES + 1), "Test", &Value::Null).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    #[ignore = "requires a configured Chromium and Playwright installation"]
    async fn browser_does_not_fetch_external_content_or_execute_scripts() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/secret", listener.local_addr().unwrap());
        let html = format!("<html><head><style>@import url('{url}');</style></head><body><img src='{url}'><script>fetch('{url}')</script><p>Safe page</p></body></html>");
        let trim = serde_json::json!({ "pageWidth":"6in", "pageHeight":"9in", "marginTop":"0.75in", "marginRight":"0.72in", "marginBottom":"0.85in", "marginLeft":"0.82in", "label":"Trade" });
        let pdf = render_pdf(&html, "Safe", &trim).await.unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(tokio::time::timeout(Duration::from_secs(2), listener.accept()).await.is_err());
    }
}

use crate::error::AppError;
use serde_json::Value;
use futures_util::StreamExt;
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
    if let Some(socket) = std::env::var_os("ARUVI_PDF_SOCKET") {
        return render_pdf_via_socket(&socket, html, name, trim).await;
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

#[cfg(unix)]
async fn render_pdf_via_socket(socket: &std::ffi::OsStr, html: &str, name: &str, trim: &Value) -> Result<Vec<u8>, AppError> {
    let socket_path = std::path::Path::new(socket);
    if !socket_path.is_absolute() {
        return Err(AppError::Validation("Product Book PDF socket path must be absolute".into()));
    }
    let client = reqwest::Client::builder().no_proxy().unix_socket(socket_path.to_path_buf())
        .redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(60)).build()
        .map_err(|_| AppError::Internal("Product Book PDF client unavailable".into()))?;
    let response = client.post("http://localhost/render")
        .json(&serde_json::json!({ "html": html, "name": name, "trim": trim })).send().await
        .map_err(|_| AppError::Validation("Isolated Product Book PDF renderer is unavailable".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Validation("Isolated Product Book PDF rendering failed".into()));
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| AppError::Validation("Product Book PDF response failed".into()))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_PDF_BYTES as usize {
            return Err(AppError::Validation("Product Book PDF exceeds the 24 MiB limit".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !bytes.starts_with(b"%PDF-") { return Err(AppError::Validation("Product Book PDF renderer returned invalid output".into())); }
    Ok(bytes)
}

#[cfg(not(unix))]
async fn render_pdf_via_socket(_socket: &std::ffi::OsStr, _html: &str, _name: &str, _trim: &Value) -> Result<Vec<u8>, AppError> {
    Err(AppError::Validation("Product Book PDF sockets are unsupported on this platform".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_oversized_pdf_input_before_starting_worker() {
        let result = render_pdf(&"x".repeat(MAX_HTML_BYTES + 1), "Test", &Value::Null).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn isolated_renderer_fails_closed_when_socket_is_missing_or_output_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("renderer.sock");
        assert!(render_pdf_via_socket(path.as_os_str(), "<html></html>", "Test", &Value::Null).await.is_err());
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\noops").await.unwrap();
        });
        assert!(render_pdf_via_socket(path.as_os_str(), "<html></html>", "Test", &Value::Null).await.is_err());
        task.await.unwrap();
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

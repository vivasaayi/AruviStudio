//! Narrow bridge to Aruvici's private local CI socket.
//!
//! The UI may inspect and queue approved Preview builds. Production approval,
//! deployment, and arbitrary command execution deliberately remain unavailable.
use directories::BaseDirs;
use serde_json::{json, Value};
use std::{
    io::{BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process::Command,
    time::Duration,
};

fn socket_path() -> Result<PathBuf, String> {
    let base = BaseDirs::new().ok_or("cannot determine user data directory")?;
    Ok(base.data_dir().join("aruvici").join("ci.sock"))
}

fn request(value: Value) -> Result<Value, String> {
    let socket = socket_path()?;
    let mut stream = UnixStream::connect(&socket).map_err(|_| {
        format!(
            "Aruvici is offline. Start its local CI service first ({})",
            socket.display()
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut stream, &value).map_err(|e| e.to_string())?;
    stream.write_all(b"\n").map_err(|e| e.to_string())?;
    let mut response = String::new();
    BufReader::new(stream)
        .take(4 * 1024 * 1024)
        .read_to_string(&mut response)
        .map_err(|e| e.to_string())?;
    let envelope: Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    if envelope["ok"] == Value::Bool(true) {
        Ok(envelope["result"].clone())
    } else {
        Err(envelope["error"]
            .as_str()
            .unwrap_or("Aruvici request failed")
            .to_owned())
    }
}

#[tauri::command]
pub fn get_local_ci_status() -> Result<Value, String> {
    request(json!({"method":"health"}))
}

#[tauri::command]
pub fn list_local_ci_targets() -> Result<Value, String> {
    request(json!({"method":"targets"}))
}

#[tauri::command]
pub fn list_local_ci_runs() -> Result<Value, String> {
    request(json!({"method":"runs"}))
}

#[tauri::command]
pub fn queue_local_preview(commit: String, product_id: Option<String>) -> Result<Value, String> {
    if !matches!(commit.len(), 40 | 64) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Preview builds require a committed 40- or 64-character Git revision".into());
    }
    request(json!({
        "method":"enqueue",
        "target":"aruvi-studio-preview",
        "commit":commit,
        "key":format!("studio-preview-{commit}"),
        "context":{"trigger":"aruvi-studio-ui","product_id":product_id},
    }))
}

#[tauri::command]
pub fn open_local_preview() -> Result<(), String> {
    let base = BaseDirs::new().ok_or("cannot determine user data directory")?;
    let app = base
        .data_dir()
        .join("aruvici/previews/aruvi-studio-preview/current/AruviStudio Preview.app");
    if !app.is_dir() {
        return Err("No Preview candidate is installed yet".into());
    }
    Command::new("/usr/bin/open")
        .arg(&app)
        .spawn()
        .map_err(|e| format!("open Preview: {e}"))?;
    Ok(())
}

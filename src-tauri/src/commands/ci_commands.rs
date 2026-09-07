//! Narrow bridge to Aruvici's private local CI socket.
//!
//! The UI may inspect and queue approved Preview builds. Production approval,
//! deployment, and arbitrary command execution deliberately remain unavailable.
use crate::state::AppState;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::{
    io::{BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process::Command,
    time::Duration,
};
use tauri::State;
use uuid::Uuid;

fn socket_path() -> Result<PathBuf, String> {
    let base = BaseDirs::new().ok_or("cannot determine user data directory")?;
    Ok(base.data_dir().join("aruvici").join("ci.sock"))
}

pub(crate) fn request(value: Value) -> Result<Value, String> {
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

#[derive(Debug, Serialize, FromRow)]
pub struct CiTargetBinding {
    pub id: String,
    pub product_id: String,
    pub repository_id: String,
    pub target_id: String,
    pub auto_preview: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveCiTargetBinding {
    pub product_id: String,
    pub repository_id: String,
    pub target_id: String,
    pub auto_preview: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CiFeedback {
    pub id: String,
    pub ci_run_id: i64,
    pub commit_sha: String,
    pub product_id: Option<String>,
    pub work_item_id: Option<String>,
    pub verdict: String,
    pub notes: String,
    pub created_at: String,
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

#[tauri::command]
pub async fn list_ci_target_bindings(
    state: State<'_, AppState>,
) -> Result<Vec<CiTargetBinding>, String> {
    sqlx::query_as("SELECT id,product_id,repository_id,target_id,auto_preview,created_at,updated_at FROM ci_target_bindings ORDER BY created_at")
        .fetch_all(&state.db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_ci_target_binding(
    state: State<'_, AppState>,
    request: SaveCiTargetBinding,
) -> Result<CiTargetBinding, String> {
    if request.target_id.is_empty()
        || !request
            .target_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err("invalid CI target ID".into());
    }
    if request.auto_preview && !request.target_id.ends_with("-preview") {
        return Err("automatic delivery is restricted to Preview targets".into());
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query_as("INSERT INTO ci_target_bindings(id,product_id,repository_id,target_id,auto_preview) VALUES(?,?,?,?,?) ON CONFLICT(product_id,repository_id) DO UPDATE SET target_id=excluded.target_id,auto_preview=excluded.auto_preview,updated_at=datetime('now') RETURNING id,product_id,repository_id,target_id,auto_preview,created_at,updated_at")
        .bind(id).bind(request.product_id).bind(request.repository_id).bind(request.target_id).bind(request.auto_preview)
        .fetch_one(&state.db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_ci_feedback(
    state: State<'_, AppState>,
    ci_run_id: i64,
    commit_sha: String,
    product_id: Option<String>,
    work_item_id: Option<String>,
    verdict: String,
    notes: String,
) -> Result<CiFeedback, String> {
    if !["works", "needs_changes", "blocked"].contains(&verdict.as_str()) {
        return Err("invalid feedback verdict".into());
    }
    if notes.len() > 20_000 {
        return Err("feedback must be 20,000 characters or fewer".into());
    }
    sqlx::query_as("INSERT INTO ci_feedback(id,ci_run_id,commit_sha,product_id,work_item_id,verdict,notes) VALUES(?,?,?,?,?,?,?) RETURNING id,ci_run_id,commit_sha,product_id,work_item_id,verdict,notes,created_at")
        .bind(Uuid::new_v4().to_string()).bind(ci_run_id).bind(commit_sha).bind(product_id).bind(work_item_id).bind(verdict).bind(notes)
        .fetch_one(&state.db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ci_feedback(
    state: State<'_, AppState>,
    ci_run_id: i64,
) -> Result<Vec<CiFeedback>, String> {
    sqlx::query_as("SELECT id,ci_run_id,commit_sha,product_id,work_item_id,verdict,notes,created_at FROM ci_feedback WHERE ci_run_id=? ORDER BY created_at DESC")
        .bind(ci_run_id).fetch_all(&state.db).await.map_err(|e| e.to_string())
}

//! Narrow bridge to Aruvici's private local CI socket.
//!
//! The UI may inspect and queue approved build targets. Production approval,
//! deployment, and arbitrary command execution deliberately remain unavailable.
use crate::persistence::repository_repo;
use crate::state::AppState;
use directories::BaseDirs;
use git2::{BranchType, Repository as GitRepository, Sort};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::{
    fs,
    io::{BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tauri::State;
use uuid::Uuid;

fn socket_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("ARUVI_CI_SOCKET") {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("ARUVI_CI_SOCKET must be an absolute path".into());
        }
        return Ok(path);
    }
    let base = BaseDirs::new().ok_or("cannot determine user data directory")?;
    Ok(base.data_dir().join("aruvici").join("ci.sock"))
}

pub(crate) fn request(value: Value) -> Result<Value, String> {
    let socket = socket_path()?;
    request_at(&socket, value)
}

fn request_at(socket: &Path, value: Value) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket).map_err(|_| {
        format!(
            "Aruvici is offline. Start its local CI service first ({})",
            socket.display()
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod socket_tests {
    use super::*;
    use std::io::BufRead;
    use std::os::unix::net::UnixListener;

    #[test]
    fn request_uses_json_protocol_and_reports_service_error() {
        // Unix socket paths are limited to 104 bytes on macOS; its default
        // temporary directory is long, so use a short unique directory name.
        let dir = std::env::temp_dir().join(format!("ci-{}", Uuid::new_v4().simple()));
        fs::create_dir(&dir).unwrap();
        let socket = dir.join("ci.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = std::thread::spawn(move || {
            for response in ["{\"ok\":true,\"result\":{\"status\":\"online\"}}\n", "{\"ok\":false,\"error\":\"synthetic failure\"}\n"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(stream.try_clone().unwrap()).read_line(&mut line).unwrap();
                assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["method"], "health");
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        assert_eq!(request_at(&socket, json!({"method":"health"})).unwrap()["status"], "online");
        assert_eq!(request_at(&socket, json!({"method":"health"})).unwrap_err(), "synthetic failure");
        server.join().unwrap();
        fs::remove_dir_all(dir).unwrap();
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

/// A local, immutable Git revision that can be safely handed to Aruvici.
/// The path is never accepted from the UI: it is loaded from a registered
/// AruviStudio repository record.
#[derive(Debug, Serialize)]
pub struct CiGitCommit {
    pub sha: String,
    pub summary: String,
    pub committed_at: i64,
}

#[derive(Debug, Serialize)]
pub struct CiGitBranch {
    pub name: String,
    pub commit: CiGitCommit,
}

#[derive(Debug, Deserialize)]
struct LocalCiTarget {
    repository: Option<String>,
    approval: Option<Value>,
    plan: Option<Value>,
}

fn open_registered_git_repository(path: &str) -> Result<GitRepository, String> {
    GitRepository::open(path).map_err(|error| {
        format!("registered repository is not available as a Git repository: {error}")
    })
}

fn commit_details(repository: &GitRepository, oid: git2::Oid) -> Result<CiGitCommit, String> {
    let commit = repository
        .find_commit(oid)
        .map_err(|error| error.to_string())?;
    Ok(CiGitCommit {
        sha: commit.id().to_string(),
        summary: commit.summary().ok().flatten().unwrap_or("(no commit message)").to_owned(),
        committed_at: commit.time().seconds(),
    })
}

async fn registered_repository_path(
    state: &AppState,
    repository_id: &str,
) -> Result<String, String> {
    repository_repo::get_repository(&state.db, repository_id)
        .await
        .map(|repository| repository.local_path)
        .map_err(|error| format!("registered repository was not found: {error}"))
}

fn normalized_repository_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("repository path must be absolute".into());
    }
    fs::canonicalize(path).map_err(|error| {
        format!(
            "repository path must exist and be accessible ({}): {error}",
            path.display()
        )
    })
}

fn target_metadata(target_id: &str) -> Result<LocalCiTarget, String> {
    let targets = request(json!({"method":"targets"}))?;
    targets
        .as_array()
        .and_then(|items| {
            items.iter().find_map(|item| {
                (item.get("id").and_then(Value::as_str) == Some(target_id))
                    .then(|| serde_json::from_value::<LocalCiTarget>(item.clone()).ok())
                    .flatten()
            })
        })
        .ok_or_else(|| format!("CI target '{target_id}' is not registered in Aruvici"))
}

async fn validate_target_binding(
    state: &AppState,
    product_id: &str,
    repository_id: &str,
    target_id: &str,
) -> Result<(), String> {
    if product_id.trim().is_empty() || repository_id.trim().is_empty() {
        return Err("a product and repository are required for an approved build target".into());
    }
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM ci_target_bindings WHERE product_id=? AND repository_id=? AND target_id=?)",
    )
    .bind(product_id)
    .bind(repository_id)
    .bind(target_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    if !bound {
        return Err(format!(
            "CI target '{target_id}' is not approved for this product and repository"
        ));
    }

    validate_repository_attachment(state, product_id, repository_id).await?;
    validate_target_repository(state, repository_id, target_id).await
}

async fn validate_repository_attachment(
    state: &AppState,
    product_id: &str,
    repository_id: &str,
) -> Result<(), String> {
    let attached: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM repository_attachments WHERE scope_type='product' AND scope_id=? AND repository_id=?)",
    )
    .bind(product_id)
    .bind(repository_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    if !attached {
        return Err("the repository must be attached to the selected product before approving or queueing a build target".into());
    }
    Ok(())
}

async fn validate_target_repository(
    state: &AppState,
    repository_id: &str,
    target_id: &str,
) -> Result<(), String> {
    let repository_path = registered_repository_path(state, repository_id).await?;
    let owned_target = target_id.to_owned();
    let target = tokio::task::spawn_blocking(move || target_metadata(&owned_target)).await.map_err(|e| e.to_string())??;
    let Some(target_path) = target.repository.as_deref() else {
        return Err(format!(
            "CI target '{target_id}' has no repository metadata"
        ));
    };
    let registered = normalized_repository_path(&repository_path)?;
    let configured = normalized_repository_path(target_path)?;
    if registered != configured {
        return Err(format!(
            "CI target '{target_id}' repository does not match the registered repository path"
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_ci_repository_branches(
    state: State<'_, AppState>,
    repository_id: String,
) -> Result<Vec<CiGitBranch>, String> {
    list_repository_branches(&state, &repository_id).await
}

pub(crate) async fn list_repository_branches(state: &AppState, repository_id: &str) -> Result<Vec<CiGitBranch>, String> {
    let path = registered_repository_path(state, repository_id).await?;
    let repository = open_registered_git_repository(&path)?;
    let mut branches = Vec::new();
    for branch in repository
        .branches(Some(BranchType::Local))
        .map_err(|error| error.to_string())?
    {
        let (branch, _) = branch.map_err(|error| error.to_string())?;
        let Some(name) = branch.name().map_err(|error| error.to_string())? else {
            continue;
        };
        let Some(target) = branch.get().target() else {
            continue;
        };
        branches.push(CiGitBranch {
            name: name.to_owned(),
            commit: commit_details(&repository, target)?,
        });
    }
    branches.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(branches)
}

#[tauri::command]
pub async fn list_ci_repository_commits(
    state: State<'_, AppState>,
    repository_id: String,
) -> Result<Vec<CiGitCommit>, String> {
    list_repository_commits(&state, &repository_id).await
}

pub(crate) async fn list_repository_commits(state: &AppState, repository_id: &str) -> Result<Vec<CiGitCommit>, String> {
    let path = registered_repository_path(state, repository_id).await?;
    let repository = open_registered_git_repository(&path)?;
    let mut walk = repository.revwalk().map_err(|error| error.to_string())?;
    walk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
        .map_err(|error| error.to_string())?;
    for reference in repository.references().map_err(|error| error.to_string())? {
        let reference = reference.map_err(|error| error.to_string())?;
        if let Some(target) = reference.target() {
            walk.push(target).map_err(|error| error.to_string())?;
        }
    }
    walk.take(100)
        .map(|oid| {
            oid.map_err(|error| error.to_string())
                .and_then(|oid| commit_details(&repository, oid))
        })
        .collect()
}

async fn resolve_ci_revision(
    state: &AppState,
    repository_id: &str,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<String, String> {
    if branch.is_some() == commit.is_some() {
        return Err("select either a branch or one specific commit".into());
    }
    let path = registered_repository_path(state, repository_id).await?;
    let repository = open_registered_git_repository(&path)?;
    let oid = if let Some(branch) = branch {
        let branch = repository
            .find_branch(&branch, BranchType::Local)
            .map_err(|_| "selected branch is no longer available locally".to_string())?;
        branch
            .get()
            .target()
            .ok_or("selected branch has no commit")?
    } else {
        let commit = commit.expect("validated above");
        if !matches!(commit.len(), 40 | 64) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("selected commit must be a full Git SHA".into());
        }
        git2::Oid::from_str(&commit)
            .map_err(|_| "selected commit is not a valid Git SHA".to_string())?
    };
    repository.find_commit(oid).map_err(|_| {
        "selected commit is not available in the registered local repository".to_string()
    })?;
    Ok(oid.to_string())
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
pub fn get_local_ci_run(id: i64) -> Result<Value, String> {
    request(json!({"method":"run","id":id}))
}

#[tauri::command]
pub fn cancel_local_ci_run(id: i64) -> Result<Value, String> {
    request(json!({"method":"cancel","id":id}))
}

#[tauri::command]
pub fn retry_local_ci_run(id: i64) -> Result<Value, String> {
    request(json!({
        "method":"retry",
        "id":id,
        "key":format!("studio-retry-{id}-{}", Uuid::new_v4()),
    }))
}

#[tauri::command]
pub fn get_local_ci_log(id: i64, stage: String, offset: Option<u64>) -> Result<Value, String> {
    request(json!({"method":"logs","id":id,"stage":stage,"offset":offset.unwrap_or(0)}))
}

#[tauri::command]
pub fn get_ci_promotion_readiness(artifact_id: i64) -> Result<Value, String> {
    request(json!({"method":"promotion_readiness","id":artifact_id}))
}

/// Reveal only a manager-recorded artifact selected by numeric ID. The UI
/// cannot supply a filesystem path through this command.
#[tauri::command]
pub fn reveal_ci_artifact(artifact_id: i64) -> Result<(), String> {
    // Promotion readiness validates run ownership, path confinement, size and
    // hash for every artifact kind before Finder receives the stored path.
    let artifact = request(json!({"method":"promotion_readiness","id":artifact_id}))?;
    let path = artifact["path"]
        .as_str()
        .ok_or("Aruvici returned an artifact without a path")?;
    let path =
        fs::canonicalize(path).map_err(|error| format!("artifact is unavailable: {error}"))?;
    if !path.is_file() {
        return Err("artifact is not a regular file".into());
    }
    let status = Command::new("/usr/bin/open")
        .arg("-R")
        .arg(&path)
        .status()
        .map_err(|error| format!("reveal artifact in Finder: {error}"))?;
    if !status.success() {
        return Err("Finder could not reveal the artifact".into());
    }
    Ok(())
}

#[tauri::command]
pub fn queue_local_preview(commit: String, product_id: Option<String>) -> Result<Value, String> {
    queue_target(
        "aruvi-studio-preview",
        commit,
        product_id,
        "aruvi-studio-ui",
    )
}

/// Queue a Preview build from a product's registered repository. A branch is
/// resolved here, immediately before dispatch, so Aruvici always receives an
/// immutable SHA rather than a moving branch name.
#[tauri::command]
pub async fn queue_local_preview_reference(
    state: State<'_, AppState>,
    repository_id: String,
    branch: Option<String>,
    commit: Option<String>,
    product_id: Option<String>,
) -> Result<Value, String> {
    let revision = resolve_ci_revision(&state, &repository_id, branch, commit).await?;
    if let Some(product_id) = product_id.as_deref() {
        validate_target_binding(&state, product_id, &repository_id, "aruvi-studio-preview").await?;
    }
    queue_target(
        "aruvi-studio-preview",
        revision,
        product_id,
        "aruvi-studio-ui",
    )
}

/// Queue any explicitly approved target from a product's registered
/// repository. The target/repository binding and Aruvici's target metadata are
/// checked immediately before dispatch; branch names are resolved to immutable
/// Git SHAs in the registered checkout.
#[tauri::command]
pub async fn queue_local_target_reference(
    state: State<'_, AppState>,
    target_id: String,
    repository_id: String,
    branch: Option<String>,
    commit: Option<String>,
    product_id: String,
) -> Result<Value, String> {
    queue_target_reference(&state, target_id, repository_id, branch, commit, product_id).await
}

pub(crate) async fn queue_target_reference(state: &AppState, target_id: String, repository_id: String, branch: Option<String>, commit: Option<String>, product_id: String) -> Result<Value, String> {
    validate_target_binding(state, &product_id, &repository_id, &target_id).await?;
    let readiness_target = target_id.clone();
    let metadata = tokio::task::spawn_blocking(move || target_metadata(&readiness_target)).await.map_err(|error| error.to_string())??;
    let target_path = metadata.repository.as_deref().ok_or("CI target has no repository metadata")?;
    if normalized_repository_path(&registered_repository_path(state, &repository_id).await?)? != normalized_repository_path(target_path)? {
        return Err("CI target repository changed before queueing".into());
    }
    if metadata.approval.as_ref().and_then(|value| value.get("approved")).and_then(Value::as_bool) != Some(true) {
        return Err("CI pipeline approval is required before queueing".into());
    }
    if metadata.plan.as_ref().and_then(|value| value.get("issues")).and_then(Value::as_array).is_some_and(|issues| !issues.is_empty()) {
        return Err("CI target has configuration issues".into());
    }
    let revision = resolve_ci_revision(state, &repository_id, branch, commit).await?;
    tokio::task::spawn_blocking(move || queue_target(&target_id, revision, Some(product_id), "aruvi-studio-target-ui"))
        .await.map_err(|error| error.to_string())?
}

fn queue_target(
    target: &str,
    commit: String,
    product_id: Option<String>,
    trigger: &str,
) -> Result<Value, String> {
    if !matches!(commit.len(), 40 | 64) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("builds require a committed 40- or 64-character Git revision".into());
    }
    request(json!({
        "method":"enqueue",
        "target":target,
        "commit":commit,
        "key":format!("studio-{target}-{commit}"),
        "context":{"trigger":trigger,"product_id":product_id},
    }))
}

#[tauri::command]
pub fn queue_local_release(
    target: String,
    commit: String,
    product_id: Option<String>,
) -> Result<Value, String> {
    if !["aruvi-studio", "aruvi-studio-intel"].contains(&target.as_str()) {
        return Err(
            "only the registered ARM64 and Intel AruviStudio release targets may be queued here"
                .into(),
        );
    }
    queue_target(&target, commit, product_id, "aruvi-studio-release-ui")
}

/// Queue an artifact-only production build from a selected local branch or
/// historic commit. This command intentionally cannot install or promote it.
#[tauri::command]
pub async fn queue_local_release_reference(
    state: State<'_, AppState>,
    target: String,
    repository_id: String,
    branch: Option<String>,
    commit: Option<String>,
    product_id: Option<String>,
) -> Result<Value, String> {
    if !["aruvi-studio", "aruvi-studio-intel"].contains(&target.as_str()) {
        return Err(
            "only the registered ARM64 and Intel AruviStudio release targets may be queued here"
                .into(),
        );
    }
    let revision = resolve_ci_revision(&state, &repository_id, branch, commit).await?;
    queue_target(&target, revision, product_id, "aruvi-studio-release-ui")
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
    list_target_bindings(&state).await
}

pub(crate) async fn list_target_bindings(state: &AppState) -> Result<Vec<CiTargetBinding>, String> {
    sqlx::query_as("SELECT id,product_id,repository_id,target_id,auto_preview,created_at,updated_at FROM ci_target_bindings ORDER BY product_id,repository_id,target_id,created_at")
        .fetch_all(&state.db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_ci_target_binding(
    state: State<'_, AppState>,
    request: SaveCiTargetBinding,
) -> Result<CiTargetBinding, String> {
    save_target_binding(&state, request).await
}

pub(crate) async fn save_target_binding(state: &AppState, request: SaveCiTargetBinding) -> Result<CiTargetBinding, String> {
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
    validate_repository_attachment(state, &request.product_id, &request.repository_id).await?;
    validate_target_repository(state, &request.repository_id, &request.target_id).await?;
    let id = Uuid::new_v4().to_string();
    sqlx::query_as("INSERT INTO ci_target_bindings(id,product_id,repository_id,target_id,auto_preview) SELECT ?,?,?,?,? WHERE EXISTS(SELECT 1 FROM repository_attachments WHERE scope_type='product' AND scope_id=? AND repository_id=?) ON CONFLICT(product_id,repository_id,target_id) DO UPDATE SET auto_preview=excluded.auto_preview,updated_at=datetime('now') RETURNING id,product_id,repository_id,target_id,auto_preview,created_at,updated_at")
        .bind(id).bind(&request.product_id).bind(&request.repository_id).bind(&request.target_id).bind(request.auto_preview).bind(&request.product_id).bind(&request.repository_id)
        .fetch_optional(&state.db).await.map_err(|e| e.to_string())?.ok_or_else(|| "repository attachment changed before assignment".into())
}

#[tauri::command]
pub async fn delete_ci_target_binding(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_target_binding(&state, id).await
}

pub(crate) async fn delete_target_binding(state: &AppState, id: String) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM ci_target_bindings WHERE id=?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    if result.rows_affected() == 0 {
        return Err("CI binding was not found".into());
    }
    Ok(())
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
    insert_ci_feedback(&state, ci_run_id, commit_sha, product_id, work_item_id, verdict, notes).await
}

pub(crate) async fn insert_ci_feedback(state: &AppState, ci_run_id: i64, commit_sha: String, product_id: Option<String>, work_item_id: Option<String>, verdict: String, notes: String) -> Result<CiFeedback, String> {
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
    list_feedback(&state, ci_run_id).await
}

pub(crate) async fn list_feedback(state: &AppState, ci_run_id: i64) -> Result<Vec<CiFeedback>, String> {
    sqlx::query_as("SELECT id,ci_run_id,commit_sha,product_id,work_item_id,verdict,notes,created_at FROM ci_feedback WHERE ci_run_id=? ORDER BY created_at DESC")
        .bind(ci_run_id).fetch_all(&state.db).await.map_err(|e| e.to_string())
}

pub(crate) async fn record_verified_feedback(state: &AppState, id: i64, verdict: String, notes: String) -> Result<CiFeedback, String> {
    if id <= 0 { return Err("invalid CI run ID".into()); }
    let run = tokio::task::spawn_blocking(move || request(json!({"method":"run","id":id}))).await.map_err(|e| e.to_string())??;
    if run.get("id").and_then(Value::as_i64) != Some(id) || run.get("status").and_then(Value::as_str) != Some("passed") {
        return Err("feedback requires a passed CI run".into());
    }
    let commit = run.get("commit").and_then(Value::as_str).filter(|s| matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit())).ok_or("CI run has no valid commit SHA")?;
    insert_ci_feedback(state, id, commit.to_owned(), None, None, verdict, notes).await
}

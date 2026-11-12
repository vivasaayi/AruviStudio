use crate::error::AppError;
use crate::execution::workspace::{confined_file_path, safe_relative_path};
use crate::persistence::settings_repo;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DeliveryManifest {
    pub repository_id: String,
    pub base_head: Option<String>,
    pub files: BTreeMap<String, String>,
    pub commit: Option<String>,
}

fn key(run_id: &str) -> String { format!("workflow.delivery_manifest.{run_id}") }

pub(crate) async fn load(db: &SqlitePool, run_id: &str) -> Result<Option<DeliveryManifest>, AppError> {
    settings_repo::get_setting(db, &key(run_id)).await?.map(|value| {
        serde_json::from_str(&value).map_err(|_| AppError::Validation("Invalid workflow delivery manifest".to_string()))
    }).transpose()
}

pub(crate) async fn save(db: &SqlitePool, run_id: &str, manifest: &DeliveryManifest) -> Result<(), AppError> {
    settings_repo::set_setting(db, &key(run_id), &serde_json::to_string(manifest)?).await
}

pub(crate) async fn previous_files(db: &SqlitePool, run_id: &str, repository_id: &str, base_head: &Option<String>) -> Result<BTreeMap<String, String>, AppError> {
    match load(db, run_id).await? {
        Some(manifest) if manifest.repository_id == repository_id && &manifest.base_head == base_head && manifest.commit.is_none() => Ok(manifest.files),
        Some(_) => Err(AppError::Validation("Workflow delivery repository or base commit changed; review before retrying".to_string())),
        None => Ok(BTreeMap::new()),
    }
}

pub(crate) fn file_digest(root: &Path, relative: &Path) -> Result<String, AppError> {
    let path = confined_file_path(root, relative, false)?;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn repository_head(repo_path: &str) -> Option<String> {
    git2::Repository::open(repo_path).ok().and_then(|repo| repo.head().ok()?.target().map(|id| id.to_string()))
}

pub(crate) async fn record(
    db: &SqlitePool, run_id: &str, repository_id: &str, repository_path: &str,
    workspace_root: &Path, paths: &[String], base_head: Option<String>,
) -> Result<(), AppError> {
    let mut manifest = load(db, run_id).await?.unwrap_or_else(|| DeliveryManifest {
        repository_id: repository_id.to_string(), base_head: base_head.clone(), files: BTreeMap::new(), commit: None,
    });
    if manifest.repository_id != repository_id || manifest.base_head != base_head || manifest.commit.is_some() {
        return Err(AppError::Validation("Workflow delivery repository or base commit changed; review before retrying".to_string()));
    }
    for path in paths {
        let relative = safe_relative_path(path)?;
        let source_hash = file_digest(workspace_root, &relative)?;
        let target_hash = file_digest(Path::new(repository_path), &relative)?;
        if source_hash != target_hash { return Err(AppError::Validation(format!("Repository file changed while syncing: {}", relative.display()))); }
        let normalized = relative.to_str().ok_or_else(|| AppError::Validation("Repository path must be UTF-8".to_string()))?;
        manifest.files.insert(normalized.to_string(), source_hash);
    }
    save(db, run_id, &manifest).await
}

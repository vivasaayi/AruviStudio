use crate::error::AppError;
use crate::persistence::planner_repo;
use crate::services::planner_service::{PlannerDraftPlan, PlannerPlan};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerSessionInfo {
    pub session_id: String,
    pub provider_id: Option<String>,
    pub model_name: Option<String>,
    pub has_pending_plan: bool,
    pub has_draft_plan: bool,
    pub selected_draft_node_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannerConversationEntry {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannerSession {
    pub(crate) provider_id: Option<String>,
    pub(crate) model_name: Option<String>,
    pub(crate) pending_plan: Option<PlannerPlan>,
    pub(crate) draft_plan: Option<PlannerDraftPlan>,
    pub(crate) selected_draft_node_id: Option<String>,
    pub(crate) conversation: Vec<PlannerConversationEntry>,
}

pub struct PlannerService {
    sessions: HashMap<String, PlannerSession>,
    session_locks: HashMap<String, Weak<Mutex<()>>>,
}

impl PlannerService {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_locks: HashMap::new(),
        }
    }

    pub(crate) fn create_session(
        &mut self,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> PlannerSessionInfo {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.sessions.insert(
            session_id.clone(),
            PlannerSession {
                provider_id: provider_id.clone(),
                model_name: model_name.clone(),
                pending_plan: None,
                draft_plan: None,
                selected_draft_node_id: None,
                conversation: vec![],
            },
        );
        PlannerSessionInfo {
            session_id,
            provider_id,
            model_name,
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        }
    }

    pub(crate) fn update_session(
        &mut self,
        session_id: &str,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.provider_id = provider_id.clone();
        session.model_name = model_name.clone();
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id,
            model_name,
            has_pending_plan: session.pending_plan.is_some(),
            has_draft_plan: session.draft_plan.is_some(),
            selected_draft_node_id: session.selected_draft_node_id.clone(),
        })
    }

    pub(crate) fn clear_pending(
        &mut self,
        session_id: &str,
    ) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.pending_plan = None;
        session.draft_plan = None;
        session.selected_draft_node_id = None;
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id: session.provider_id.clone(),
            model_name: session.model_name.clone(),
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn get_session(&self, session_id: &str) -> Result<PlannerSession, AppError> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Planner session {} not found", session_id)))
    }

    pub(crate) fn save_session(&mut self, session_id: &str, session: PlannerSession) {
        self.sessions.insert(session_id.to_string(), session);
    }
}

// The in-process lock avoids contention between tasks; an advisory lock beside
// the SQLite database coordinates cooperating Studio processes on Unix.
pub(crate) struct PlannerSessionGuard {
    _task_guard: tokio::sync::OwnedMutexGuard<()>,
    #[cfg(unix)]
    _file: Option<std::fs::File>,
}

pub(crate) async fn lock_planner_session(
    planner_service: &Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSessionGuard, AppError> {
    let lock = {
        let mut service = planner_service.lock().await;
        service
            .session_locks
            .retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = service
            .session_locks
            .get(session_id)
            .and_then(Weak::upgrade)
        {
            lock
        } else {
            let lock = Arc::new(Mutex::new(()));
            service
                .session_locks
                .insert(session_id.to_string(), Arc::downgrade(&lock));
            lock
        }
    };
    let task_guard = lock.lock_owned().await;
    // Do not create lock files for nonexistent, attacker-supplied session IDs.
    planner_repo::get_session(db, session_id).await?;
    #[cfg(unix)]
    let file = acquire_planner_file_lock(db, session_id).await?;
    Ok(PlannerSessionGuard {
        _task_guard: task_guard,
        #[cfg(unix)]
        _file: file,
    })
}

#[cfg(unix)]
async fn acquire_planner_file_lock(
    db: &SqlitePool,
    session_id: &str,
) -> Result<Option<std::fs::File>, AppError> {
    use sha2::{Digest, Sha256};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    let databases: Vec<(i64, String, String)> =
        sqlx::query_as("PRAGMA database_list").fetch_all(db).await?;
    let path = databases
        .into_iter()
        .find(|(_, name, _)| name == "main")
        .map(|(_, _, path)| path)
        .unwrap_or_default();
    // SQLite in-memory databases have no path and cannot be shared by processes.
    if path.is_empty() {
        return Ok(None);
    }
    let database = std::fs::canonicalize(path)?;
    let parent = database
        .parent()
        .ok_or_else(|| AppError::Internal("Planner database has no parent directory".into()))?;
    let filename = database
        .file_name()
        .ok_or_else(|| AppError::Internal("Planner database has no filename".into()))?;
    let mut directory_name = std::ffi::OsString::from(".");
    directory_name.push(filename);
    directory_name.push(".planner-locks");
    let directory = parent.join(directory_name);
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
            "Planner lock directory must be private and must not be a symlink".into(),
        ));
    }
    let digest = Sha256::digest(session_id.as_bytes());
    let lock_name = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(directory.join(lock_name))?;
    if !file.metadata()?.is_file() {
        return Err(AppError::Internal(
            "Planner lock must be a regular file".into(),
        ));
    }
    loop {
        // SAFETY: the file remains owned by this function and then the guard,
        // so its descriptor stays valid throughout the flock call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(Some(file));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if error.kind() != std::io::ErrorKind::WouldBlock {
            return Err(error.into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

pub(crate) async fn get_or_load_session(
    planner_service: &Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSession, AppError> {
    // Every caller holds the session guard. Re-read after acquiring it so a
    // different process's completed mutation cannot be overwritten by a cache.
    let loaded = load_session_from_db(db, session_id).await?;
    planner_service
        .lock()
        .await
        .save_session(session_id, loaded.clone());
    Ok(loaded)
}

pub(crate) async fn load_session_from_db(
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSession, AppError> {
    let record = planner_repo::get_session(db, session_id).await?;
    let conversation = planner_repo::list_conversation_entries(db, session_id)
        .await?
        .into_iter()
        .map(|entry| PlannerConversationEntry {
            role: entry.role,
            content: entry.content,
        })
        .collect::<Vec<_>>();
    let pending_plan = match record.pending_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerPlan>(&value)?),
        None => None,
    };
    let draft_plan = match record.draft_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerDraftPlan>(&value)?),
        None => None,
    };
    Ok(PlannerSession {
        provider_id: record.provider_id,
        model_name: record.model_name,
        pending_plan,
        draft_plan,
        selected_draft_node_id: record.selected_draft_node_id,
        conversation,
    })
}

pub(crate) async fn persist_pending_plan(
    db: &SqlitePool,
    session_id: &str,
    pending_plan: Option<&PlannerPlan>,
) -> Result<(), AppError> {
    let serialized = pending_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_pending_plan(db, session_id, serialized.as_deref()).await?;
    Ok(())
}

pub(crate) async fn persist_draft_state(
    db: &SqlitePool,
    session_id: &str,
    draft_plan: Option<&PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
) -> Result<(), AppError> {
    let serialized = draft_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_draft_state(
        db,
        session_id,
        serialized.as_deref(),
        selected_draft_node_id,
    )
    .await?;
    Ok(())
}

pub(crate) async fn persist_manual_draft_state(
    db: &SqlitePool,
    session_id: &str,
    draft_plan: Option<&PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
) -> Result<(), AppError> {
    let serialized = draft_plan.map(serde_json::to_string).transpose()?;
    planner_repo::replace_draft_and_clear_pending(db, session_id, serialized.as_deref(), selected_draft_node_id).await
}

pub(crate) async fn append_conversation(
    db: &SqlitePool,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), AppError> {
    planner_repo::append_conversation_entry(
        db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        role,
        content,
    )
    .await?;
    Ok(())
}

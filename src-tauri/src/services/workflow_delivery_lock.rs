//! Serialize cooperating delivery attempts for a workflow run.
use crate::error::AppError;
use crate::persistence::workflow_repo;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};

pub(crate) struct WorkflowDeliveryGuard {
    _task: OwnedMutexGuard<()>,
    #[cfg(unix)]
    _file: Option<std::fs::File>,
}

pub(crate) async fn acquire(db: &SqlitePool, run_id: &str) -> Result<WorkflowDeliveryGuard, AppError> {
    static LOCKS: OnceLock<std::sync::Mutex<HashMap<String, Weak<Mutex<()>>>>> = OnceLock::new();
    let lock = {
        let mut locks = LOCKS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
            .lock().map_err(|_| AppError::Internal("Workflow delivery lock registry failed".into()))?;
        locks.retain(|_, lock| lock.strong_count() > 0);
        match locks.get(run_id).and_then(Weak::upgrade) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(Mutex::new(()));
                locks.insert(run_id.to_string(), Arc::downgrade(&lock));
                lock
            }
        }
    };
    let task = lock.lock_owned().await;
    // Avoid creating files for arbitrary nonexistent run IDs.
    workflow_repo::get_workflow_run(db, run_id).await?;
    #[cfg(unix)]
    let file = crate::services::planner_session::acquire_database_file_lock(db, "workflow-delivery", run_id).await?;
    Ok(WorkflowDeliveryGuard { _task: task, #[cfg(unix)] _file: file })
}

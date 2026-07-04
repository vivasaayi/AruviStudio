use crate::domain::agent_work::{AgentWorkCatalogLinkResult, AgentWorkMaterializationResult};
use crate::error::AppError;
pub use crate::persistence::agent_work_batch_repo::complete_batch;
pub use crate::persistence::agent_work_claim_repo::claim_next_item;
pub use crate::persistence::agent_work_dependency_repo::{
    delete_dependency, list_dependencies, list_ready_items, upsert_dependency,
};
pub use crate::persistence::agent_work_event_repo::{
    append_event, append_evidence, list_events, list_evidence, AppendAgentWorkEventInput,
    AppendAgentWorkEvidenceInput,
};
pub use crate::persistence::agent_work_item_repo::{
    get_item, list_items, upsert_item, UpsertAgentWorkItemInput,
};
pub use crate::persistence::agent_work_lifecycle_repo::{
    heartbeat_item, link_commit, release_item_locks, requeue_expired_items, requeue_item,
    update_item_status, UpdateAgentWorkItemStatusInput,
};
pub use crate::persistence::agent_work_lock_repo::{
    inspect_conflict_zone, list_active_locks, list_conflict_zones, release_conflict_zone,
    reserve_conflict_zone, ReserveConflictZoneInput,
};
pub use crate::persistence::agent_work_run_repo::{
    get_run, get_run_health, get_run_summary, list_agent_activity, list_runs, upsert_run,
    UpsertAgentWorkRunInput,
};
pub(crate) use crate::persistence::agent_work_status::normalize_batch_status;
use serde_json::Value;
use sqlx::SqlitePool;

pub async fn materialize_catalog(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    create_work_items: bool,
) -> Result<AgentWorkMaterializationResult, AppError> {
    crate::persistence::agent_work_catalog_repo::materialize_catalog(
        pool,
        run_id,
        product_id,
        create_work_items,
    )
    .await
}

pub async fn link_catalog_work_items(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    sync_statuses: bool,
) -> Result<AgentWorkCatalogLinkResult, AppError> {
    crate::persistence::agent_work_catalog_repo::link_catalog_work_items(
        pool,
        run_id,
        product_id,
        sync_statuses,
    )
    .await
}

pub async fn import_legacy_checkpoint(
    pool: &SqlitePool,
    checkpoint_path: &str,
    run_id: Option<&str>,
    source_label: Option<&str>,
) -> Result<Value, AppError> {
    crate::persistence::agent_work_import_repo::import_legacy_checkpoint(
        pool,
        checkpoint_path,
        run_id,
        source_label,
    )
    .await
}

#[cfg(test)]
mod tests;

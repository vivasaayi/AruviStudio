use crate::error::AppError;
use crate::persistence::planner_repo;
#[cfg(test)]
pub(crate) use crate::services::planner_commit::commit_draft_plan;
#[cfg(test)]
pub(crate) use crate::services::planner_draft::build_draft_tree_nodes;
#[cfg(test)]
pub(crate) use crate::services::planner_draft_apply::apply_actions_to_draft;
pub use crate::services::planner_draft_commands::{
    add_planner_draft_child, delete_planner_draft_node, rename_planner_draft_node,
};
#[cfg(test)]
pub(crate) use crate::services::planner_draft_mutation::{
    add_draft_child_node, delete_draft_node, rename_draft_node,
};
use crate::services::planner_model::resolve_planner_model_binding;
pub(crate) use crate::services::planner_session::persist_draft_state;
use crate::services::planner_session::persist_pending_plan;
pub use crate::services::planner_session::{PlannerService, PlannerSessionInfo};
pub use crate::services::planner_text_turn::{confirm_planner_plan, submit_planner_turn};
pub use crate::services::planner_types::{
    PlannerDraftNode, PlannerDraftPlan, PlannerPlan, PlannerTraceEvent, PlannerTreeNode,
    PlannerTurnResponse,
};
use crate::state::AppState;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn create_planner_session(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let (provider_id, model_name) =
        resolve_planner_model_binding(db, provider_id, model_name).await?;
    let mut service = planner_service.lock().await;
    let info = service.create_session(provider_id.clone(), model_name.clone());
    planner_repo::create_session(
        db,
        &info.session_id,
        provider_id.as_deref(),
        model_name.as_deref(),
    )
    .await?;
    Ok(info)
}

pub async fn update_planner_session(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let (provider_id, model_name) =
        resolve_planner_model_binding(db, provider_id, model_name).await?;
    let mut service = planner_service.lock().await;
    let info = service.update_session(&session_id, provider_id.clone(), model_name.clone())?;
    planner_repo::update_session(
        db,
        &session_id,
        provider_id.as_deref(),
        model_name.as_deref(),
    )
    .await?;
    Ok(info)
}

pub async fn clear_planner_pending(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
) -> Result<PlannerSessionInfo, AppError> {
    let mut service = planner_service.lock().await;
    let info = service.clear_pending(&session_id)?;
    persist_pending_plan(db, &session_id, None).await?;
    persist_draft_state(db, &session_id, None, None).await?;
    Ok(info)
}

pub async fn submit_planner_voice_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    transcript: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    crate::services::planner_voice_turn::submit_planner_voice_turn(
        planner_service,
        state,
        session_id,
        transcript,
        selected_draft_node_id,
        selected_product_id,
    )
    .await
}

pub async fn analyze_repository_for_planner(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    artifact_base_path: &Path,
    session_id: String,
    repository_id: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    crate::services::planner_repository_turn::analyze_repository_for_planner(
        planner_service,
        db,
        artifact_base_path,
        session_id,
        repository_id,
        selected_draft_node_id,
        selected_product_id,
    )
    .await
}

#[cfg(test)]
mod tests;

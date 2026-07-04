pub(super) use super::{
    add_draft_child_node, apply_actions_to_draft, build_draft_tree_nodes, commit_draft_plan,
    create_planner_session, delete_draft_node, persist_draft_state, rename_draft_node,
    submit_planner_voice_turn, PlannerDraftPlan,
};
use crate::persistence::db as db_service;
use crate::services::planner_action_parser::normalize_planner_action;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};

mod action_application;
mod draft_mutations;
mod planner_turns;
mod repository_analysis;

fn planner_test_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

async fn acquire_planner_test_lock() -> OwnedMutexGuard<()> {
    planner_test_lock().lock_owned().await
}

fn make_temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "aruvi_planner_service_{}_{}",
        name,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

async fn make_test_state(name: &str) -> AppState {
    let temp_root = make_temp_dir(name);
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");
    AppState::new(pool, temp_root)
        .await
        .expect("failed to create app state")
}

fn normalize_actions(values: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    values
        .into_iter()
        .filter_map(normalize_planner_action)
        .collect::<Vec<_>>()
}

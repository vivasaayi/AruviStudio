use super::{
    add_draft_child_node, apply_actions_to_draft, build_draft_tree_nodes, commit_draft_plan,
    create_planner_session, delete_draft_node, persist_draft_state, rename_draft_node,
    submit_planner_voice_turn, PlannerDraftPlan,
};
use crate::domain::repository::Repository;
use crate::persistence::{db as db_service, product_repo, work_item_repo};
use crate::services::planner_action_parser::normalize_planner_action;
use crate::services::planner_repository_analysis::{
    build_repository_analysis_snapshot, RepositoryAnalysisSnapshot,
};
use crate::state::AppState;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};

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

fn make_repository(temp_root: &Path, name: &str) -> Repository {
    Repository {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        local_path: temp_root.display().to_string(),
        remote_url: "".to_string(),
        default_branch: "main".to_string(),
        auth_profile: None,
        created_at: "2026-03-21 00:00:00".to_string(),
        updated_at: "2026-03-21 00:00:00".to_string(),
    }
}

#[test]
fn normalize_planner_action_repairs_relaxed_model_shapes() {
    let normalized = normalize_planner_action(json!({
        "type": "create_work_item",
        "target": "Guest Profile Management",
        "work_item_name": "Implement Guest Profile CRUD",
        "description": "Build CRUD flows."
    }))
    .expect("action should normalize");

    assert_eq!(
        normalized.get("title").and_then(serde_json::Value::as_str),
        Some("Implement Guest Profile CRUD")
    );
    assert_eq!(
        normalized
            .get("target")
            .and_then(|value| value.get("capabilityName"))
            .and_then(serde_json::Value::as_str),
        Some("Guest Profile Management")
    );
}

#[test]
fn build_repository_analysis_snapshot_extracts_structured_signals() {
    let temp_root = make_temp_dir("repo_analysis_snapshot");
    fs::create_dir_all(temp_root.join("src/features/planner"))
        .expect("failed to create feature dir");
    fs::create_dir_all(temp_root.join("app/hotels")).expect("failed to create route dir");
    fs::create_dir_all(temp_root.join("e2e")).expect("failed to create e2e dir");
    fs::write(
        temp_root.join("README.md"),
        "# Hotel Management System\n## Planner\nInteractive planning workspace.",
    )
    .expect("failed to write README");
    fs::write(
            temp_root.join("package.json"),
            r#"{
              "name": "hotel-management-system",
              "scripts": { "dev": "vite", "test:e2e": "playwright test" },
              "dependencies": { "react": "^18.0.0", "vite": "^5.0.0", "@tanstack/react-query": "^5.0.0" },
              "devDependencies": { "@playwright/test": "^1.0.0" }
            }"#,
        )
        .expect("failed to write package.json");
    fs::write(
        temp_root.join("src/features/planner/PlannerPage.tsx"),
        "export function PlannerPage() { return null; }",
    )
    .expect("failed to write planner page");
    fs::write(
        temp_root.join("app/hotels/page.tsx"),
        "export default function Hotels() { return null; }",
    )
    .expect("failed to write route");
    fs::write(
        temp_root.join("e2e/planner.spec.ts"),
        "test('planner', () => {});",
    )
    .expect("failed to write test");

    let snapshot: RepositoryAnalysisSnapshot =
        build_repository_analysis_snapshot(&make_repository(&temp_root, "hotel-management-system"))
            .expect("snapshot should build");

    assert!(
        !snapshot.manifests.is_empty(),
        "manifest signals should be extracted"
    );
    assert!(!snapshot.docs.is_empty(), "doc signals should be extracted");
    assert!(
        !snapshot.routes.is_empty(),
        "route signals should be extracted"
    );
    assert!(
        !snapshot.tests.is_empty(),
        "test signals should be extracted"
    );
    assert!(
        snapshot
            .candidate_areas
            .iter()
            .any(|area| area.name.contains("Planner") || area.name.contains("Hotels")),
        "candidate areas should include feature or route-derived areas"
    );
}

#[tokio::test]
async fn apply_actions_to_draft_supports_selected_node_refinement_flow() {
    let _guard = acquire_planner_test_lock().await;

    let create_root = normalize_actions(vec![json!({
        "type": "create_product",
        "target": "Hotel Management System",
        "name": "Hotel Management System",
        "description": "Hotel operations root."
    })]);
    let draft =
        apply_actions_to_draft(None, None, &create_root).expect("failed to create root draft");
    let product_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .expect("missing product node")
        .id
        .clone();

    let add_product_area = normalize_actions(vec![json!({
        "type": "create_product_area",
        "name": "Guest Management",
        "description": "Guest workflows."
    })]);
    let draft = apply_actions_to_draft(Some(draft), Some(&product_id), &add_product_area)
        .expect("failed to add product_area");
    let product_area = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Guest Management")
        .expect("missing product_area node");
    let product_area_id = product_area.id.clone();
    assert_eq!(product_area.parent_id.as_deref(), Some(product_id.as_str()));

    let add_capability = normalize_actions(vec![json!({
        "type": "create_capability",
        "name": "Guest Profile Management",
        "description": "Profiles and preferences."
    })]);
    let draft = apply_actions_to_draft(Some(draft), Some(&product_area_id), &add_capability)
        .expect("failed to add capability");
    let capability = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "capability" && node.name == "Guest Profile Management")
        .expect("missing capability node");
    let capability_id = capability.id.clone();
    assert_eq!(
        capability.parent_id.as_deref(),
        Some(product_area_id.as_str())
    );

    let add_work_item = normalize_actions(vec![json!({
        "type": "create_work_item",
        "work_item_name": "Implement Guest Profile CRUD",
        "description": "Backend and frontend CRUD."
    })]);
    let draft = apply_actions_to_draft(Some(draft), Some(&capability_id), &add_work_item)
        .expect("failed to add work item");
    let work_item = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "work_item" && node.name == "Implement Guest Profile CRUD")
        .expect("missing work item node");
    assert_eq!(work_item.parent_id.as_deref(), Some(capability_id.as_str()));
}

#[tokio::test]
async fn draft_tree_nodes_surface_repository_analysis_metadata() {
    let _guard = acquire_planner_test_lock().await;
    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "name": "Hotel Management System",
            "description": "Hotel root.",
            "analysis": {
                "source": "repository_analysis",
                "confidence": "high",
                "evidence": [
                    "doc: README.md -> Hotel Management System",
                    "manifest: package.json -> hotel-management-system (React, Vite)"
                ]
            }
        }),
        json!({
            "type": "create_product_area",
            "target": { "productName": "Hotel Management System" },
            "name": "Interactive Planner",
            "description": "Conversational planning surface.",
            "analysis": {
                "source": "repository_analysis",
                "confidence": "medium",
                "evidence": [
                    "path: src/features/planner/PlannerPage.tsx"
                ]
            }
        }),
    ]);

    let draft = apply_actions_to_draft(None, None, &actions).expect("failed to build draft");
    let tree = build_draft_tree_nodes(&draft, None);
    let product = tree.first().expect("product node should exist");
    let product_area = product
        .children
        .first()
        .expect("product_area node should exist");

    assert_eq!(product.source.as_deref(), Some("repository_analysis"));
    assert_eq!(product.confidence.as_deref(), Some("high"));
    assert!(product
        .evidence
        .iter()
        .any(|line| line.contains("README.md")));
    assert_eq!(product_area.source.as_deref(), Some("repository_analysis"));
    assert_eq!(
        product_area.summary.as_deref(),
        Some("Conversational planning surface.")
    );
}

#[tokio::test]
async fn apply_actions_to_draft_handles_relaxed_nested_targets_from_trace() {
    let _guard = acquire_planner_test_lock().await;

    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "target": "Hotel Management System",
            "name": "Hotel Management System",
            "description": "Hotel root."
        }),
        json!({
            "type": "create_product_area",
            "target": "Hotel Management System",
            "product_area_name": "Guest Management",
            "description": "Guest workflows."
        }),
        json!({
            "type": "create_capability",
            "target": "Guest Management",
            "capability_name": "Guest Profile Management",
            "description": "Profiles."
        }),
        json!({
            "type": "create_work_item",
            "target": "Guest Profile Management",
            "work_item_name": "Implement Guest Profile CRUD",
            "description": "Build CRUD."
        }),
    ]);

    let draft = apply_actions_to_draft(None, None, &actions)
        .expect("failed to apply relaxed nested actions");

    let product = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product" && node.name == "Hotel Management System")
        .expect("missing product");
    let product_area = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Guest Management")
        .expect("missing product_area");
    let capability = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "capability" && node.name == "Guest Profile Management")
        .expect("missing capability");
    let work_item = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "work_item" && node.name == "Implement Guest Profile CRUD")
        .expect("missing work item");

    assert_eq!(product_area.parent_id.as_deref(), Some(product.id.as_str()));
    assert_eq!(
        capability.parent_id.as_deref(),
        Some(product_area.id.as_str())
    );
    assert_eq!(work_item.parent_id.as_deref(), Some(capability.id.as_str()));
}

#[tokio::test]
async fn apply_actions_to_draft_infers_product_from_product_area_target_on_follow_up() {
    let _guard = acquire_planner_test_lock().await;

    let seed_actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "name": "Library Management System",
            "description": "Library root."
        }),
        json!({
            "type": "create_product_area",
            "target": { "productName": "Library Management System" },
            "name": "Circulation",
            "description": "Loans and returns."
        }),
    ]);

    let draft = apply_actions_to_draft(None, None, &seed_actions).expect("failed to seed draft");

    let follow_up_actions = normalize_actions(vec![json!({
        "type": "create_product_area",
        "target": { "productAreaName": "Circulation" },
        "name": "Notifications",
        "description": "Email and WhatsApp alerts for due dates."
    })]);

    let draft = apply_actions_to_draft(Some(draft), None, &follow_up_actions)
        .expect("failed to infer sibling product_area product");

    let product = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product" && node.name == "Library Management System")
        .expect("missing product");
    let notifications = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Notifications")
        .expect("missing notifications product_area");

    assert_eq!(
        notifications.parent_id.as_deref(),
        Some(product.id.as_str())
    );
}

#[tokio::test]
async fn submit_planner_voice_turn_selects_draft_nodes() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("voice_selects_draft_node").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await
        .expect("failed to create planner session");

    let draft = apply_actions_to_draft(
        None,
        None,
        &normalize_actions(vec![
            json!({
                "type": "create_product",
                "name": "Hotel Management System",
                "description": "Hotel root."
            }),
            json!({
                "type": "create_product_area",
                "target": { "productName": "Hotel Management System" },
                "name": "Guest Management",
                "description": "Guest workflows."
            }),
        ]),
    )
    .expect("failed to build draft");
    let product_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .expect("missing product")
        .id
        .clone();
    let product_area_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Guest Management")
        .expect("missing product_area")
        .id
        .clone();

    {
        let mut service = state.planner_service.lock().await;
        let mut loaded = service
            .get_session(&session.session_id)
            .expect("planner session should exist");
        loaded.draft_plan = Some(draft.clone());
        loaded.selected_draft_node_id = Some(product_id.clone());
        service.save_session(&session.session_id, loaded);
    }
    persist_draft_state(
        &state.db,
        &session.session_id,
        Some(&draft),
        Some(product_id.as_str()),
    )
    .await
    .expect("failed to persist draft state");

    let response = submit_planner_voice_turn(
        state.planner_service.clone(),
        &state,
        session.session_id.clone(),
        "select product area guest management".to_string(),
        None,
        None,
    )
    .await
    .expect("voice turn should succeed");

    assert_eq!(response.status, "session_update");
    assert_eq!(
        response.selected_draft_node_id.as_deref(),
        Some(product_area_id.as_str())
    );
    assert!(response
        .assistant_message
        .contains("Selected product area \"Guest Management\""));
    assert!(response.draft_tree_nodes.is_some());
}

#[tokio::test]
async fn commit_draft_plan_persists_tree_structure() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("commit_draft").await;

    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "target": "Hotel Management System",
            "name": "Hotel Management System",
            "description": "Hotel root."
        }),
        json!({
            "type": "create_product_area",
            "target": "Hotel Management System",
            "product_area_name": "Guest Management",
            "description": "Guest workflows."
        }),
        json!({
            "type": "create_capability",
            "target": "Guest Management",
            "capability_name": "Guest Profile Management",
            "description": "Profiles."
        }),
        json!({
            "type": "create_work_item",
            "target": "Guest Profile Management",
            "work_item_name": "Implement Guest Profile CRUD",
            "description": "Build CRUD."
        }),
    ]);
    let draft: PlannerDraftPlan =
        apply_actions_to_draft(None, None, &actions).expect("failed to build draft");

    let execution = commit_draft_plan(&state, &draft)
        .await
        .expect("failed to commit draft");
    assert!(!execution.is_empty());

    let products = product_repo::list_products(&state.db)
        .await
        .expect("failed to list products");
    let product = products
        .iter()
        .find(|product| product.name == "Hotel Management System")
        .expect("committed product not found");

    let product_areas = product_repo::list_product_areas(&state.db, &product.id)
        .await
        .expect("failed to list product_areas");
    let product_area = product_areas
        .iter()
        .find(|product_area| product_area.name == "Guest Management")
        .expect("committed product_area not found");

    let tree = product_repo::get_product_tree(&state.db, &product.id)
        .await
        .expect("failed to load product tree");
    let capability = tree
        .product_areas
        .iter()
        .find(|product_area_tree| product_area_tree.product_area.id == product_area.id)
        .and_then(|product_area_tree| product_area_tree.features.first())
        .map(|feature| feature.capability.name.clone());
    assert_eq!(capability.as_deref(), Some("Guest Profile Management"));

    let work_items = work_item_repo::list_work_items_page(
        &state.db,
        work_item_repo::WorkItemListQuery {
            product_id: Some(&product.id),
            limit: Some(20),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("failed to list work items");
    assert!(work_items
        .iter()
        .any(|item| item.title == "Implement Guest Profile CRUD"));
}

#[tokio::test]
async fn rename_draft_node_updates_descendant_targets() {
    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "name": "Hotel Management System",
            "description": "Hotel root."
        }),
        json!({
            "type": "create_product_area",
            "target": { "productName": "Hotel Management System" },
            "name": "Guest Management",
            "description": "Guest workflows."
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": "Hotel Management System",
                "productAreaName": "Guest Management"
            },
            "name": "Guest Profile Management",
            "description": "Profiles."
        }),
        json!({
            "type": "create_work_item",
            "target": {
                "productName": "Hotel Management System",
                "productAreaName": "Guest Management",
                "capabilityName": "Guest Profile Management"
            },
            "title": "Implement Guest Profile CRUD",
            "description": "Build CRUD."
        }),
    ]);

    let mut draft = apply_actions_to_draft(None, None, &actions).expect("failed to create draft");
    let product_area_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Guest Management")
        .map(|node| node.id.clone())
        .expect("product_area should exist");

    let renamed = rename_draft_node(&mut draft, &product_area_id, "Guest Operations")
        .expect("rename should succeed");

    assert_eq!(renamed.name, "Guest Operations");
    let capability = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "capability")
        .expect("capability should exist");
    let work_item = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "work_item")
        .expect("work item should exist");

    assert_eq!(
        capability
            .details
            .get("target")
            .and_then(|value| value.get("productAreaName"))
            .and_then(serde_json::Value::as_str),
        Some("Guest Operations")
    );
    assert_eq!(
        work_item
            .details
            .get("target")
            .and_then(|value| value.get("productAreaName"))
            .and_then(serde_json::Value::as_str),
        Some("Guest Operations")
    );
}

#[tokio::test]
async fn add_and_delete_draft_child_nodes_preserve_hierarchy() {
    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "name": "Hotel Management System",
            "description": "Hotel root."
        }),
        json!({
            "type": "create_product_area",
            "target": { "productName": "Hotel Management System" },
            "name": "Billing & Payments",
            "description": "Billing workflows."
        }),
    ]);

    let mut draft = apply_actions_to_draft(None, None, &actions).expect("failed to create draft");
    let product_area_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Billing & Payments")
        .map(|node| node.id.clone())
        .expect("product_area should exist");

    let capability = add_draft_child_node(
        &mut draft,
        &product_area_id,
        "capability",
        "Notification Preferences",
        Some("Manage guest delivery preferences."),
    )
    .expect("capability should be created");
    let work_item = add_draft_child_node(
        &mut draft,
        &capability.id,
        "work_item",
        "Build Preference Capture Form",
        Some("Add guest preference controls."),
    )
    .expect("work item should be created");

    assert_eq!(
        capability.parent_id.as_deref(),
        Some(product_area_id.as_str())
    );
    assert_eq!(work_item.parent_id.as_deref(), Some(capability.id.as_str()));

    let (_, fallback_parent_id) =
        delete_draft_node(&mut draft, &capability.id).expect("delete should succeed");

    assert_eq!(
        fallback_parent_id.as_deref(),
        Some(product_area_id.as_str())
    );
    assert!(draft
        .nodes
        .iter()
        .all(|node| node.name != "Notification Preferences"
            && node.name != "Build Preference Capture Form"));
}

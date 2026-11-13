use super::*;
use crate::persistence::{product_repo, work_item_repo};
use serde_json::json;

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
async fn concurrent_draft_edits_are_serialized_per_session() {
    use crate::services::planner_draft_commands::add_planner_draft_child;
    use crate::services::planner_session::{load_session_from_db, lock_planner_session};
    use std::time::Duration;

    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("concurrent_draft_edits").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await
        .unwrap();
    let draft = apply_actions_to_draft(
        None,
        None,
        &normalize_actions(vec![json!({
            "type": "create_product", "name": "Concurrent Product", "description": "Test root"
        })]),
    )
    .unwrap();
    let root_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .unwrap()
        .id
        .clone();
    {
        let mut service = state.planner_service.lock().await;
        let mut current = service.get_session(&session.session_id).unwrap();
        current.draft_plan = Some(draft.clone());
        service.save_session(&session.session_id, current);
    }
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None)
        .await
        .unwrap();

    let held = lock_planner_session(&state.planner_service, &session.session_id).await;
    let other = tokio::time::timeout(
        Duration::from_secs(1),
        lock_planner_session(&state.planner_service, "another-session"),
    )
    .await
    .expect("another session should not be blocked");
    drop(other);
    let mut edits = Vec::new();
    for name in ["First Area", "Second Area"] {
        let service = state.planner_service.clone();
        let db = state.db.clone();
        let id = session.session_id.clone();
        let root = root_id.clone();
        edits.push(tokio::spawn(async move {
            add_planner_draft_child(
                service,
                &db,
                id,
                root,
                "product_area".into(),
                name.into(),
                None,
            )
            .await
        }));
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        edits.iter().all(|edit| !edit.is_finished()),
        "edits must wait for the session lock"
    );
    drop(held);
    for edit in edits {
        edit.await.unwrap().unwrap();
    }
    let persisted = load_session_from_db(&state.db, &session.session_id)
        .await
        .unwrap();
    let draft = persisted.draft_plan.unwrap();
    for name in ["First Area", "Second Area"] {
        assert!(
            draft.nodes.iter().any(|node| node.name == name),
            "missing {name}"
        );
    }
}

#[tokio::test]
async fn explicit_planner_model_never_falls_back_to_another_provider() {
    use crate::persistence::model_repo;
    use crate::services::planner_model::resolve_planner_model_binding;

    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("explicit_planner_binding").await;
    model_repo::create_provider(
        &state.db,
        "planner-test-provider",
        "Planner Test",
        "openai_compatible",
        "http://localhost:12345/v1",
        None,
    )
    .await
    .unwrap();
    model_repo::create_model_definition(
        &state.db,
        "planner-test-model",
        "planner-test-provider",
        "planner-test-name",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let resolve = |provider: Option<&str>, model: Option<&str>| {
        resolve_planner_model_binding(
            &state.db,
            provider.map(str::to_owned),
            model.map(str::to_owned),
        )
    };
    assert_eq!(
        resolve(Some("planner-test-provider"), Some("planner-test-name"))
            .await
            .unwrap(),
        (
            Some("planner-test-provider".into()),
            Some("planner-test-name".into())
        )
    );
    assert!(
        resolve(Some("planner-test-provider"), Some("missing-model"))
            .await
            .is_err()
    );
    assert!(resolve(Some("missing-provider"), Some("planner-test-name"))
        .await
        .is_err());
    assert!(resolve(Some("missing-provider"), None).await.is_err());
    assert!(resolve(None, Some("missing-model")).await.is_err());
    assert_eq!(
        resolve(Some("planner-test-provider"), None).await.unwrap(),
        (
            Some("planner-test-provider".into()),
            Some("planner-test-name".into())
        )
    );
}

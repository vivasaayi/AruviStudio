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

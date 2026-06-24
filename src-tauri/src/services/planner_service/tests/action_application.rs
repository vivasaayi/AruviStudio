use super::*;
use serde_json::json;

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

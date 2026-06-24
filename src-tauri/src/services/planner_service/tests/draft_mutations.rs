use super::*;
use serde_json::json;

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

use super::*;
use crate::persistence::{db as db_service, product_repo, work_item_repo};
use crate::state::AppState;

mod definition_discovery;

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aruvi_mcp_tools_{}_{}", name, uuid::Uuid::new_v4()))
}

async fn create_test_state(name: &str) -> AppState {
    let temp_root = make_temp_dir(name);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let db_path = temp_root.join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());
    let pool = db_service::create_pool(&database_url)
        .await
        .expect("test database should be created");
    AppState::new(pool, temp_root)
        .await
        .expect("test app state should be created")
}

async fn create_test_product(state: &AppState, product_id: &str) {
    product_repo::create_product(
        &state.db,
        product_repo::CreateProductInput {
            id: product_id,
            name: "MCP Test Product",
            description: "",
            vision: "",
            goals: "[]",
            tags: "[]",
            lifecycle: Some("active"),
            health: Some("healthy"),
            owner_label: None,
            investment_status: Some("invest"),
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("product should be created");
}

async fn create_test_work_item(state: &AppState, id: &str, product_id: &str, title: &str) {
    work_item_repo::create_work_item(
        &state.db,
        work_item_repo::CreateWorkItemInput {
            id,
            product_id,
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title,
            problem_statement: "",
            description: "",
            acceptance_criteria: "",
            constraints: "",
            work_item_type: "story",
            priority: "medium",
            complexity: "medium",
        },
    )
    .await
    .expect("work item should be created");
}

#[tokio::test]
async fn work_items_list_supports_opt_in_pagination_metadata() {
    let state = create_test_state("work_item_list_pagination").await;
    create_test_product(&state, "mcp-product-page").await;
    for (id, title) in [
        ("mcp-page-story-1", "First"),
        ("mcp-page-story-2", "Second"),
        ("mcp-page-story-3", "Third"),
    ] {
        create_test_work_item(&state, id, "mcp-product-page", title).await;
    }

    let raw_response = dispatch_tool(
        &state,
        "work_items.list",
        json!({
            "productId": "mcp-product-page",
            "limit": 2
        }),
    )
    .await
    .expect("legacy raw page should load");

    assert_eq!(raw_response["action"], json!("list_work_items"));
    assert_eq!(
        raw_response["result"]
            .as_array()
            .expect("legacy result should remain an array")
            .len(),
        2
    );

    let paged_response = dispatch_tool(
        &state,
        "work_items.list",
        json!({
            "productId": "mcp-product-page",
            "limit": 2,
            "offset": 0,
            "includePagination": true
        }),
    )
    .await
    .expect("metadata page should load");

    assert_eq!(
        paged_response["result"]["workItems"]
            .as_array()
            .expect("metadata result should include workItems")
            .len(),
        2
    );
    assert_eq!(
        paged_response["result"]["pagination"],
        json!({
            "limit": 2,
            "offset": 0,
            "returned": 2,
            "hasMore": true,
            "nextOffset": 2,
        })
    );
}

#[test]
fn translate_first_class_tool_wraps_action_payload_for_legacy_handlers() {
    let translated = translate_first_class_tool(
        "catalog.product_areas.create",
        json!({
            "productId": "product-123",
            "name": "Runtime Model"
        }),
    )
    .expect("translation should succeed")
    .expect("known first-class tool");

    assert_eq!(translated.0, "aruvi_catalog");
    assert_eq!(
        translated.1,
        json!({
            "action": "create_product_area",
            "arguments": {
                "productId": "product-123",
                "name": "Runtime Model"
            }
        })
    );
}

#[test]
fn translate_bulk_import_first_class_tool_wraps_action_payload() {
    let translated = translate_first_class_tool(
        "catalog.bulk_import.submit",
        json!({
            "filePath": "/tmp/import.json",
            "format": "json",
            "productId": "product-123"
        }),
    )
    .expect("translation should succeed")
    .expect("known bulk import tool");

    assert_eq!(translated.0, "aruvi_catalog");
    assert_eq!(
        translated.1,
        json!({
            "action": "submit_bulk_import",
            "arguments": {
                "filePath": "/tmp/import.json",
                "format": "json",
                "productId": "product-123"
            }
        })
    );
}

#[test]
fn translate_agent_work_first_class_tool_wraps_action_payload() {
    let translated = translate_first_class_tool(
        "agent_work.items.claim_next",
        json!({
            "runId": "run-123",
            "agent": "agent-a"
        }),
    )
    .expect("translation should succeed")
    .expect("known first-class tool");

    assert_eq!(translated.0, "aruvi_agent_work");
    assert_eq!(
        translated.1,
        json!({
            "action": "claim_next_item",
            "arguments": {
                "runId": "run-123",
                "agent": "agent-a"
            }
        })
    );
}

#[test]
fn translate_feature_context_tool_wraps_action_payload() {
    let translated = translate_first_class_tool(
        "agent_work.context.get_feature",
        json!({
            "featureId": "feature-123",
            "runId": "run-123"
        }),
    )
    .expect("translation should succeed")
    .expect("known context tool");

    assert_eq!(translated.0, "aruvi_agent_work");
    assert_eq!(
        translated.1,
        json!({
            "action": "get_feature_context",
            "arguments": {
                "featureId": "feature-123",
                "runId": "run-123"
            }
        })
    );
}

#[test]
fn translate_link_catalog_work_items_tool_wraps_action_payload() {
    let translated = translate_first_class_tool(
        "agent_work.link_catalog_work_items",
        json!({
            "runId": "run-123",
            "productId": "product-123",
            "syncStatuses": true
        }),
    )
    .expect("translation should succeed")
    .expect("known link tool");

    assert_eq!(translated.0, "aruvi_agent_work");
    assert_eq!(
        translated.1,
        json!({
            "action": "link_catalog_work_items",
            "arguments": {
                "runId": "run-123",
                "productId": "product-123",
                "syncStatuses": true
            }
        })
    );
}

#[test]
fn translate_materialize_catalog_tool_wraps_action_payload() {
    let translated = translate_first_class_tool(
        "agent_work.materialize_catalog",
        json!({
            "runId": "run-001",
            "productId": "mayyam",
            "createWorkItems": true
        }),
    )
    .expect("translation should succeed")
    .expect("known materialization tool");

    assert_eq!(translated.0, "aruvi_agent_work");
    assert_eq!(
        translated.1,
        json!({
            "action": "materialize_catalog",
            "arguments": {
                "runId": "run-001",
                "productId": "mayyam",
                "createWorkItems": true
            }
        })
    );
}

#[test]
fn translate_product_area_tool_names_for_catalog() {
    let translated = translate_first_class_tool(
        "catalog.product_areas.create",
        json!({
            "productId": "product-123",
            "name": "Runtime Model"
        }),
    )
    .expect("translation should succeed")
    .expect("known compatibility tool");

    assert_eq!(translated.0, "aruvi_catalog");
    assert_eq!(
        translated.1,
        json!({
            "action": "create_product_area",
            "arguments": {
                "productId": "product-123",
                "name": "Runtime Model"
            }
        })
    );
}

#[test]
fn translate_first_class_tool_rejects_non_object_arguments() {
    let error = translate_first_class_tool("work_items.list", json!("bad payload"))
        .expect_err("translation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert_eq!(
        error.to_string(),
        "Validation error: work_items.list arguments must be a JSON object"
    );
}

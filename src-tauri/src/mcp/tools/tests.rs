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
async fn catalog_exposes_desktop_capability_reads_and_tree_summary() {
    let state = create_test_state("catalog_read_parity").await;
    create_test_product(&state, "catalog-product-a").await;
    create_test_product(&state, "catalog-product-b").await;
    let area = dispatch_tool(&state, "catalog.product_areas.create", json!({
        "productId": "catalog-product-a", "name": "Inventory"
    })).await.unwrap();
    let area_id = area["result"]["id"].as_str().unwrap();
    let capability = dispatch_tool(&state, "catalog.capabilities.create", json!({
        "productAreaId": area_id, "name": "Stock", "nodeKind": "capability"
    })).await.unwrap();
    let capability_id = capability["result"]["id"].as_str().unwrap();
    let feature = dispatch_tool(&state, "catalog.capabilities.create", json!({
        "productAreaId": area_id, "parentCapabilityId": capability_id,
        "name": "Updates", "nodeKind": "feature"
    })).await.unwrap();
    let feature_id = feature["result"]["id"].as_str().unwrap();

    let get = dispatch_tool(&state, "catalog.capabilities.get", json!({"id": feature_id})).await.unwrap();
    assert_eq!(get["result"]["id"], feature_id);
    let legacy_get = dispatch_tool(&state, "aruvi_catalog", json!({
        "action": "get_capability", "arguments": {"id": feature_id}
    })).await.unwrap();
    assert_eq!(get["result"], legacy_get["result"]);
    assert!(dispatch_tool(&state, "catalog.capabilities.get", json!({"id": "missing"})).await.is_err());

    let list = dispatch_tool(&state, "catalog.capabilities.list_for_product", json!({"productId": "catalog-product-a"})).await.unwrap();
    let rows = list["result"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], capability_id);
    assert_eq!(rows[1]["id"], feature_id);
    let legacy_list = dispatch_tool(&state, "aruvi_catalog", json!({
        "action": "list_product_capabilities", "arguments": {"product_id": "catalog-product-a"}
    })).await.unwrap();
    assert_eq!(list["result"], legacy_list["result"]);
    let other = dispatch_tool(&state, "aruvi_catalog", json!({
        "action": "list_product_capabilities", "arguments": {"product_id": "catalog-product-b"}
    })).await.unwrap();
    assert!(other["result"].as_array().unwrap().is_empty());

    let summary = dispatch_tool(&state, "catalog.products.summarize_tree", json!({"productId": "catalog-product-a"})).await.unwrap();
    assert_eq!(summary["result"]["product_area_count"], 1);
    assert_eq!(summary["result"]["capability_count"], 2);
    assert_eq!(summary["result"]["leaf_node_count"], 1);
    let legacy_summary = dispatch_tool(&state, "aruvi_catalog", json!({
        "action": "summarize_product_tree", "arguments": {"product_id": "catalog-product-a"}
    })).await.unwrap();
    assert_eq!(summary["result"], legacy_summary["result"]);
    assert!(dispatch_tool(&state, "catalog.products.summarize_tree", json!({"productId": "missing"})).await.is_err());
}

#[tokio::test]
async fn mcp_exposes_scope_summaries_and_all_repository_attachments() {
    let state = create_test_state("scope_and_attachment_read_parity").await;
    create_test_product(&state, "scope-product-a").await;
    create_test_product(&state, "scope-product-b").await;
    create_test_work_item(&state, "scope-work-a", "scope-product-a", "First").await;
    create_test_work_item(&state, "scope-work-b", "scope-product-b", "Second").await;
    crate::persistence::repository_repo::create_repository(&state.db, "scope-repo", "Workspace", "/tmp/scope-fixture", "", "main").await.unwrap();
    crate::persistence::repository_repo::attach_repository(&state.db, "scope-attachment", "product", "scope-product-a", "scope-repo", true).await.unwrap();

    let scoped = dispatch_tool(&state, "work_items.summarize_by_scope", json!({"productId": "scope-product-a"})).await.unwrap();
    let rows = scoped["result"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["product_id"], "scope-product-a");
    assert_eq!(rows[0]["total_count"], 1);
    let legacy = dispatch_tool(&state, "aruvi_work_items", json!({
        "action": "summarize_work_items_by_scope", "arguments": {"product_id": "scope-product-a"}
    })).await.unwrap();
    assert_eq!(legacy["result"], scoped["result"]);
    let all = dispatch_tool(&state, "work_items.summarize_by_scope", json!({})).await.unwrap();
    let all_rows = all["result"].as_array().unwrap();
    assert!(all_rows.iter().any(|row| row["product_id"] == "scope-product-a"));
    assert!(all_rows.iter().any(|row| row["product_id"] == "scope-product-b"));

    let attachments = dispatch_tool(&state, "repositories.attachments.list_all", json!({})).await.unwrap();
    let attachment_rows = attachments["result"].as_array().unwrap();
    assert_eq!(attachment_rows.len(), 1);
    assert_eq!(attachment_rows[0]["id"], "scope-attachment");
    assert_eq!(attachment_rows[0]["scope_id"], "scope-product-a");
    let legacy_attachments = dispatch_tool(&state, "aruvi_repositories", json!({
        "action": "list_all_repository_attachments", "arguments": {}
    })).await.unwrap();
    assert_eq!(legacy_attachments["result"], attachments["result"]);
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

#[tokio::test]
async fn database_settings_use_desktop_validation_and_persist() {
    let state = create_test_state("database_settings_parity").await;
    let path = state.app_data_dir.join("alternate.db").to_string_lossy().to_string();
    let call = |action: &str, arguments: serde_json::Value| json!({"action": action, "arguments": arguments});
    for bad in ["   ", "relative.db"] {
        assert!(dispatch_tool(&state, "aruvi_settings", call("set_database_path_override", json!({"db_path": bad}))).await.is_err());
    }
    dispatch_tool(&state, "aruvi_settings", call("set_database_path_override", json!({"db_path": format!("  {path}  ")}))).await.unwrap();
    let result = dispatch_tool(&state, "aruvi_settings", call("get_database_path_override", json!({}))).await.unwrap();
    assert_eq!(result["result"]["path"], path);
    assert!(dispatch_tool(&state, "aruvi_settings", call("set_database_path_override", json!({"db_path":"/tmp/other.db", "expected":{"path":null}}))).await.is_err());
    assert!(dispatch_tool(&state, "aruvi_settings", call("clear_database_path_override", json!({"expected":{"path":"/tmp/stale.db"}}))).await.is_err());
    let still = dispatch_tool(&state, "aruvi_settings", call("get_database_path_override", json!({}))).await.unwrap();
    assert_eq!(still["result"]["path"], path);
    let health = dispatch_tool(&state, "aruvi_settings", call("get_database_health", json!({}))).await.unwrap();
    assert!(health["result"]["applied_migrations"].as_u64().unwrap() > 0);
    dispatch_tool(&state, "aruvi_settings", call("clear_database_path_override", json!({"expected":{"path":path}}))).await.unwrap();
    let cleared = dispatch_tool(&state, "aruvi_settings", call("get_database_path_override", json!({}))).await.unwrap();
    assert!(cleared["result"]["path"].is_null());
}

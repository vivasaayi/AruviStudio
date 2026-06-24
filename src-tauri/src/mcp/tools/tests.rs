use super::*;
use crate::persistence::{db as db_service, product_repo, work_item_repo};
use crate::state::AppState;

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

#[test]
fn definitions_include_first_class_tools_after_legacy_tools() {
    let definitions = definitions();
    let legacy_index = definitions
        .iter()
        .position(|tool| tool.name == "aruvi_catalog")
        .expect("legacy aruvi_catalog tool");
    let first_class_index = definitions
        .iter()
        .position(|tool| tool.name == "catalog.products.get_tree")
        .expect("catalog.products.get_tree");
    let first_class_tool = &definitions[first_class_index];

    assert!(legacy_index < first_class_index);
    assert_eq!(first_class_tool.title.as_deref(), Some("Get Product Tree"));
    assert_eq!(
        first_class_tool
            .input_schema
            .get("additionalProperties")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn discovery_exposes_node_kind_constraints_in_catalog_tool_schemas() {
    let definitions = definitions();
    let product_area_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.product_areas.create")
        .expect("catalog.product_areas.create");
    let capability_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.capabilities.create")
        .expect("catalog.capabilities.create");

    let root_kind_enum = product_area_create_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("nodeKind"))
        .and_then(Value::as_object)
        .and_then(|node_kind| node_kind.get("enum"))
        .and_then(Value::as_array)
        .expect("product area nodeKind enum");

    assert_eq!(root_kind_enum, &vec![json!("product_area")]);
    assert!(product_area_create_tool
        .description
        .contains("aruvi://catalog/node-kind-constraints"));
    assert!(capability_create_tool
        .description
        .contains("Feature is the product-management leaf"));
}

#[test]
fn discovery_exposes_catalog_reference_tools() {
    let definitions = definitions();
    let reference_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.references.create")
        .expect("catalog.references.create");

    let scope_type_enum = reference_create_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("scopeType"))
        .and_then(Value::as_object)
        .and_then(|scope_type| scope_type.get("enum"))
        .and_then(Value::as_array)
        .expect("reference scopeType enum");

    assert!(scope_type_enum.contains(&json!("product_area")));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.references.list"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.references.delete"));
}

#[test]
fn discovery_exposes_bulk_import_tools() {
    let definitions = definitions();
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.schema"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.submit"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.get_status"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.list_jobs"));
}

#[test]
fn discovery_exposes_agent_work_coordination_tools() {
    let definitions = definitions();
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "aruvi_agent_work"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.claim_next"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.heartbeat"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.requeue_expired"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.dependencies.upsert"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.evidence.append"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.commits.link"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.materialize_catalog"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.link_catalog_work_items"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.runs.summary"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.runs.health"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.context.get_feature"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.context.export_feature"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "repositories.git.status"));
}

#[test]
fn discovery_exposes_work_item_pagination_metadata_option() {
    let definitions = definitions();
    let list_tool = definitions
        .iter()
        .find(|tool| tool.name == "work_items.list")
        .expect("work_items.list tool");

    assert!(list_tool.description.contains("includePagination=true"));
    let properties = list_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("properties object");
    assert!(properties.contains_key("includePagination"));
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

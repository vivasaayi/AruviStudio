use super::build_webhook_router;
use crate::commands::test_helpers::make_test_app;
use crate::persistence::{artifact_repo, product_repo, settings_repo, workflow_repo};
use crate::state::AppState;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tauri::test::MockRuntime;
use tauri::Manager;
use tower::ServiceExt;

async fn send_json(
    router: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, String) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if payload.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let body = payload
        .map(|value| Body::from(value.to_string()))
        .unwrap_or_else(Body::empty);
    let response = router
        .clone()
        .oneshot(builder.body(body).expect("request should build"))
        .await
        .expect("router should answer request");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    (
        status,
        String::from_utf8(bytes.to_vec()).expect("response body should be utf-8"),
    )
}

fn parse_json(body: &str) -> Value {
    serde_json::from_str(body).expect("response body should be JSON")
}

#[tokio::test]
async fn health_reports_database_availability() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_database_health").await;
    let state = app.state::<AppState>().inner().clone();
    let router = build_webhook_router(state.clone());
    let (status, _) = send_json(&router, Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    state.db.close().await;
    let (status, body) = send_json(&router, Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "database unavailable");
}

#[tokio::test]
async fn remote_product_creation_requires_authentication_and_a_name() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_product_create").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "product-test-token")
        .await.expect("configure token");
    let router = build_webhook_router(state);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/products", None,
        Some(json!({"name":"Unauthorized"}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/products", Some("product-test-token"),
        Some(json!({"name":"  "}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("product-test-token"),
        Some(json!({"name":"  Remote product  ", "description":"  Browser setup  "}))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let product = parse_json(&body);
    assert_eq!(product["name"], "Remote product");
    assert_eq!(product["description"], "Browser setup");
    let (status, body) = send_json(&router, Method::GET, "/api/mobile/products", Some("product-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(parse_json(&body).as_array().unwrap().iter().any(|item| item["id"] == product["id"]));
}

#[tokio::test]
async fn remote_model_setup_requires_auth_and_rejects_credential_urls() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_model_setup").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "model-test-token").await.unwrap();
    let router = build_webhook_router(state);
    let payload = json!({"name":"Provider", "base_url":"https://user:password@example.com/v1"});
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/providers", None, Some(payload.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/providers", Some("model-test-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/providers", Some("model-test-token"), Some(json!({"name":"Provider", "base_url":"https://example.com/v1", "api_key":"ref:test-sensitive-reference"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let provider = parse_json(&body);
    assert!(provider.get("auth_secret_ref").is_none());
    assert!(!body.contains("test-sensitive-reference"));
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/models", Some("model-test-token"), Some(json!({"provider_id":provider["id"], "name":"example-model"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let model = parse_json(&body);
    assert_eq!(model["name"], "example-model");
    let (status, body) = send_json(&router, Method::GET, "/api/mobile/providers", Some("model-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(parse_json(&body).as_array().unwrap().iter().all(|item| item.get("auth_secret_ref").is_none()));
    let (status, _) = send_json(&router, Method::GET, "/api/mobile/agents", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, body) = send_json(&router, Method::GET, "/api/mobile/agents", Some("model-test-token"), None).await;
    let agent_id = parse_json(&body)[0]["id"].as_str().unwrap().to_string();
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/agent-model-bindings", Some("model-test-token"), Some(json!({"agent_id":agent_id, "model_id":model["id"]}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/agent-model-bindings", Some("model-test-token"), Some(json!({"agent_id":agent_id, "model_id":"missing-model"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, body) = send_json(&router, Method::GET, "/api/mobile/agent-model-bindings", Some("model-test-token"), None).await;
    assert!(parse_json(&body).as_array().unwrap().iter().any(|item| item["agent_id"] == agent_id && item["model_id"] == model["id"]));
}

#[tokio::test]
async fn remote_workspace_registration_is_authenticated_and_idempotent() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_workspace_setup").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "workspace-test-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let dir = std::env::temp_dir().join(format!("aruvi-web-repo-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    git2::Repository::init(&dir).unwrap();
    let payload = json!({"name":"Test workspace", "local_path":dir.to_str().unwrap()});
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/repositories", None, Some(payload.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/repositories", Some("workspace-test-token"), Some(json!({"name":"Bad", "local_path":"relative"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/repositories", Some("workspace-test-token"), Some(payload.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let repository = parse_json(&body);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/repositories", Some("workspace-test-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(parse_json(&body)["id"], repository["id"]);
    let mcp = crate::mcp::dispatch_tool(&state, "aruvi_repositories", json!({"action":"register_repository", "arguments":{"name":"Same", "local_path":dir.to_str().unwrap()}})).await.unwrap();
    assert_eq!(mcp["result"]["id"], repository["id"]);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("workspace-test-token"), Some(json!({"name":"Workspace product"}))).await;
    let product = parse_json(&body);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/repository-attachments", Some("workspace-test-token"), Some(json!({"product_id":product["id"], "repository_id":repository["id"]}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, body) = send_json(&router, Method::GET, "/api/mobile/repository-attachments", Some("workspace-test-token"), None).await;
    assert!(parse_json(&body).as_array().unwrap().iter().any(|item| item["repository_id"] == repository["id"] && item["is_default"] == true));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn remote_test_command_configuration_is_authenticated_and_validated() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_test_config").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "config-token").await.unwrap();
    let router = build_webhook_router(state);
    let payload = json!({"argv":["true"]});
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", None, Some(payload.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(json!({"argv":[""]}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = send_json(&router, Method::GET, "/api/mobile/workflow-test-config", Some("config-token"), None).await;
    assert_eq!(parse_json(&body)["argv"], json!(["true"]));
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("config-token"), Some(json!({"name":"Scoped product"}))).await;
    let product = parse_json(&body);
    let id = product["id"].as_str().unwrap();
    let uri = format!("/api/mobile/workflow-test-config?product_id={id}");
    let (_, body) = send_json(&router, Method::GET, &uri, Some("config-token"), None).await;
    assert_eq!(parse_json(&body), json!({"mode":"inherit","argv":["true"]}));
    for (mode, argv) in [("custom", json!(["false"])), ("disabled", json!([])), ("inherit", json!(["true"]))] {
        let (status, body) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(json!({"product_id":id,"mode":mode,"argv":["false"]}))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(parse_json(&body), json!({"mode":mode,"argv":argv}));
    }
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(json!({"product_id":id,"mode":"custom","argv":[]}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(json!({"product_id":"missing","mode":"disabled"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/workflow-test-config", Some("config-token"), Some(json!({"argv":[]}))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn remote_test_result_is_scoped_and_confined_to_artifacts() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_test_result").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "result-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("result-token"), Some(json!({"name":"Result product"}))).await;
    let product = parse_json(&body);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("result-token"), Some(json!({"product_id":product["id"], "title":"Result task"}))).await;
    let item = parse_json(&body);
    let path = state.artifact_base_path.join("test-result.json");
    tokio::fs::write(&path, "{\"status\":\"passed\"}").await.unwrap();
    let artifact = artifact_repo::create_artifact(&state.db, artifact_repo::CreateArtifactInput {
        id:"test-result", work_item_id:item["id"].as_str().unwrap(), workflow_run_id:None, agent_run_id:None,
        artifact_type:"workflow_test_result", summary:"result", storage_path:path.to_str().unwrap(),
    }).await.unwrap();
    let uri = format!("/api/mobile/work-items/{}/test-results/{}", item["id"].as_str().unwrap(), artifact.id);
    let (status, _) = send_json(&router, Method::GET, &uri, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, &uri, Some("result-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(parse_json(&body)["content"].as_str().unwrap().contains("passed"));
    let (status, _) = send_json(&router, Method::GET, "/api/mobile/work-items/other/test-results/test-result", Some("result-token"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    sqlx::query("UPDATE artifacts SET storage_path=? WHERE id=?").bind("/etc/passwd").bind(&artifact.id).execute(&state.db).await.unwrap();
    let (status, _) = send_json(&router, Method::GET, &uri, Some("result-token"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remote_work_api_auth_create_start_approve_and_delivery_flow() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_work_http_flow").await;
    let state = app.state::<AppState>().inner().clone();
    let token = "remote-work-test-token";
    settings_repo::set_setting(&state.db, "mobile.api_token", token)
        .await
        .expect("mobile token should be configured");

    let product = product_repo::create_product(
        &state.db,
        product_repo::CreateProductInput {
            id: "remote-work-product",
            name: "Remote Work Product",
            description: "HTTP integration test product",
            vision: "",
            goals: "[]",
            tags: "[]",
            lifecycle: None,
            health: None,
            owner_label: None,
            investment_status: None,
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("product should be created");
    let router = build_webhook_router(state.clone());

    let (status, _) = send_json(&router, Method::GET, "/api/mobile/work-items", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = send_json(
        &router,
        Method::POST,
        "/api/mobile/work-items",
        Some(token),
        Some(json!({
            "product_id": product.id,
            "title": "Ship from remote Studio",
            "description": "Exercise the authenticated HTTP workflow",
            "acceptance_criteria": "The workflow run is returned to the caller"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let work_item = parse_json(&body);
    let work_item_id = work_item["id"]
        .as_str()
        .expect("created work item should have an id");

    let (status, body) = send_json(
        &router,
        Method::POST,
        &format!("/api/mobile/work-items/{work_item_id}/workflow/start"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(
        parse_json(&body)["error"]
            .as_str()
            .is_some_and(|message| message.contains("must be approved")),
        "{body}"
    );

    let seeded_run =
        workflow_repo::create_workflow_run(&state.db, "remote-workflow-run", work_item_id)
            .await
            .expect("workflow run should be seeded");
    workflow_repo::update_workflow_stage(&state.db, &seeded_run.id, "pending_plan_approval")
        .await
        .expect("workflow should be placed at a stable gate");

    let (status, body) = send_json(
        &router,
        Method::POST,
        &format!("/api/mobile/work-items/{work_item_id}/approve"),
        Some(token),
        Some(json!({ "notes": "Approved by HTTP integration test" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let started = parse_json(&body);
    assert_eq!(started["status"], "started");
    assert_eq!(started["work_item_id"], work_item_id);
    assert_eq!(started["workflow_run"]["id"], seeded_run.id);
    assert_eq!(
        started["workflow_run"]["current_stage"],
        "pending_plan_approval"
    );

    let (status, body) = send_json(
        &router,
        Method::POST,
        &format!("/api/mobile/work-items/{work_item_id}/workflow/start"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(parse_json(&body)["workflow_run"]["id"], seeded_run.id);

    let (status, body) = send_json(
        &router,
        Method::GET,
        &format!("/api/mobile/work-items/{work_item_id}/delivery"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let delivery = parse_json(&body);
    assert_eq!(delivery["work_item"]["status"], "approved");
    assert_eq!(delivery["workflow_run"]["id"], seeded_run.id);

    let (status, body) = send_json(
        &router,
        Method::POST,
        &format!("/api/mobile/workflows/{}/action", seeded_run.id),
        Some(token),
        Some(json!({ "action": "unsupported" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

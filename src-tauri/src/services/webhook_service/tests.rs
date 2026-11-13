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
async fn remote_planner_draft_edits_are_authenticated_and_persisted() {
    use crate::services::planner_service::{apply_actions_to_draft, create_planner_session, persist_draft_state};
    use crate::services::planner_action_parser::normalize_planner_action;
    let app: tauri::App<MockRuntime> = make_test_app("remote_planner_draft_edits").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "draft-test-token").await.unwrap();
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let action = normalize_planner_action(json!({"type":"create_product", "name":"Test product"})).unwrap();
    let draft = apply_actions_to_draft(None, None, &[action]).unwrap();
    let root = draft.nodes[0].id.clone();
    {
        let mut service = state.planner_service.lock().await;
        let mut loaded = service.get_session(&session.session_id).unwrap();
        loaded.draft_plan = Some(draft.clone());
        service.save_session(&session.session_id, loaded);
    }
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.unwrap();
    let router = build_webhook_router(state.clone());
    let base = format!("/api/mobile/planner/sessions/{}/draft", session.session_id);
    let payload = json!({"node_id":root,"name":"Renamed product"});
    let (status, _) = send_json(&router, Method::POST, &format!("{base}/rename"), None, Some(payload.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::POST, &format!("{base}/rename"), Some("draft-test-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(parse_json(&body)["draft_tree_nodes"][0]["label"], "Renamed product");
    let (status, body) = send_json(&router, Method::POST, &format!("{base}/add"), Some("draft-test-token"), Some(json!({"node_id":root,"name":"Area","child_type":"product_area"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let child = parse_json(&body)["selected_draft_node_id"].as_str().unwrap().to_string();
    let empty_session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let empty_url = format!("/api/mobile/planner/sessions/{}/review-packet", empty_session.session_id);
    assert_eq!(send_json(&router, Method::GET, &empty_url, Some("draft-test-token"), None).await.0, StatusCode::BAD_REQUEST);
    let packet_url = format!("/api/mobile/planner/sessions/{}/review-packet", session.session_id);
    assert_eq!(send_json(&router, Method::GET, &packet_url, None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, &packet_url, Some("draft-test-token"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let packet = parse_json(&body);
    assert!(packet["filename"].as_str().unwrap().ends_with("-design-review-packet.html"));
    assert!(packet["html"].as_str().unwrap().contains("Approval Section"));
    assert!(packet["html"].as_str().unwrap().contains("Area"));
    let mcp_packet = crate::mcp::dispatch_tool(&state, "aruvi_planner", json!({"action":"export_planner_review_packet", "arguments":{"session_id":session.session_id}})).await.unwrap();
    assert!(mcp_packet["result"]["html"].as_str().unwrap().contains("Approval Section"));
    let validation = format!("/api/mobile/planner/sessions/{}/validation", session.session_id);
    assert_eq!(send_json(&router, Method::GET, &validation, None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, summary) = send_json(&router, Method::GET, &validation, Some("draft-test-token"), None).await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    let summary = parse_json(&summary);
    assert_eq!(summary["counts"]["product area"], 1);
    assert_eq!(summary["score"], 88);
    let via_mcp = crate::mcp::dispatch_tool(&state, "aruvi_planner", json!({"action":"validate_planner_draft", "arguments":{"session_id":session.session_id}})).await.unwrap();
    assert_eq!(via_mcp["result"], summary);
    let (status, body) = send_json(&router, Method::POST, &format!("{base}/delete"), Some("draft-test-token"), Some(json!({"node_id":child}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let persisted = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    let persisted_draft: Value = serde_json::from_str(persisted.draft_plan_json.as_deref().unwrap()).unwrap();
    assert_eq!(persisted_draft["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(persisted_draft["nodes"][0]["name"], "Renamed product");
    let injected = json!({"node_id":root,"name":"<script>alert(1)</script>"});
    assert_eq!(send_json(&router, Method::POST, &format!("{base}/rename"), Some("draft-test-token"), Some(injected)).await.0, StatusCode::OK);
    let (status, body) = send_json(&router, Method::GET, &packet_url, Some("draft-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    let html = parse_json(&body)["html"].as_str().unwrap().to_string();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
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
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/providers", Some("model-test-token"), Some(json!({"name":"Claude", "protocol":"anthropic", "base_url":"https://api.anthropic.com"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(parse_json(&body)["protocol"], "anthropic");
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/providers", Some("model-test-token"), Some(json!({"name":"Invalid", "protocol":"unknown", "base_url":"https://example.com"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
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

#[tokio::test]
async fn anthropic_protocol_routes_mcp_and_remote_chat() {
    use axum::{routing::{get, post}, extract::Query, Json};
    use crate::mcp::dispatch_tool;
    async fn messages(headers: axum::http::HeaderMap, Json(body): Json<Value>) -> Json<Value> {
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-synthetic-key-123456");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        assert!(headers.get("authorization").is_none());
        assert_eq!(body["model"], "claude-test");
        assert_eq!(body["messages"][0]["role"], "user");
        let stop_reason = if body["messages"][0]["content"] == "truncated" { "max_tokens" } else { "end_turn" };
        Json(json!({"content":[{"type":"text","text":"Claude fixture response"}],"stop_reason":stop_reason,"usage":{"input_tokens":4,"output_tokens":3}}))
    }
    async fn models(headers: axum::http::HeaderMap, Query(query): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-synthetic-key-123456");
        if query.get("after_id").is_some_and(|value| value == "claude-first") {
            Json(json!({"data":[{"id":"claude-second","display_name":"Claude second"}],"has_more":false,"last_id":"claude-second"}))
        } else {
            Json(json!({"data":[{"id":"claude-first","display_name":"Claude first"}],"has_more":true,"last_id":"claude-first"}))
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, axum::Router::new().route("/v1/messages", post(messages)).route("/v1/models", get(models))).await.unwrap();
    });
    let app: tauri::App<MockRuntime> = make_test_app("anthropic_mcp_remote").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "anthropic-test-token").await.unwrap();
    let created = dispatch_tool(&state, "aruvi_models", json!({"action":"create_provider","arguments":{
        "name":"Claude fixture","protocol":"anthropic","base_url":format!("http://{address}")
    }})).await.unwrap();
    assert_eq!(created["result"]["protocol"], "anthropic");
    let id = created["result"]["id"].as_str().unwrap();
    let missing_key = dispatch_tool(&state, "aruvi_models", json!({"action":"run_model_chat_completion","arguments":{
        "provider_id":id,"model":"claude-test","messages":[{"role":"user","content":"hello"}]
    }})).await;
    assert!(missing_key.is_err());
    // Use the documented legacy-read path for a synthetic key; do not write the host secret store.
    sqlx::query("UPDATE model_providers SET auth_secret_ref=? WHERE id=?")
        .bind("sk-test-synthetic-key-123456").bind(id).execute(&state.db).await.unwrap();
    let mcp = dispatch_tool(&state, "aruvi_models", json!({"action":"run_model_chat_completion","arguments":{
        "provider_id":id,"model":"claude-test","messages":[{"role":"user","content":"hello"}]
    }})).await.unwrap();
    assert_eq!(mcp["result"]["content"], "Claude fixture response");
    assert_eq!(mcp["result"]["token_count_input"], 4);
    let truncated = dispatch_tool(&state, "aruvi_models", json!({"action":"run_model_chat_completion","arguments":{
        "provider_id":id,"model":"claude-test","messages":[{"role":"user","content":"truncated"}]
    }})).await.unwrap_err();
    assert!(truncated.to_string().contains("truncated"));
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_definitions").fetch_one(&state.db).await.unwrap();
    let page = dispatch_tool(&state, "aruvi_models", json!({"action":"discover_provider_models","arguments":{"id":id}})).await.unwrap();
    assert_eq!(page["result"]["models"][0]["id"], "claude-first");
    assert_eq!(page["result"]["next_cursor"], "claude-first");
    let next = dispatch_tool(&state, "aruvi_models", json!({"action":"discover_provider_models","arguments":{"id":id,"cursor":"claude-first"}})).await.unwrap();
    assert_eq!(next["result"]["models"][0]["id"], "claude-second");
    assert!(next["result"]["next_cursor"].is_null());
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_definitions").fetch_one(&state.db).await.unwrap();
    assert_eq!(before, after);
    let router = build_webhook_router(state);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/providers/discover", None, Some(json!({"id":id}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/providers/discover", Some("anthropic-test-token"), Some(json!({"id":id,"cursor":"claude-first"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(parse_json(&body)["models"][0]["id"], "claude-second");
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/chat/completions", Some("anthropic-test-token"), Some(json!({
        "provider_id":id,"model_name":"claude-test","messages":[{"role":"user","content":"hello"}]
    }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(parse_json(&body)["content"], "Claude fixture response");
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/chat/completions", Some("anthropic-test-token"), Some(json!({
        "provider_id":id,"model_name":"claude-test","messages":[{"role":"user","content":"truncated"}]
    }))).await;
    assert!(!status.is_success(), "truncated response unexpectedly succeeded: {body}");
    assert!(body.contains("truncated"));
    server.abort();
}

#[tokio::test]
async fn remote_repository_analysis_requires_auth_and_stages_design() {
    use crate::persistence::{model_repo, repository_repo};
    use crate::services::planner_service::create_planner_session;
    use axum::{extract::State, routing::post, Json, Router};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    async fn completion(State(calls): State<Arc<AtomicUsize>>, Json(request): Json<Value>) -> Json<Value> {
        let count = calls.fetch_add(1, Ordering::SeqCst);
        let request_text = request.to_string();
        assert!(!request_text.contains("fixture-secret"));
        assert!(!request_text.contains("fixture-query-secret"));
        assert!(!request_text.contains("aruvi-repo-test-"));
        assert!(!request_text.contains("remote_url"));
        assert!(!request_text.contains("local_path"));
        let actions = if count == 0 {
            json!([{"node_type":"product_area", "name":"Inventory", "details":{"type":"create_product_area"}}])
        } else {
            assert!(request["messages"].as_array().unwrap().iter().any(|message| message["content"].as_str().unwrap_or("").contains("Correct the previous response")));
            if count == 1 { json!([{"type":"start_workflow"}]) }
            else { json!([{"type":"create_product_area", "name":"Inventory", "target":{"productName":"Analysis Product"}}]) }
        };
        Json(json!({"id":"fixture", "model":"fixture", "choices":[{"message":{"role":"assistant", "content":json!({"type":"final", "assistant_response":"Proposed inventory area", "needs_confirmation":false, "clarification_question":null, "actions":actions}).to_string()}, "finish_reason":"stop"}]}))
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let server_calls = calls.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, Router::new().route("/v1/chat/completions", post(completion)).with_state(server_calls)).await.unwrap(); });
    let app: tauri::App<MockRuntime> = make_test_app("remote_repository_analysis").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "analysis-token").await.unwrap();
    model_repo::create_provider(&state.db, "analysis-provider", "Fixture", "openai_compatible", &format!("http://{address}/v1"), None).await.unwrap();
    model_repo::create_model_definition(&state.db, "analysis-model", "analysis-provider", "fixture", None, None, None).await.unwrap();
    let session = create_planner_session(state.planner_service.clone(), &state.db, Some("analysis-provider".into()), Some("fixture".into())).await.unwrap();
    let router = build_webhook_router(state.clone());
    let (status, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("analysis-token"), Some(json!({"name":"Analysis Product"}))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let product = parse_json(&body)["id"].as_str().unwrap().to_string();
    let root = std::env::temp_dir().join(format!("aruvi-repo-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("src/features/inventory")).unwrap();
    std::fs::write(root.join("README.md"), "# Inventory application").unwrap();
    repository_repo::create_repository(&state.db, "analysis-repo", "Fixture", root.to_str().unwrap(), "https://fixture-user:fixture-secret@example.invalid/repo?token=fixture-query-secret", "main").await.unwrap();
    let path = format!("/api/mobile/planner/sessions/{}/analyze-repository", session.session_id);
    let payload = json!({"repository_id":"analysis-repo", "product_id":product});
    assert_eq!(send_json(&router, Method::POST, &path, None, Some(payload.clone())).await.0, StatusCode::UNAUTHORIZED);
    let missing = json!({"repository_id":"missing-repository", "product_id":product});
    assert_eq!(send_json(&router, Method::POST, &path, Some("analysis-token"), Some(missing)).await.0, StatusCode::BAD_REQUEST);
    let cached = state.planner_service.lock().await.get_session(&session.session_id).unwrap();
    let persisted = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    assert_eq!(serde_json::to_value(&cached.draft_plan).unwrap(), serde_json::to_value(&persisted.draft_plan).unwrap());
    let (status, body) = send_json(&router, Method::POST, &path, Some("analysis-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let result = parse_json(&body);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(result["trace_events"].as_array().unwrap().iter().any(|event| event["stage"] == "format_retry"));
    assert_eq!(result["draft_tree_nodes"][0]["children"][0]["label"], "Inventory");
    let saved = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    assert!(saved.draft_plan.unwrap().nodes.iter().any(|node| node.name == "Inventory"));
    assert!(product_repo::list_product_areas(&state.db, &product).await.unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
    server.abort();
}

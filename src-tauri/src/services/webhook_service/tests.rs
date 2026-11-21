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

#[tokio::test]
async fn webhook_shutdown_waits_for_in_flight_http_requests() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let gate = std::sync::Arc::new(std::sync::Mutex::new(Some((entered_tx, release_rx))));
    let router = axum::Router::new().route("/slow", axum::routing::get(move || {
        let gate = gate.clone();
        async move {
            let (entered, release) = gate.lock().unwrap().take().unwrap();
            let _ = entered.send(());
            let _ = release.await;
            "complete"
        }
    }));
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(super::serve_webhook(listener, router, async { let _ = shutdown_rx.await; }));
    let request = tokio::spawn(async move { reqwest::get(format!("http://{address}/slow")).await.unwrap().text().await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered_rx).await.unwrap().unwrap();
    shutdown_tx.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!server.is_finished(), "server exited before the active response completed");
    release_tx.send(()).unwrap();
    assert_eq!(request.await.unwrap(), "complete");
    tokio::time::timeout(std::time::Duration::from_secs(5), server).await.unwrap().unwrap().unwrap();
}

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
    let (status, body) = send_json(&router, Method::POST, &format!("{base}/add"), Some("draft-test-token"), Some(json!({"node_id":root,"name":"Area","child_type":"product_area","summary":"Staged detail <script>fixture</script>"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let child = parse_json(&body)["selected_draft_node_id"].as_str().unwrap().to_string();
    let empty_session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let empty_url = format!("/api/mobile/planner/sessions/{}/review-packet", empty_session.session_id);
    assert_eq!(send_json(&router, Method::GET, &empty_url, Some("draft-test-token"), None).await.0, StatusCode::BAD_REQUEST);
    let snapshot = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    let expected = crate::services::planner_draft::build_draft_tree_nodes(snapshot.draft_plan.as_ref().unwrap(), snapshot.selected_draft_node_id.as_deref());
    let desktop = crate::services::planner_review_packet::build_desktop_review_packet(state.planner_service.clone(), &state.db, session.session_id.clone(), expected.clone(), Some(child.clone())).await.unwrap();
    assert!(desktop.html.contains("&quot;selectedNode&quot;: &quot;Area&quot;"));
    let mut stale = expected.clone();
    stale[0].label = "Stale name".into();
    assert!(crate::services::planner_review_packet::build_desktop_review_packet(state.planner_service.clone(), &state.db, session.session_id.clone(), stale, None).await.is_err());
    assert!(crate::services::planner_review_packet::build_desktop_review_packet(state.planner_service.clone(), &state.db, session.session_id.clone(), expected, Some("missing-node".into())).await.is_err());
    let packet_url = format!("/api/mobile/planner/sessions/{}/review-packet", session.session_id);
    assert_eq!(send_json(&router, Method::GET, &packet_url, None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, &packet_url, Some("draft-test-token"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let packet = parse_json(&body);
    assert!(packet["filename"].as_str().unwrap().ends_with("-design-review-packet.html"));
    assert!(packet["html"].as_str().unwrap().contains("Approval Section"));
    assert!(packet["html"].as_str().unwrap().contains("Area"));
    assert!(packet["html"].as_str().unwrap().contains("draftNodes"));
    assert!(packet["html"].as_str().unwrap().contains("Staged detail &lt;script&gt;fixture&lt;/script&gt;"));
    assert!(!packet["html"].as_str().unwrap().contains("<script>"));
    assert!(packet["html"].as_str().unwrap().contains("absence does not imply deletion"));
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
async fn bridge_status_requires_mobile_auth_and_does_not_return_credentials() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_bridge_status").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "mobile-fixture-secret").await.unwrap();
    settings_repo::set_setting(&state.db, "mcp.api_token", "mcp-fixture-secret").await.unwrap();
    let router = build_webhook_router(state);
    assert_eq!(send_json(&router, Method::GET, "/api/mobile/bridge-status", None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, "/api/mobile/bridge-status", Some("mobile-fixture-secret"), None).await;
    assert_eq!(status, StatusCode::OK);
    let value = parse_json(&body);
    assert_eq!(value["mcp_token_configured"], true);
    assert!(value["configured_bind_port"].is_number());
    assert!(!body.contains("mcp-fixture-secret"));
    assert!(!body.contains("mobile-fixture-secret"));
    assert!(value.get("endpoint_url").is_none());
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
async fn prompt_templates_support_guarded_management_and_legacy_references() {
    let app: tauri::App<MockRuntime> = make_test_app("prompt_template_management").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "template-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let path = "/api/mobile/prompt-templates";
    let fields = json!({"name":"Team coding","role":"coding","template_text":"Use clear commit messages."});
    assert_eq!(send_json(&router, Method::POST, path, None, Some(fields.clone())).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::POST, path, Some("template-token"), Some(fields.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let item = parse_json(&body); let id = item["id"].as_str().unwrap();
    let reference = format!("template:{id}");
    assert_eq!(crate::persistence::prompt_template_repo::resolve(&state.db, "coding_v1", "coding").await.unwrap(), None);
    assert_eq!(crate::persistence::prompt_template_repo::resolve(&state.db, &reference, "coding").await.unwrap().as_deref(), Some("Use clear commit messages."));
    assert!(crate::persistence::prompt_template_repo::resolve(&state.db, &reference, "planning").await.is_err());
    assert!(crate::persistence::prompt_template_repo::resolve(&state.db, "template:missing", "coding").await.is_err());
    let changed = json!({"name":"Revised","role":"*","template_text":"Review changes."});
    let edit = json!({"id":id,"expected_version":1,"fields":changed});
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/prompt-templates/update", Some("template-token"), Some(edit.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(send_json(&router, Method::POST, "/api/mobile/prompt-templates/update", Some("template-token"), Some(edit)).await.0, StatusCode::CONFLICT);
    assert!(crate::mcp::dispatch_tool(&state, "aruvi_agents", json!({"action":"update_prompt_template","arguments":{"id":id,"expected_version":1,"name":"Stale","role":"*","template_text":"Stale"}})).await.is_err());
    let result = crate::mcp::dispatch_tool(&state, "aruvi_agents", json!({"action":"list_prompt_templates","arguments":{}})).await.unwrap();
    assert_eq!(result["result"].as_array().unwrap().len(), 1);
    sqlx::query("UPDATE agent_definitions SET prompt_template_ref=? WHERE id='coding-agent'").bind(&reference).execute(&state.db).await.unwrap();
    let delete = json!({"id":id,"expected_version":2});
    assert_eq!(send_json(&router, Method::POST, "/api/mobile/prompt-templates/delete", Some("template-token"), Some(delete.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE agent_definitions SET prompt_template_ref='coding_v1' WHERE id='coding-agent'").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, "/api/mobile/prompt-templates/delete", Some("template-token"), Some(delete)).await.0, StatusCode::OK);
}

#[tokio::test]
async fn work_order_moves_are_atomic_scoped_and_available_to_mcp() {
    let app: tauri::App<MockRuntime> = make_test_app("work_order_moves").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "order-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("order-token"), Some(json!({"name":"Order product"}))).await;
    let product = parse_json(&body); let product_id = product["id"].as_str().unwrap();
    for title in ["A", "B", "C", "D"] {
        assert_eq!(send_json(&router, Method::POST, "/api/mobile/work-items", Some("order-token"), Some(json!({"product_id":product_id,"title":title}))).await.0, StatusCode::CREATED);
    }
    let read = || async { sqlx::query_as::<_, (String, i64)>("SELECT id,sort_order FROM work_items WHERE product_id=? ORDER BY sort_order,created_at DESC,id").bind(product_id).fetch_all(&state.db).await.unwrap() };
    let original = read().await;
    let entries = |rows: &[(String,i64)]| rows.iter().map(|(id,sort_order)| json!({"id":id,"sort_order":sort_order})).collect::<Vec<_>>();
    let path = "/api/mobile/work-items/move";
    let payload = json!({"product_id":product_id,"offset":1,"items":entries(&original[1..3]),"source_id":original[2].0});
    assert_eq!(send_json(&router, Method::POST, path, None, Some(payload.clone())).await.0, StatusCode::UNAUTHORIZED);
    let wrong_scope = json!({"product_id":"other","offset":1,"items":entries(&original[1..3]),"source_id":original[2].0});
    assert_eq!(send_json(&router, Method::POST, path, Some("order-token"), Some(wrong_scope)).await.0, StatusCode::CONFLICT);
    sqlx::query("CREATE TRIGGER fixture_order_failure BEFORE UPDATE OF sort_order ON work_items WHEN OLD.id=? BEGIN SELECT RAISE(ABORT, 'fixture'); END".replace("?", &format!("'{}'", original[2].0)).as_str()).execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, path, Some("order-token"), Some(payload.clone())).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(read().await, original);
    let legacy_ids = original.iter().map(|row| row.0.clone()).collect::<Vec<_>>();
    assert!(crate::persistence::work_item_repo::reorder_work_items(&state.db, &legacy_ids).await.is_err());
    assert_eq!(read().await, original);
    sqlx::query("DROP TRIGGER fixture_order_failure").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, path, Some("order-token"), Some(payload.clone())).await.0, StatusCode::OK);
    let changed = read().await;
    assert_eq!(changed.iter().map(|x| &x.0).collect::<Vec<_>>(), vec![&original[0].0,&original[2].0,&original[1].0,&original[3].0]);
    assert_eq!(send_json(&router, Method::POST, path, Some("order-token"), Some(payload)).await.0, StatusCode::CONFLICT);
    crate::mcp::dispatch_tool(&state, "aruvi_work_items", json!({"action":"move_work_item","arguments":{"product_id":product_id,"offset":0,"items":entries(&changed[0..3]),"source_id":changed[0].0}})).await.unwrap();
    let final_order = read().await;
    assert_eq!(final_order.iter().map(|x| &x.0).collect::<Vec<_>>(), vec![&original[2].0,&original[1].0,&original[0].0,&original[3].0]);
    sqlx::query("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<105) INSERT INTO work_items(id,product_id,title,work_item_type,priority,complexity,status,sort_order) SELECT printf('order-extra-%03d',x),?,printf('Extra %03d',x),'task','medium','medium','draft',100+x FROM n")
        .bind(product_id).execute(&state.db).await.unwrap();
    let before_page_move = read().await;
    let page_move = json!({"product_id":product_id,"offset":99,"items":entries(&before_page_move[99..102]),"source_id":before_page_move[101].0});
    assert_eq!(send_json(&router, Method::POST, path, Some("order-token"), Some(page_move)).await.0, StatusCode::OK);
    let after_page_move = read().await;
    let mut expected = before_page_move.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
    let moved = expected.remove(101); expected.insert(99, moved);
    assert_eq!(after_page_move.iter().map(|x| x.0.clone()).collect::<Vec<_>>(), expected);
}

#[tokio::test]
async fn batch_task_decisions_are_atomic_scoped_and_available_to_mcp() {
    let app: tauri::App<MockRuntime> = make_test_app("batch_task_decisions").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "batch-token").await.unwrap();
    settings_repo::set_setting(&state.db, "workflow.auto_start_after_work_item_approval", "false").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("batch-token"), Some(json!({"name":"Batch product"}))).await;
    let product = parse_json(&body);
    let mut items = Vec::new();
    for title in ["First", "Second"] {
        let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("batch-token"), Some(json!({"product_id":product["id"],"title":title}))).await;
        items.push(parse_json(&body));
    }
    let first = items[0]["id"].as_str().unwrap(); let second = items[1]["id"].as_str().unwrap();
    let selected = json!([{"id":first,"expected":items[0]},{"id":second,"expected":items[1]}]);
    let payload = json!({"product_id":product["id"],"action":"approve","notes":"Reviewed","items":selected});
    let path = "/api/mobile/work-items/batch-decision";
    assert_eq!(send_json(&router, Method::POST, path, None, Some(payload.clone())).await.0, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE work_items SET description='concurrent' WHERE id=?").bind(second).execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, path, Some("batch-token"), Some(payload.clone())).await.0, StatusCode::CONFLICT);
    assert_eq!(crate::persistence::work_item_repo::get_work_item(&state.db,first).await.unwrap().status.to_string(), "draft");
    assert!(crate::persistence::approval_repo::list_approvals(&state.db,first).await.unwrap().is_empty());
    sqlx::query("UPDATE work_items SET description=? WHERE id=?").bind(items[1]["description"].as_str().unwrap()).bind(second).execute(&state.db).await.unwrap();
    let duplicate = json!({"product_id":product["id"],"action":"approve","notes":"Reviewed","items":[{"id":first,"expected":items[0]},{"id":first,"expected":items[0]}]});
    assert_eq!(send_json(&router, Method::POST, path, Some("batch-token"), Some(duplicate)).await.0, StatusCode::CONFLICT);
    let wrong_product = json!({"product_id":"different","action":"approve","notes":"Reviewed","items":selected});
    assert_eq!(send_json(&router, Method::POST, path, Some("batch-token"), Some(wrong_product)).await.0, StatusCode::CONFLICT);
    let (status, body) = send_json(&router, Method::POST, path, Some("batch-token"), Some(payload)).await;
    assert_eq!(status, StatusCode::OK, "{body}"); assert_eq!(parse_json(&body)["approvals"].as_array().unwrap().len(), 2);
    let current_a = serde_json::to_value(crate::persistence::work_item_repo::get_work_item(&state.db,first).await.unwrap()).unwrap();
    let current_b = serde_json::to_value(crate::persistence::work_item_repo::get_work_item(&state.db,second).await.unwrap()).unwrap();
    let result = crate::mcp::dispatch_tool(&state, "aruvi_checkpoints", json!({"action":"reject_work_items","arguments":{"product_id":product["id"],"notes":"Revise","items":[{"id":first,"expected":current_a},{"id":second,"expected":current_b}]}})).await.unwrap();
    assert_eq!(result["result"]["approvals"].as_array().unwrap().len(), 2);
    for id in [first, second] {
        assert_eq!(crate::persistence::work_item_repo::get_work_item(&state.db,id).await.unwrap().status.to_string(), "draft");
        assert_eq!(crate::persistence::approval_repo::list_approvals(&state.db,id).await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn task_decisions_are_authenticated_atomic_and_conflict_checked() {
    let app: tauri::App<MockRuntime> = make_test_app("task_decision_atomic").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "decision-token").await.unwrap();
    settings_repo::set_setting(&state.db, "workflow.auto_start_after_work_item_approval", "false").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("decision-token"), Some(json!({"name":"Decision product"}))).await;
    let product = parse_json(&body);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("decision-token"), Some(json!({"product_id":product["id"],"title":"Decision task"}))).await;
    let item = parse_json(&body); let id = item["id"].as_str().unwrap();
    let uri = format!("/api/mobile/work-items/{id}/decision");
    let decision = json!({"action":"approve","notes":"Reviewed","expected":item});
    assert_eq!(send_json(&router, Method::POST, &uri, None, Some(decision.clone())).await.0, StatusCode::UNAUTHORIZED);
    sqlx::query("CREATE TRIGGER fixture_approval_failure BEFORE INSERT ON approvals BEGIN SELECT RAISE(ABORT, 'fixture'); END").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("decision-token"), Some(decision.clone())).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(crate::persistence::work_item_repo::get_work_item(&state.db,id).await.unwrap().status.to_string(), "draft");
    assert!(crate::persistence::approval_repo::list_approvals(&state.db,id).await.unwrap().is_empty());
    sqlx::query("DROP TRIGGER fixture_approval_failure").execute(&state.db).await.unwrap();
    let (status, body) = send_json(&router, Method::POST, &uri, Some("decision-token"), Some(decision.clone())).await;
    assert_eq!(status, StatusCode::OK, "{body}"); assert_eq!(parse_json(&body)["auto_start_requested"], false);
    sqlx::query("CREATE TRIGGER fixture_mcp_approval_failure BEFORE INSERT ON approvals BEGIN SELECT RAISE(ABORT, 'fixture'); END").execute(&state.db).await.unwrap();
    assert!(crate::mcp::dispatch_tool(&state, "aruvi_checkpoints", json!({"action":"reject_work_item","arguments":{"work_item_id":id,"notes":"fixture"}})).await.is_err());
    assert_eq!(crate::persistence::work_item_repo::get_work_item(&state.db,id).await.unwrap().status.to_string(), "approved");
    sqlx::query("DROP TRIGGER fixture_mcp_approval_failure").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("decision-token"), Some(decision)).await.0, StatusCode::CONFLICT);
    let current = serde_json::to_value(crate::persistence::work_item_repo::get_work_item(&state.db,id).await.unwrap()).unwrap();
    let priority = current["priority"].as_str().unwrap().to_string();
    let rejection = json!({"action":"reject","notes":"Revise the scope","expected":current});
    sqlx::query("UPDATE work_items SET priority='high' WHERE id=?").bind(id).execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("decision-token"), Some(rejection.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE work_items SET priority=? WHERE id=?").bind(priority).bind(id).execute(&state.db).await.unwrap();
    workflow_repo::create_workflow_run(&state.db,"decision-active",id).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("decision-token"), Some(rejection.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE workflow_runs SET status='failed',current_stage='failed' WHERE id='decision-active'").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("decision-token"), Some(rejection)).await.0, StatusCode::OK);
    assert_eq!(crate::persistence::work_item_repo::get_work_item(&state.db,id).await.unwrap().status.to_string(), "draft");
    assert_eq!(crate::persistence::approval_repo::list_approvals(&state.db,id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn external_cli_recovery_is_atomic_on_database_failure() {
    let app: tauri::App<MockRuntime> = make_test_app("external_cli_recovery_atomic").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "recovery-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("recovery-token"), Some(json!({"name":"Recovery product"}))).await;
    let product = parse_json(&body);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("recovery-token"), Some(json!({"product_id":product["id"],"title":"Recovery task"}))).await;
    let item = parse_json(&body);
    let invocation = crate::domain::external_cli::ExternalCliInvocation {
        work_item_id:item["id"].as_str().unwrap().into(), provider:"codex".into(), label:"Fixture".into(),
        command:"fixture".into(), args:vec![], prompt:"fixture".into(), cwd:"/tmp".into(), session_log_path:"".into(),
    };
    crate::persistence::external_cli_repo::create_external_cli_run(&state.db, "recovery-test-run", &invocation).await.unwrap();
    sqlx::query("CREATE TRIGGER fixture_recovery_failure BEFORE UPDATE ON external_cli_runs BEGIN SELECT RAISE(ABORT, 'fixture'); END").execute(&state.db).await.unwrap();
    assert!(crate::persistence::external_cli_repo::mark_interrupted_external_cli_runs(&state.db).await.is_err());
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM external_cli_run_events WHERE run_id='recovery-test-run'").fetch_one(&state.db).await.unwrap();
    assert_eq!(count.0, 0);
    assert_eq!(crate::persistence::external_cli_repo::get_external_cli_run(&state.db, "recovery-test-run").await.unwrap().status, "running");
    sqlx::query("DROP TRIGGER fixture_recovery_failure").execute(&state.db).await.unwrap();
    assert_eq!(crate::persistence::external_cli_repo::mark_interrupted_external_cli_runs(&state.db).await.unwrap(), 1);
    let events = crate::persistence::external_cli_repo::list_external_cli_run_events(&state.db, "recovery-test-run", 10).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn external_cli_remote_routes_require_authentication() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_external_cli_auth").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "external-cli-token").await.unwrap();
    let router = build_webhook_router(state);
    let path = "/api/mobile/work-items/missing/external-cli";
    assert_eq!(send_json(&router, Method::GET, path, None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send_json(&router, Method::POST, path, None, Some(json!({"provider":"codex"}))).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send_json(&router, Method::GET, "/api/mobile/work-items/missing/external-cli/run/events", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send_json(&router, Method::GET, path, Some("external-cli-token"), None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(send_json(&router, Method::GET, "/api/mobile/work-items/missing/external-cli/run/events", Some("external-cli-token"), None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remote_delete_work_item_protects_stale_and_active_workflows() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_delete_work_item").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "delete-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("delete-token"), Some(json!({"name":"Delete product"}))).await;
    let product = parse_json(&body);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("delete-token"), Some(json!({"product_id":product["id"],"title":"Delete parent"}))).await;
    let item = parse_json(&body);
    let id = item["id"].as_str().unwrap();
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("delete-token"), Some(json!({"product_id":product["id"],"title":"Keep child","parent_work_item_id":id,"work_item_type":"task"}))).await;
    let child = parse_json(&body);
    let expected = item.clone();
    let uri = format!("/api/mobile/work-items/{id}/delete");
    let request = json!({"expected":expected});
    assert_eq!(send_json(&router, Method::POST, &uri, None, Some(request.clone())).await.0, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE work_items SET description='changed' WHERE id=?").bind(id).execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("delete-token"), Some(request.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE work_items SET description=? WHERE id=?").bind(item["description"].as_str().unwrap()).bind(id).execute(&state.db).await.unwrap();
    workflow_repo::create_workflow_run(&state.db, "delete-child-active", child["id"].as_str().unwrap()).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("delete-token"), Some(request.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE workflow_runs SET status='failed',current_stage='failed' WHERE id='delete-child-active'").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("delete-token"), Some(request)).await.0, StatusCode::OK);
    let child = crate::persistence::work_item_repo::get_work_item(&state.db, child["id"].as_str().unwrap()).await.unwrap();
    assert!(child.parent_work_item_id.is_none());
}

#[tokio::test]
async fn remote_workspace_assignment_checks_conflicts_and_active_workflows() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_workspace_assignment").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "workspace-token").await.unwrap();
    crate::persistence::repository_repo::create_repository(&state.db, "workspace-a", "Workspace A", "/tmp/a", "", "main").await.unwrap();
    let router = build_webhook_router(state.clone());
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/products", Some("workspace-token"), Some(json!({"name":"Workspace product"}))).await;
    let product = parse_json(&body);
    let (_, body) = send_json(&router, Method::POST, "/api/mobile/work-items", Some("workspace-token"), Some(json!({"product_id":product["id"],"title":"Workspace item"}))).await;
    let item = parse_json(&body);
    let id = item["id"].as_str().unwrap();
    let uri = format!("/api/mobile/work-items/{id}/workspace");
    let initial = json!({"repo_override_id":null,"active_repo_id":null,"branch_name":null});
    let body = json!({"repository_id":"workspace-a","branch_name":"work/fixture","expected":initial});
    assert_eq!(send_json(&router, Method::POST, &uri, None, Some(body.clone())).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(json!({"repository_id":"missing","branch_name":"main","expected":initial}))).await.0, StatusCode::CONFLICT);
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(json!({"repository_id":"workspace-a","branch_name":"bad branch","expected":initial}))).await.0, StatusCode::CONFLICT);
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(body.clone())).await.0, StatusCode::OK);
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(body)).await.0, StatusCode::CONFLICT);
    let assigned = json!({"repo_override_id":"workspace-a","active_repo_id":"workspace-a","branch_name":"work/fixture"});
    workflow_repo::create_workflow_run(&state.db, "workspace-active", id).await.unwrap();
    let clear = json!({"repository_id":null,"branch_name":null,"expected":assigned});
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(clear.clone())).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE workflow_runs SET status='failed',current_stage='failed' WHERE id='workspace-active'").execute(&state.db).await.unwrap();
    assert_eq!(send_json(&router, Method::POST, &uri, Some("workspace-token"), Some(clear)).await.0, StatusCode::OK);
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
    let artifact_uri = format!("/api/mobile/work-items/{}/artifacts/{}", item["id"].as_str().unwrap(), artifact.id);
    assert_eq!(send_json(&router, Method::GET, &artifact_uri, None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, &artifact_uri, Some("result-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(parse_json(&body)["content"].as_str().unwrap().contains("passed"));
    assert_eq!(send_json(&router, Method::GET, "/api/mobile/work-items/other/artifacts/test-result", Some("result-token"), None).await.0, StatusCode::NOT_FOUND);
    tokio::fs::write(&path, vec![b'x'; 2 * 1024 * 1024 + 1]).await.unwrap();
    assert_eq!(send_json(&router, Method::GET, &artifact_uri, Some("result-token"), None).await.0, StatusCode::BAD_REQUEST);
    tokio::fs::write(&path, [0xff]).await.unwrap();
    assert_eq!(send_json(&router, Method::GET, &artifact_uri, Some("result-token"), None).await.0, StatusCode::BAD_REQUEST);
    #[cfg(unix)]
    {
        let link = state.artifact_base_path.join("outside-link");
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        sqlx::query("UPDATE artifacts SET storage_path=? WHERE id=?").bind(link.to_str().unwrap()).bind(&artifact.id).execute(&state.db).await.unwrap();
        assert_eq!(send_json(&router, Method::GET, &artifact_uri, Some("result-token"), None).await.0, StatusCode::NOT_FOUND);
    }
    let (status, _) = send_json(&router, Method::GET, "/api/mobile/work-items/other/test-results/test-result", Some("result-token"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    sqlx::query("UPDATE artifacts SET storage_path=? WHERE id=?").bind("/etc/passwd").bind(&artifact.id).execute(&state.db).await.unwrap();
    let (status, _) = send_json(&router, Method::GET, &uri, Some("result-token"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(send_json(&router, Method::GET, &artifact_uri, Some("result-token"), None).await.0, StatusCode::NOT_FOUND);
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

#[tokio::test]
async fn remote_catalog_settings_are_authenticated_conflict_aware_and_seed_examples() {
    let app: tauri::App<MockRuntime> = make_test_app("remote_catalog_settings").await;
    let state = app.state::<AppState>().inner().clone();
    settings_repo::set_setting(&state.db, "mobile.api_token", "catalog-test-token").await.unwrap();
    let router = build_webhook_router(state.clone());
    let uri = "/api/mobile/catalog-settings";
    let (status, _) = send_json(&router, Method::GET, uri, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/catalog-seed-examples", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = send_json(&router, Method::GET, uri, Some("catalog-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    let initial = parse_json(&body);
    let update = json!({"expected_raw":initial["raw"], "enabled":false});
    let (status, _) = send_json(&router, Method::POST, uri, Some("catalog-test-token"), Some(update.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send_json(&router, Method::POST, uri, Some("catalog-test-token"), Some(update)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/catalog-seed-examples", Some("catalog-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    let count = product_repo::list_products(&state.db).await.unwrap().len();
    assert!(count > 0);
    let (status, _) = send_json(&router, Method::POST, "/api/mobile/catalog-seed-examples", Some("catalog-test-token"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(product_repo::list_products(&state.db).await.unwrap().len(), count);
    let (_, body) = send_json(&router, Method::GET, uri, Some("catalog-test-token"), None).await;
    assert_eq!(parse_json(&body)["enabled"], false);
}

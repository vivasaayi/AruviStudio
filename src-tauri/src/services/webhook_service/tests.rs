use super::build_webhook_router;
use crate::commands::test_helpers::make_test_app;
use crate::persistence::{product_repo, settings_repo, workflow_repo};
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

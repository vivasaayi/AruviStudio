use crate::{error::AppError, persistence::prompt_template_repo, services::{webhook_bridge::ensure_mobile_api_authorized, webhook_service::WebhookState}};
use axum::{extract::{Json, State}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(crate) struct TemplateFields { name: String, role: String, template_text: String }
#[derive(Deserialize)]
pub(crate) struct TemplateEdit { id: String, expected_version: i64, fields: TemplateFields }
#[derive(Deserialize)]
pub(crate) struct TemplateDelete { id: String, expected_version: i64 }

fn error_response(error: AppError) -> Response {
    match error { AppError::Validation(message) => (StatusCode::CONFLICT, message).into_response(), _ => (StatusCode::INTERNAL_SERVER_ERROR, "Template operation failed").into_response() }
}
pub(crate) async fn list(State(state): State<WebhookState>, headers: HeaderMap) -> Response {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match prompt_template_repo::list(&state.app_state.db).await { Ok(items) => Json(items).into_response(), Err(error) => error_response(error) }
}
pub(crate) async fn create(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TemplateFields>) -> Response {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match prompt_template_repo::create(&state.app_state.db, &body.name, &body.role, &body.template_text).await { Ok(item) => (StatusCode::CREATED, Json(item)).into_response(), Err(error) => error_response(error) }
}
pub(crate) async fn update(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TemplateEdit>) -> Response {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match prompt_template_repo::update(&state.app_state.db, &body.id, body.expected_version, &body.fields.name, &body.fields.role, &body.fields.template_text).await { Ok(item) => Json(item).into_response(), Err(error) => error_response(error) }
}
pub(crate) async fn delete(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TemplateDelete>) -> Response {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match prompt_template_repo::delete(&state.app_state.db, &body.id, body.expected_version).await { Ok(()) => Json(json!({"deleted":true})).into_response(), Err(error) => error_response(error) }
}

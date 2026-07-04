use crate::persistence::{model_repo, planner_repo};
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::ChatMessage;
use crate::secrets;
use crate::services::webhook_bridge::ensure_mobile_api_authorized;
use crate::services::webhook_mobile_model::{
    resolve_mobile_chat_model_name, resolve_mobile_chat_provider_id,
};
use crate::services::webhook_mobile_planner_chat_support::resolve_mobile_planner_product_context;
use crate::services::webhook_mobile_planner_chat_turn::{
    run_mobile_planner_chat_turn, MobilePlannerChatTurnInput,
};
use crate::services::webhook_service::WebhookState;
use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerChatSessionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    product_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MobilePlannerChatSessionResponse {
    session_id: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerChatTurnRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    product_id: Option<String>,
    messages: Vec<ChatMessage>,
    max_tool_steps: Option<u8>,
}

pub(crate) async fn mobile_create_planner_chat_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobilePlannerChatSessionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let provider_id =
        match resolve_mobile_chat_provider_id(&state.app_state, body.provider_id).await {
            Ok(provider_id) => provider_id,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let model_name =
        match resolve_mobile_chat_model_name(&state.app_state, &provider_id, body.model_name).await
        {
            Ok(model_name) => model_name,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let active_product = match resolve_mobile_planner_product_context(
        &state.app_state,
        body.product_id.as_deref(),
        None,
        None,
    )
    .await
    {
        Ok(context) => context,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    match planner_repo::create_mobile_planner_chat_session(
        &state.app_state.db,
        &session_id,
        Some(&provider_id),
        Some(&model_name),
        active_product.as_ref().map(|product| product.id.as_str()),
        active_product.as_ref().map(|product| product.name.as_str()),
    )
    .await
    {
        Ok(session) => Json(MobilePlannerChatSessionResponse {
            session_id: session.id,
            provider_id,
            model_name,
            product_id: session.active_product_id,
            product_name: session.active_product_name,
        })
        .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_submit_planner_chat_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerChatTurnRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let messages = body
        .messages
        .into_iter()
        .filter(|message| !message.content.trim().is_empty())
        .collect::<Vec<_>>();
    if messages.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "At least one planner chat message is required.",
        )
            .into_response();
    }
    let latest_user_input = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.trim().to_string())
        .filter(|content| !content.is_empty());
    let Some(latest_user_input) = latest_user_input else {
        return (
            StatusCode::BAD_REQUEST,
            "Planner chat turns require a user message.",
        )
            .into_response();
    };

    let provider_id =
        match resolve_mobile_chat_provider_id(&state.app_state, body.provider_id).await {
            Ok(provider_id) => provider_id,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let model_name =
        match resolve_mobile_chat_model_name(&state.app_state, &provider_id, body.model_name).await
        {
            Ok(model_name) => model_name,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let provider = match model_repo::get_provider(&state.app_state.db, &provider_id).await {
        Ok(provider) if provider.enabled => provider,
        Ok(_) => {
            return (StatusCode::BAD_REQUEST, "Selected provider is disabled.").into_response()
        }
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let api_key = match secrets::resolve_provider_secret(&provider) {
        Ok(api_key) => api_key,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let existing_session =
        match planner_repo::get_mobile_planner_chat_session(&state.app_state.db, &session_id).await
        {
            Ok(session) => session,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    let active_product = match resolve_mobile_planner_product_context(
        &state.app_state,
        body.product_id.as_deref(),
        existing_session.active_product_id.as_deref(),
        Some(&latest_user_input),
    )
    .await
    {
        Ok(context) => context,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let updated_session = match planner_repo::update_mobile_planner_chat_session(
        &state.app_state.db,
        &session_id,
        Some(&provider_id),
        Some(&model_name),
        active_product.as_ref().map(|product| product.id.as_str()),
        active_product.as_ref().map(|product| product.name.as_str()),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if let Err(error) = planner_repo::append_mobile_planner_chat_message(
        &state.app_state.db,
        &uuid::Uuid::new_v4().to_string(),
        &session_id,
        "user",
        &latest_user_input,
    )
    .await
    {
        return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
    }
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
    match run_mobile_planner_chat_turn(
        &state.app_state,
        MobilePlannerChatTurnInput {
            gateway: &gateway,
            provider: &provider,
            session_id,
            provider_id,
            model_name,
            product_id: updated_session.active_product_id,
            product_name: updated_session.active_product_name,
            max_tool_steps: body.max_tool_steps.unwrap_or(4).clamp(1, 8),
        },
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

use crate::mcp;
use crate::persistence::{model_call_repo, model_repo, planner_repo, product_repo, settings_repo};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
use crate::services::channel_service::{
    handle_inbound_message, resolve_twilio_config, ChannelInboundMessage,
};
use crate::services::planner_service::{
    clear_planner_pending, confirm_planner_plan, create_planner_session, submit_planner_turn,
    submit_planner_voice_turn, update_planner_session,
};
use crate::services::product_service::HIDE_EXAMPLE_PRODUCTS_KEY;
use crate::services::speech_service::{
    transcribe_audio_with_provider, SpeechToTextRequest, SpeechToTextResponse,
};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::Sha1;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::Instant;
use tracing::{error, info};

const MOBILE_API_TOKEN_KEY: &str = "mobile.api_token";
const MCP_API_TOKEN_KEY: &str = "mcp.api_token";
pub const MOBILE_BIND_HOST_KEY: &str = "mobile.bind_host";
pub const MOBILE_BIND_PORT_KEY: &str = "mobile.bind_port";
const SPEECH_PROVIDER_KEY: &str = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY: &str = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY: &str = "speech.locale";

fn char_count_i64(content: &str) -> i64 {
    i64::try_from(content.chars().count()).unwrap_or(i64::MAX)
}

fn message_char_count(messages: &[ChatMessage]) -> i64 {
    messages
        .iter()
        .map(|message| char_count_i64(&message.content))
        .sum()
}

fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

struct WebhookModelCallContext<'a> {
    source_kind: &'a str,
    source_id: Option<&'a str>,
    source_label: &'a str,
    session_id: Option<&'a str>,
    product_id: Option<&'a str>,
}

async fn record_webhook_model_call(
    state: &AppState,
    context: WebhookModelCallContext<'_>,
    provider: &crate::domain::model::ModelProvider,
    model_name: &str,
    messages: &[ChatMessage],
    max_tokens: Option<i64>,
    temperature: Option<f64>,
    response_chars: i64,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
    duration_ms: i64,
    status: &str,
    error_message: Option<&str>,
    response_text: Option<&str>,
) -> Result<(), crate::error::AppError> {
    let call_index =
        model_call_repo::next_model_call_index(&state.db, context.source_kind, context.source_id)
            .await?;
    let call_id = uuid::Uuid::new_v4().to_string();
    let request_messages_json = serde_json::to_string_pretty(messages)?;
    let snapshots = model_call_repo::write_model_call_snapshots(
        &state.artifact_base_path,
        &call_id,
        Some(&request_messages_json),
        response_text,
    )
    .await?;
    model_call_repo::create_model_call(
        &state.db,
        model_call_repo::CreateModelCallParams {
            id: &call_id,
            source_kind: context.source_kind,
            source_id: context.source_id,
            source_label: context.source_label,
            workflow_run_id: None,
            agent_run_id: None,
            work_item_id: None,
            product_id: context.product_id,
            session_id: context.session_id,
            agent_id: None,
            stage: None,
            provider_id: &provider.id,
            provider_name: &provider.name,
            provider_type: provider.provider_type.as_str(),
            provider_base_url: &provider.base_url,
            model_id: None,
            model_name,
            call_index,
            request_message_count: i64::try_from(messages.len()).unwrap_or(i64::MAX),
            prompt_chars: message_char_count(messages),
            response_chars,
            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms: Some(duration_ms),
            status,
            error_message,
        },
    )
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct WebhookState {
    pub app_state: AppState,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookBindConfig {
    pub host: String,
    pub port: u16,
    pub host_source: String,
    pub port_source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileBridgeStatus {
    pub bind_host: String,
    pub bind_port: u16,
    pub host_source: String,
    pub port_source: String,
    pub bind_scope: String,
    pub detected_lan_ip: Option<String>,
    pub desktop_base_url: String,
    pub phone_base_url: Option<String>,
    pub lan_ready: bool,
    pub bind_changes_require_restart: bool,
    pub env_overrides_settings: bool,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpBridgeStatus {
    pub bind_host: String,
    pub bind_port: u16,
    pub host_source: String,
    pub port_source: String,
    pub bind_scope: String,
    pub detected_lan_ip: Option<String>,
    pub desktop_base_url: String,
    pub lan_base_url: Option<String>,
    pub endpoint_url: String,
    pub lan_endpoint_url: Option<String>,
    pub token_configured: bool,
    pub requests_allowed: bool,
    pub auth_mode: String,
    pub origin_policy: String,
    pub bind_changes_require_restart: bool,
    pub env_overrides_settings: bool,
    pub guidance: String,
}

#[derive(Debug, Deserialize)]
struct TwilioMessagingForm {
    #[serde(rename = "Body")]
    body: Option<String>,
    #[serde(rename = "From")]
    from: Option<String>,
    #[serde(rename = "WaId")]
    wa_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwilioVoiceForm {
    #[serde(rename = "CallSid")]
    call_sid: Option<String>,
    #[serde(rename = "From")]
    from: Option<String>,
    #[serde(rename = "SpeechResult")]
    speech_result: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MobilePlannerSessionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelCallListQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MobilePlannerUpdateRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MobilePlannerTurnRequest {
    user_input: String,
    selected_draft_node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MobileChatCompletionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MobilePlannerChatSessionRequest {
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
struct MobilePlannerChatTurnRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    product_id: Option<String>,
    messages: Vec<ChatMessage>,
    max_tool_steps: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
struct MobilePlannerToolTraceEntry {
    step: u8,
    tool_name: String,
    arguments: Value,
    result: Option<Value>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MobilePlannerChatTurnResponse {
    session_id: String,
    status: String,
    assistant_message: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
    tool_trace: Vec<MobilePlannerToolTraceEntry>,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MobileSpeechTranscriptionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    audio_bytes_base64: String,
    mime_type: String,
    locale: Option<String>,
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn messaging_twiml(message: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Message>{}</Message></Response>"#,
        xml_escape(message)
    )
}

fn voice_gather_twiml(prompt: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
  <Say>{}</Say>
  <Gather input="speech" action="/webhooks/twilio/voice/gather" method="POST" speechTimeout="auto" />
  <Redirect method="POST">/webhooks/twilio/voice</Redirect>
</Response>"#,
        xml_escape(prompt)
    )
}

fn planner_reply_text(response: &crate::services::planner_service::PlannerTurnResponse) -> String {
    let mut parts = vec![response.assistant_message.clone()];
    if !response.execution_lines.is_empty() {
        parts.extend(response.execution_lines.clone());
    }
    if !response.execution_errors.is_empty() {
        parts.push(format!("Errors: {}", response.execution_errors.join(" | ")));
    }
    if response.status == "proposal" {
        parts.push("Reply confirm to apply the proposal.".to_string());
    }
    parts.join("\n")
}

async fn healthcheck() -> impl IntoResponse {
    "ok"
}

async fn remote_app() -> impl IntoResponse {
    Html(REMOTE_APP_HTML)
}

async fn mobile_healthcheck(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    Json(serde_json::json!({
        "status": "ok",
        "service": "aruvi-mobile-api",
    }))
    .into_response()
}

async fn mobile_list_model_calls(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Query(query): Query<ModelCallListQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match model_call_repo::list_model_calls(&state.app_state.db, query.limit.unwrap_or(100)).await {
        Ok(calls) => Json(calls).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_get_model_call(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(call_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match model_call_repo::get_model_call(&state.app_state.db, &call_id).await {
        Ok(call) => Json(call).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_list_products(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let hide_examples =
        match settings_repo::get_bool_setting(&state.app_state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true)
            .await
        {
            Ok(value) => value,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    let mut products = match product_repo::list_products(&state.app_state.db).await {
        Ok(products) => products,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if hide_examples {
        products.retain(|product| !product.is_example_product());
    }
    Json(products).into_response()
}

async fn mobile_get_product_tree(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match product_repo::get_product_tree(&state.app_state.db, &product_id).await {
        Ok(tree) => Json(tree).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_create_planner_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobilePlannerSessionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match create_planner_session(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        body.provider_id,
        body.model_name,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_update_planner_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerUpdateRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match update_planner_session(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        session_id,
        body.provider_id,
        body.model_name,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_submit_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerTurnRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match submit_planner_turn(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
        body.user_input,
        body.selected_draft_node_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_submit_planner_voice_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerTurnRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match submit_planner_voice_turn(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
        body.user_input,
        body.selected_draft_node_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_confirm_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match confirm_planner_plan(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_clear_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match clear_planner_pending(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        session_id,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_chat_completion(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileChatCompletionRequest>,
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
            "At least one chat message is required.",
        )
            .into_response();
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
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
    let max_tokens = body.max_tokens.or(Some(4096));
    let temperature = body.temperature;
    let started = Instant::now();
    match gateway
        .run_completion(CompletionRequest {
            model: model_name.clone(),
            messages: messages.clone(),
            temperature,
            max_tokens,
        })
        .await
    {
        Ok(response) => {
            if let Err(record_error) = record_webhook_model_call(
                &state.app_state,
                WebhookModelCallContext {
                    source_kind: "mobile_chat",
                    source_id: None,
                    source_label: "Mobile Chat",
                    session_id: None,
                    product_id: None,
                },
                &provider,
                &model_name,
                &messages,
                max_tokens,
                temperature,
                char_count_i64(&response.content),
                response.token_count_input,
                response.token_count_output,
                elapsed_ms(started),
                "completed",
                None,
                Some(&response.content),
            )
            .await
            {
                error!(error = %record_error, "Failed to record mobile chat telemetry");
            }
            Json::<CompletionResponse>(response).into_response()
        }
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_webhook_model_call(
                &state.app_state,
                WebhookModelCallContext {
                    source_kind: "mobile_chat",
                    source_id: None,
                    source_label: "Mobile Chat",
                    session_id: None,
                    product_id: None,
                },
                &provider,
                &model_name,
                &messages,
                max_tokens,
                temperature,
                0,
                None,
                None,
                elapsed_ms(started),
                "failed",
                Some(&error_message),
                None,
            )
            .await
            {
                error!(error = %record_error, "Failed to record failed mobile chat telemetry");
            }
            (StatusCode::BAD_REQUEST, error_message).into_response()
        }
    }
}

async fn mobile_create_planner_chat_session(
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

async fn mobile_submit_planner_chat_turn(
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
        &gateway,
        &provider,
        session_id,
        provider_id,
        model_name,
        updated_session.active_product_id,
        updated_session.active_product_name,
        body.max_tool_steps.unwrap_or(4).clamp(1, 8),
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

async fn run_mobile_planner_chat_turn(
    state: &AppState,
    gateway: &OpenAiCompatibleProvider,
    provider: &crate::domain::model::ModelProvider,
    session_id: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
    max_tool_steps: u8,
) -> Result<MobilePlannerChatTurnResponse, String> {
    let persisted_messages =
        planner_repo::list_mobile_planner_chat_messages(&state.db, &session_id, 24)
            .await
            .map_err(|error| error.to_string())?;
    let prior_tool_traces =
        planner_repo::list_mobile_planner_chat_tool_traces(&state.db, &session_id, 12)
            .await
            .map_err(|error| error.to_string())?;

    let mut conversation =
        Vec::with_capacity(persisted_messages.len() + prior_tool_traces.len() + 4);
    conversation.push(ChatMessage {
        role: "system".to_string(),
        content: build_mobile_planner_system_prompt(product_id.as_deref(), product_name.as_deref()),
    });
    for message in persisted_messages {
        conversation.push(ChatMessage {
            role: message.role,
            content: message.content,
        });
    }
    if !prior_tool_traces.is_empty() {
        let mut trace_context =
            String::from("Recent persisted MCP tool observations for this planner session:\n");
        for trace in prior_tool_traces {
            trace_context.push_str("- ");
            trace_context.push_str(&trace.tool_name);
            trace_context.push_str(" args=");
            trace_context.push_str(&truncate_for_prompt(&trace.arguments_json, 600));
            if let Some(error) = trace.error {
                trace_context.push_str(" error=");
                trace_context.push_str(&truncate_for_prompt(&error, 400));
            } else if let Some(result_json) = trace.result_json {
                trace_context.push_str(" result=");
                trace_context.push_str(&truncate_for_prompt(&result_json, 1200));
            }
            trace_context.push('\n');
        }
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: trace_context,
        });
    }

    let mut tool_trace = Vec::new();
    let mut token_count_input = 0_i64;
    let mut token_count_output = 0_i64;

    for step in 1..=max_tool_steps {
        let max_tokens = Some(4096);
        let temperature = Some(0.2);
        let started = Instant::now();
        let completion = match gateway
            .run_completion(CompletionRequest {
                model: model_name.clone(),
                messages: conversation.clone(),
                temperature,
                max_tokens,
            })
            .await
        {
            Ok(completion) => completion,
            Err(error) => {
                let error_message = error.to_string();
                if let Err(record_error) = record_webhook_model_call(
                    state,
                    WebhookModelCallContext {
                        source_kind: "mobile_planner_chat",
                        source_id: Some(&session_id),
                        source_label: "Mobile Planner Chat",
                        session_id: Some(&session_id),
                        product_id: product_id.as_deref(),
                    },
                    provider,
                    &model_name,
                    &conversation,
                    max_tokens,
                    temperature,
                    0,
                    None,
                    None,
                    elapsed_ms(started),
                    "failed",
                    Some(&error_message),
                    None,
                )
                .await
                {
                    error!(error = %record_error, "Failed to record failed mobile planner telemetry");
                }
                return Err(error_message);
            }
        };
        if let Err(record_error) = record_webhook_model_call(
            state,
            WebhookModelCallContext {
                source_kind: "mobile_planner_chat",
                source_id: Some(&session_id),
                source_label: "Mobile Planner Chat",
                session_id: Some(&session_id),
                product_id: product_id.as_deref(),
            },
            provider,
            &model_name,
            &conversation,
            max_tokens,
            temperature,
            char_count_i64(&completion.content),
            completion.token_count_input,
            completion.token_count_output,
            elapsed_ms(started),
            "completed",
            None,
            Some(&completion.content),
        )
        .await
        {
            error!(error = %record_error, "Failed to record mobile planner telemetry");
        }
        if let Some(tokens) = completion.token_count_input {
            token_count_input += tokens;
        }
        if let Some(tokens) = completion.token_count_output {
            token_count_output += tokens;
        }

        let model_output = completion.content.trim().to_string();
        let Some(decision) = extract_json_payload(&model_output)
            .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        else {
            persist_mobile_planner_assistant_message(state, &session_id, &model_output).await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message: model_output,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        };

        let decision_type = decision
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        if decision_type == "final" {
            let assistant_message = decision
                .get("message")
                .or_else(|| decision.get("assistant_message"))
                .or_else(|| decision.get("summary"))
                .and_then(Value::as_str)
                .unwrap_or(&model_output)
                .trim()
                .to_string();
            persist_mobile_planner_assistant_message(state, &session_id, &assistant_message)
                .await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        }

        if decision_type != "tool_call" {
            persist_mobile_planner_assistant_message(state, &session_id, &model_output).await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message: model_output,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        }

        let tool_name = decision
            .get("tool")
            .or_else(|| decision.get("tool_name"))
            .or_else(|| decision.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Planner tool call is missing tool name.".to_string())?
            .to_string();
        let arguments = decision
            .get("arguments")
            .or_else(|| decision.get("args"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let step_trace =
            match execute_mobile_planner_mcp_tool(state, step, &tool_name, arguments.clone()).await
            {
                Ok(result) => MobilePlannerToolTraceEntry {
                    step,
                    tool_name: tool_name.clone(),
                    arguments: arguments.clone(),
                    result: Some(result.clone()),
                    error: None,
                },
                Err(error) => MobilePlannerToolTraceEntry {
                    step,
                    tool_name: tool_name.clone(),
                    arguments: arguments.clone(),
                    result: None,
                    error: Some(error),
                },
            };
        persist_mobile_planner_tool_trace(state, &session_id, &step_trace).await?;
        let tool_observation = serde_json::to_string(&step_trace)
            .unwrap_or_else(|_| "{\"error\":\"failed to serialize tool trace\"}".to_string());
        tool_trace.push(step_trace);
        conversation.push(ChatMessage {
            role: "assistant".to_string(),
            content: model_output,
        });
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Tool observation for step {step}: {tool_observation}\nContinue. Return another tool_call only if essential; otherwise return type=final with a natural mobile-friendly summary of what you did, including created/updated item names and where they were added. End with a short follow-up invitation if another refinement would be useful."
            ),
        });
    }

    conversation.push(ChatMessage {
        role: "user".to_string(),
        content: "You reached the mobile planner tool-step limit. Return exactly one JSON object with type=final. In message, give a natural mobile-friendly summary of what you learned or changed, list created/updated item names when available, and invite one concise follow-up question. Do not call another tool.".to_string(),
    });
    let max_tokens = Some(2048);
    let temperature = Some(0.2);
    let started = Instant::now();
    let completion = match gateway
        .run_completion(CompletionRequest {
            model: model_name.clone(),
            messages: conversation.clone(),
            temperature,
            max_tokens,
        })
        .await
    {
        Ok(completion) => completion,
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_webhook_model_call(
                state,
                WebhookModelCallContext {
                    source_kind: "mobile_planner_chat",
                    source_id: Some(&session_id),
                    source_label: "Mobile Planner Chat",
                    session_id: Some(&session_id),
                    product_id: product_id.as_deref(),
                },
                provider,
                &model_name,
                &conversation,
                max_tokens,
                temperature,
                0,
                None,
                None,
                elapsed_ms(started),
                "failed",
                Some(&error_message),
                None,
            )
            .await
            {
                error!(error = %record_error, "Failed to record failed mobile planner telemetry");
            }
            return Err(error_message);
        }
    };
    if let Err(record_error) = record_webhook_model_call(
        state,
        WebhookModelCallContext {
            source_kind: "mobile_planner_chat",
            source_id: Some(&session_id),
            source_label: "Mobile Planner Chat",
            session_id: Some(&session_id),
            product_id: product_id.as_deref(),
        },
        provider,
        &model_name,
        &conversation,
        max_tokens,
        temperature,
        char_count_i64(&completion.content),
        completion.token_count_input,
        completion.token_count_output,
        elapsed_ms(started),
        "completed",
        None,
        Some(&completion.content),
    )
    .await
    {
        error!(error = %record_error, "Failed to record mobile planner telemetry");
    }
    if let Some(tokens) = completion.token_count_input {
        token_count_input += tokens;
    }
    if let Some(tokens) = completion.token_count_output {
        token_count_output += tokens;
    }
    let assistant_message = extract_json_payload(&completion.content)
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| {
            value
                .get("message")
                .or_else(|| value.get("assistant_message"))
                .or_else(|| value.get("summary"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| completion.content.trim().to_string());
    persist_mobile_planner_assistant_message(state, &session_id, &assistant_message).await?;

    Ok(MobilePlannerChatTurnResponse {
        session_id,
        status: "tool_limit_final".to_string(),
        assistant_message,
        provider_id,
        model_name,
        product_id,
        product_name,
        tool_trace,
        token_count_input: non_zero_token_count(token_count_input),
        token_count_output: non_zero_token_count(token_count_output),
    })
}

fn build_mobile_planner_system_prompt(
    product_id: Option<&str>,
    product_name: Option<&str>,
) -> String {
    let product_context = product_id
        .map(|id| {
            format!(
                "Current selected product: {} ({id})\n",
                product_name.unwrap_or("unknown")
            )
        })
        .unwrap_or_else(|| {
            "No product is selected yet. Use catalog.products.list if needed.\n".to_string()
        });
    format!(
        "You are Aruvi Studio's first-class mobile planner.\n\
{product_context}\
Use MCP tools to inspect or update the product plan when the user asks for planning work. \
Prefer the selected product when one is provided. Keep replies short enough for mobile.\n\
\n\
Allowed MCP tools:\n\
- catalog.products.list, catalog.products.get, catalog.products.get_tree\n\
- catalog.modules.list, catalog.modules.create, catalog.modules.update, catalog.modules.reorder\n\
- catalog.capabilities.list, catalog.capabilities.create, catalog.capabilities.update, catalog.capabilities.reorder, catalog.capabilities.apply_template, catalog.capabilities.convert_kind\n\
- work_items.list, work_items.get, work_items.create, work_items.update, work_items.list_children, work_items.summarize_by_product\n\
- repositories.list, repositories.resolution.for_scope, repositories.resolution.for_work_item, repositories.trees.list, repositories.files.read\n\
\n\
Return exactly one JSON object, with no markdown.\n\
To call a tool: {{\"type\":\"tool_call\",\"tool\":\"catalog.products.get_tree\",\"arguments\":{{\"productId\":\"...\"}},\"reason\":\"...\"}}\n\
To answer: {{\"type\":\"final\",\"message\":\"...\"}}\n\
Final message style: be natural and explicit. If you changed data, say what changed, name the created/updated items, say where they were added, and end with a short follow-up invitation such as \"Want me to split any of these further?\".\n\
Before creating hundreds of nodes, inspect the existing tree and create a small useful slice unless the user explicitly asks for a broad commit. \
If you mutate catalog or work items, mention the exact objects changed in the final message."
    )
}

async fn resolve_mobile_planner_product_context(
    state: &AppState,
    requested_product_id: Option<&str>,
    current_product_id: Option<&str>,
    user_input: Option<&str>,
) -> Result<Option<crate::domain::product::Product>, String> {
    if let Some(product_id) = requested_product_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return product_repo::get_product(&state.db, product_id)
            .await
            .map(Some)
            .map_err(|error| error.to_string());
    }

    let products = product_repo::list_products(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(input) = user_input.map(normalize_product_match_text) {
        let mut matches = products
            .iter()
            .filter(|product| {
                let product_name = normalize_product_match_text(&product.name);
                !product_name.is_empty() && input.contains(&product_name)
            })
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.name.len().cmp(&left.name.len()));
        if let Some(product) = matches.into_iter().next() {
            return Ok(Some(product));
        }
    }

    if let Some(product_id) = current_product_id {
        if let Some(product) = products
            .into_iter()
            .find(|product| product.id == product_id)
        {
            return Ok(Some(product));
        }
    }

    Ok(None)
}

fn normalize_product_match_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

async fn persist_mobile_planner_assistant_message(
    state: &AppState,
    session_id: &str,
    content: &str,
) -> Result<(), String> {
    planner_repo::append_mobile_planner_chat_message(
        &state.db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        "assistant",
        content,
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn persist_mobile_planner_tool_trace(
    state: &AppState,
    session_id: &str,
    trace: &MobilePlannerToolTraceEntry,
) -> Result<(), String> {
    let arguments_json =
        serde_json::to_string(&trace.arguments).map_err(|error| error.to_string())?;
    let result_json = trace
        .result
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    planner_repo::append_mobile_planner_chat_tool_trace(
        &state.db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        i64::from(trace.step),
        &trace.tool_name,
        &arguments_json,
        result_json.as_deref(),
        trace.error.as_deref(),
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn truncate_for_prompt(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for ch in value.chars().take(max_chars) {
        output.push(ch);
    }
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

async fn execute_mobile_planner_mcp_tool(
    state: &AppState,
    step: u8,
    tool_name: &str,
    arguments: Value,
) -> Result<Value, String> {
    if !is_mobile_planner_tool_allowed(tool_name) {
        return Err(format!(
            "Tool is not allowed in mobile planner chat: {tool_name}"
        ));
    }
    let response = mcp::handle_json_rpc_value(
        state,
        json!({
            "jsonrpc": "2.0",
            "id": format!("mobile-planner-chat-{step}"),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments,
            }
        }),
    )
    .await
    .ok_or_else(|| "MCP tool did not return a response.".to_string())?;

    if let Some(error) = response.get("error") {
        return Err(error.to_string());
    }
    let result = response
        .get("result")
        .cloned()
        .ok_or_else(|| "MCP response is missing result.".to_string())?;
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(render_mcp_tool_text(&result).unwrap_or_else(|| result.to_string()));
    }
    Ok(result.get("structuredContent").cloned().unwrap_or(result))
}

fn is_mobile_planner_tool_allowed(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "catalog.products.list"
            | "catalog.products.get"
            | "catalog.products.get_tree"
            | "catalog.modules.list"
            | "catalog.modules.create"
            | "catalog.modules.update"
            | "catalog.modules.reorder"
            | "catalog.capabilities.list"
            | "catalog.capabilities.create"
            | "catalog.capabilities.update"
            | "catalog.capabilities.reorder"
            | "catalog.capabilities.apply_template"
            | "catalog.capabilities.convert_kind"
            | "work_items.list"
            | "work_items.get"
            | "work_items.create"
            | "work_items.update"
            | "work_items.list_children"
            | "work_items.summarize_by_product"
            | "repositories.list"
            | "repositories.resolution.for_scope"
            | "repositories.resolution.for_work_item"
            | "repositories.trees.list"
            | "repositories.files.read"
    )
}

fn render_mcp_tool_text(result: &Value) -> Option<String> {
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn non_zero_token_count(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn extract_json_payload(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(trimmed[start..=end].to_string())
}

async fn mobile_transcribe_audio(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileSpeechTranscriptionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }

    let (default_provider_id, default_model_name, default_locale) =
        match resolve_mobile_speech_defaults(&state.app_state).await {
            Ok(values) => values,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let provider_id = body
        .provider_id
        .filter(|value| !value.trim().is_empty())
        .or(default_provider_id);
    let Some(provider_id) = provider_id else {
        return (
            StatusCode::BAD_REQUEST,
            "A speech transcription provider is required.",
        )
            .into_response();
    };
    let model_name = body
        .model_name
        .filter(|value| !value.trim().is_empty())
        .or(default_model_name)
        .unwrap_or_else(|| "whisper-1".to_string());
    let request = SpeechToTextRequest {
        audio_bytes_base64: body.audio_bytes_base64,
        mime_type: body.mime_type,
        locale: body.locale.or(default_locale),
    };
    let provider = match model_repo::get_provider(&state.app_state.db, &provider_id).await {
        Ok(provider) => provider,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    match transcribe_audio_with_provider(&provider, &model_name, request).await {
        Ok(response) => Json::<SpeechToTextResponse>(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

fn validate_twilio_signature(
    auth_token: Option<&str>,
    base_url: Option<&str>,
    path: &str,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
) -> Result<(), String> {
    let Some(auth_token) = auth_token else {
        return Ok(());
    };
    let Some(base_url) = base_url else {
        return Ok(());
    };
    let Some(signature) = headers
        .get("X-Twilio-Signature")
        .and_then(|value| value.to_str().ok())
    else {
        return Err("Missing X-Twilio-Signature header".to_string());
    };

    let mut data = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut sorted = params.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    for (key, value) in sorted {
        data.push_str(key);
        data.push_str(value);
    }

    let mut mac =
        Hmac::<Sha1>::new_from_slice(auth_token.as_bytes()).map_err(|error| error.to_string())?;
    mac.update(data.as_bytes());
    let expected = STANDARD.encode(mac.finalize().into_bytes());
    if expected != signature {
        return Err("Invalid Twilio signature".to_string());
    }
    Ok(())
}

async fn resolve_mobile_api_token(state: &AppState) -> Result<Option<String>, String> {
    Ok(std::env::var("ARUVI_MOBILE_API_TOKEN")
        .ok()
        .or(settings_repo::get_setting(&state.db, MOBILE_API_TOKEN_KEY)
            .await
            .map_err(|error| error.to_string())?))
}

pub async fn resolve_mcp_api_token(state: &AppState) -> Result<Option<String>, String> {
    Ok(std::env::var("ARUVI_MCP_API_TOKEN")
        .ok()
        .or(settings_repo::get_setting(&state.db, MCP_API_TOKEN_KEY)
            .await
            .map_err(|error| error.to_string())?))
}

fn configured_token(token: Option<String>) -> Option<String> {
    token.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn provided_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-aruvi-token")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn unauthorized(message: impl Into<String>) -> Response {
    (StatusCode::UNAUTHORIZED, message.into()).into_response()
}

fn unavailable(message: impl Into<String>) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, message.into()).into_response()
}

async fn ensure_mobile_api_authorized(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Response> {
    let configured_token =
        configured_token(resolve_mobile_api_token(state).await.map_err(unavailable)?);
    let Some(configured_token) = configured_token else {
        return Err(unavailable(
            "Mobile API token is not configured. Set mobile.api_token first.",
        ));
    };

    let provided_token = provided_bearer_token(headers);

    match provided_token {
        Some(candidate) if candidate == configured_token => Ok(()),
        _ => Err(unauthorized("Mobile API authorization failed.")),
    }
}

async fn resolve_mobile_speech_defaults(
    state: &AppState,
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let provider_id =
        std::env::var("ARUVI_SPEECH_PROVIDER_ID")
            .ok()
            .or(settings_repo::get_setting(&state.db, SPEECH_PROVIDER_KEY)
                .await
                .map_err(|error| error.to_string())?);
    let model_name = std::env::var("ARUVI_SPEECH_MODEL_NAME")
        .ok()
        .or(settings_repo::get_setting(&state.db, SPEECH_MODEL_KEY)
            .await
            .map_err(|error| error.to_string())?);
    let locale = std::env::var("ARUVI_SPEECH_LOCALE")
        .ok()
        .or(settings_repo::get_setting(&state.db, SPEECH_LOCALE_KEY)
            .await
            .map_err(|error| error.to_string())?);
    Ok((provider_id, model_name, locale))
}

async fn resolve_mobile_chat_provider_id(
    state: &AppState,
    requested_provider_id: Option<String>,
) -> Result<String, String> {
    if let Some(provider_id) = requested_provider_id.filter(|value| !value.trim().is_empty()) {
        return Ok(provider_id);
    }
    model_repo::list_providers(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|provider| provider.enabled)
        .map(|provider| provider.id)
        .ok_or_else(|| "No enabled model provider is configured.".to_string())
}

async fn resolve_mobile_chat_model_name(
    state: &AppState,
    provider_id: &str,
    requested_model_name: Option<String>,
) -> Result<String, String> {
    if let Some(model_name) = requested_model_name.filter(|value| !value.trim().is_empty()) {
        return Ok(model_name);
    }
    model_repo::list_model_definitions(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|model| model.enabled && model.provider_id == provider_id)
        .map(|model| model.name)
        .ok_or_else(|| "No enabled model is configured for the selected provider.".to_string())
}

fn parse_bind_host(value: Option<String>) -> Option<String> {
    value.and_then(|host| {
        let trimmed = host.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_bind_port(value: Option<String>) -> Option<u16> {
    value.and_then(|port| port.trim().parse::<u16>().ok())
}

pub async fn resolve_webhook_bind_config(state: &AppState) -> Result<WebhookBindConfig, String> {
    let env_host = parse_bind_host(std::env::var("ARUVI_WEBHOOK_HOST").ok());
    let settings_host = parse_bind_host(
        settings_repo::get_setting(&state.db, MOBILE_BIND_HOST_KEY)
            .await
            .map_err(|error| error.to_string())?,
    );
    let (host, host_source) = if let Some(host) = env_host {
        (host, "env".to_string())
    } else if let Some(host) = settings_host {
        (host, "settings".to_string())
    } else {
        ("127.0.0.1".to_string(), "default".to_string())
    };

    let env_port = parse_bind_port(std::env::var("ARUVI_WEBHOOK_PORT").ok());
    let settings_port = parse_bind_port(
        settings_repo::get_setting(&state.db, MOBILE_BIND_PORT_KEY)
            .await
            .map_err(|error| error.to_string())?,
    );
    let (port, port_source) = if let Some(port) = env_port {
        (port, "env".to_string())
    } else if let Some(port) = settings_port {
        (port, "settings".to_string())
    } else {
        (8787, "default".to_string())
    };

    Ok(WebhookBindConfig {
        host,
        port,
        host_source,
        port_source,
    })
}

pub fn detect_primary_lan_ip() -> Option<String> {
    for probe in ["8.8.8.8:80", "1.1.1.1:80"] {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
            continue;
        };
        if socket.connect(probe).is_err() {
            continue;
        }
        let Ok(local_addr) = socket.local_addr() else {
            continue;
        };
        match local_addr.ip() {
            IpAddr::V4(ip) if !ip.is_loopback() => return Some(ip.to_string()),
            _ => continue,
        }
    }
    None
}

pub fn classify_bind_scope(host: &str) -> &'static str {
    match host.trim() {
        "127.0.0.1" | "localhost" | "::1" => "localhost-only",
        "0.0.0.0" | "::" => "lan",
        _ => "custom",
    }
}

fn format_http_host(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub fn build_desktop_base_url(host: &str, port: u16) -> String {
    let normalized_host = match host.trim() {
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        other => other,
    };
    format!("http://{}:{port}", format_http_host(normalized_host))
}

pub fn build_phone_base_url(host: &str, port: u16, lan_ip: Option<&str>) -> Option<String> {
    let phone_host = match host.trim() {
        "127.0.0.1" | "localhost" | "::1" => return None,
        "0.0.0.0" | "::" => lan_ip?,
        other => other,
    };
    Some(format!("http://{}:{port}", format_http_host(phone_host)))
}

fn build_mcp_endpoint(base_url: &str) -> String {
    format!("{}/api/mcp", base_url.trim_end_matches('/'))
}

pub async fn resolve_mobile_bridge_status(state: &AppState) -> Result<MobileBridgeStatus, String> {
    let bind_config = resolve_webhook_bind_config(state).await?;
    let detected_lan_ip = detect_primary_lan_ip();
    let desktop_base_url = build_desktop_base_url(&bind_config.host, bind_config.port);
    let phone_base_url = build_phone_base_url(
        &bind_config.host,
        bind_config.port,
        detected_lan_ip.as_deref(),
    );
    let bind_scope = classify_bind_scope(&bind_config.host).to_string();
    let lan_ready = phone_base_url.is_some();
    let guidance = if lan_ready {
        "Use the phone base URL from the same Wi-Fi network. Bind host or port changes apply on next app launch.".to_string()
    } else if let Some(lan_ip) = &detected_lan_ip {
        format!(
            "This bridge is currently localhost-only. Set mobile.bind_host to 0.0.0.0 and restart, then connect the iPhone to http://{}:{}.",
            lan_ip, bind_config.port
        )
    } else {
        "This bridge is currently localhost-only. Set mobile.bind_host to 0.0.0.0 and restart to enable same-LAN iPhone access.".to_string()
    };

    Ok(MobileBridgeStatus {
        bind_host: bind_config.host.clone(),
        bind_port: bind_config.port,
        host_source: bind_config.host_source.clone(),
        port_source: bind_config.port_source.clone(),
        bind_scope,
        detected_lan_ip,
        desktop_base_url,
        phone_base_url,
        lan_ready,
        bind_changes_require_restart: true,
        env_overrides_settings: bind_config.host_source == "env"
            || bind_config.port_source == "env",
        guidance,
    })
}

pub async fn resolve_mcp_bridge_status(state: &AppState) -> Result<McpBridgeStatus, String> {
    let bind_config = resolve_webhook_bind_config(state).await?;
    let detected_lan_ip = detect_primary_lan_ip();
    let desktop_base_url = build_desktop_base_url(&bind_config.host, bind_config.port);
    let lan_base_url = build_phone_base_url(
        &bind_config.host,
        bind_config.port,
        detected_lan_ip.as_deref(),
    );
    let endpoint_url = build_mcp_endpoint(&desktop_base_url);
    let lan_endpoint_url = lan_base_url
        .as_ref()
        .map(|base_url| build_mcp_endpoint(base_url));
    let bind_scope = classify_bind_scope(&bind_config.host).to_string();
    let token_configured = configured_token(resolve_mcp_api_token(state).await?).is_some();
    let requests_allowed = token_configured || bind_scope == "localhost-only";
    let auth_mode = if token_configured {
        "bearer_token"
    } else if bind_scope == "localhost-only" {
        "localhost_only_no_token"
    } else {
        "blocked_until_token_configured"
    }
    .to_string();
    let origin_policy = if bind_scope == "localhost-only" {
        "Rejects browser origins that are not loopback or localhost.".to_string()
    } else {
        "Rejects browser origins that do not match the configured desktop or LAN bridge URL."
            .to_string()
    };
    let guidance = if token_configured {
        if let Some(lan_endpoint_url) = &lan_endpoint_url {
            format!(
                "Connect local agents to {endpoint_url}, or connect same-LAN agents to {lan_endpoint_url}, and send Authorization: Bearer <mcp.api_token>."
            )
        } else {
            format!(
                "Connect local agents to {endpoint_url} and send Authorization: Bearer <mcp.api_token>."
            )
        }
    } else if bind_scope == "localhost-only" {
        format!(
            "Local agents can connect to {endpoint_url} without a token while the bridge stays on localhost. Before exposing the bridge beyond localhost, set mcp.api_token."
        )
    } else {
        "This MCP bridge is reachable beyond localhost but mcp.api_token is not configured, so HTTP MCP requests are rejected. Set mcp.api_token or switch mobile.bind_host back to 127.0.0.1 and restart.".to_string()
    };

    Ok(McpBridgeStatus {
        bind_host: bind_config.host.clone(),
        bind_port: bind_config.port,
        host_source: bind_config.host_source.clone(),
        port_source: bind_config.port_source.clone(),
        bind_scope,
        detected_lan_ip,
        desktop_base_url,
        lan_base_url,
        endpoint_url,
        lan_endpoint_url,
        token_configured,
        requests_allowed,
        auth_mode,
        origin_policy,
        bind_changes_require_restart: true,
        env_overrides_settings: bind_config.host_source == "env"
            || bind_config.port_source == "env",
        guidance,
    })
}

fn forbidden(message: impl Into<String>) -> Response {
    (StatusCode::FORBIDDEN, message.into()).into_response()
}

fn json_response(status: StatusCode, payload: Value) -> Response {
    (status, Json(payload)).into_response()
}

fn is_allowed_origin_host(candidate: &str, bind_host: &str, detected_lan_ip: Option<&str>) -> bool {
    let normalized = candidate.trim().trim_matches(['[', ']']);
    if matches!(normalized, "localhost" | "127.0.0.1" | "::1") {
        return true;
    }

    if normalized.eq_ignore_ascii_case(bind_host.trim().trim_matches(['[', ']'])) {
        return true;
    }

    match bind_host.trim() {
        "0.0.0.0" | "::" => detected_lan_ip.is_some_and(|lan_ip| normalized == lan_ip),
        _ => false,
    }
}

async fn ensure_mcp_origin_allowed(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(origin) = headers.get("origin").and_then(|value| value.to_str().ok()) else {
        return Ok(());
    };
    if origin.trim().is_empty() || origin.eq_ignore_ascii_case("null") {
        return Err(forbidden("MCP origin validation failed."));
    }

    let origin_url = Url::parse(origin).map_err(|_| forbidden("MCP origin validation failed."))?;
    let bind_config = resolve_webhook_bind_config(state)
        .await
        .map_err(unavailable)?;
    let detected_lan_ip = detect_primary_lan_ip();
    let origin_host = origin_url
        .host_str()
        .ok_or_else(|| forbidden("MCP origin validation failed."))?;
    let origin_port = origin_url
        .port_or_known_default()
        .ok_or_else(|| forbidden("MCP origin validation failed."))?;

    if origin_port != bind_config.port
        || !is_allowed_origin_host(origin_host, &bind_config.host, detected_lan_ip.as_deref())
    {
        return Err(forbidden("MCP origin validation failed."));
    }

    Ok(())
}

async fn ensure_mcp_api_authorized(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    ensure_mcp_origin_allowed(state, headers).await?;

    let bind_config = resolve_webhook_bind_config(state)
        .await
        .map_err(unavailable)?;
    let configured_token =
        configured_token(resolve_mcp_api_token(state).await.map_err(unavailable)?);
    if let Some(configured_token) = configured_token {
        return match provided_bearer_token(headers) {
            Some(candidate) if candidate == configured_token => Ok(()),
            _ => Err(unauthorized("MCP API authorization failed.")),
        };
    }

    if classify_bind_scope(&bind_config.host) == "localhost-only" {
        Ok(())
    } else {
        Err(unavailable(
            "MCP API token is not configured. Set mcp.api_token before exposing HTTP MCP beyond localhost.",
        ))
    }
}

async fn mcp_http_get(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mcp_origin_allowed(&state.app_state, &headers).await {
        return response;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This MCP endpoint accepts POST JSON-RPC requests. SSE is not enabled.",
    )
        .into_response()
}

async fn mcp_http_delete(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mcp_origin_allowed(&state.app_state, &headers).await {
        return response;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This MCP endpoint does not manage explicit HTTP sessions.",
    )
        .into_response()
}

async fn mcp_http_post(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = ensure_mcp_api_authorized(&state.app_state, &headers).await {
        return response;
    }

    let payload = match serde_json::from_slice::<Value>(&body) {
        Ok(payload) => payload,
        Err(error) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error",
                        "data": {
                            "details": error.to_string(),
                        }
                    }
                }),
            )
        }
    };

    match mcp::handle_json_rpc_value(&state.app_state, payload).await {
        Some(response) => json_response(StatusCode::OK, response),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn twilio_whatsapp(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let config = match resolve_twilio_config(&state.app_state).await {
        Ok(config) => config,
        Err(error) => {
            return Html(messaging_twiml(&format!("Planner error: {}", error))).into_response()
        }
    };
    if let Err(error) = validate_twilio_signature(
        config.auth_token.as_deref(),
        config.webhook_base_url.as_deref(),
        "/webhooks/twilio/whatsapp",
        &params,
        &headers,
    ) {
        return (axum::http::StatusCode::UNAUTHORIZED, error).into_response();
    }
    let form: TwilioMessagingForm = serde_json::from_value(
        serde_json::to_value(&params).unwrap_or_default(),
    )
    .unwrap_or(TwilioMessagingForm {
        body: None,
        from: None,
        wa_id: None,
    });
    let content = form.body.unwrap_or_default();
    let remote_user_id = form
        .wa_id
        .or(form.from.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let remote_conversation_id = form.from.unwrap_or_else(|| remote_user_id.clone());
    match handle_inbound_message(
        &state.app_state,
        ChannelInboundMessage {
            channel: "twilio_whatsapp".to_string(),
            remote_user_id,
            remote_conversation_id,
            content,
        },
    )
    .await
    {
        Ok(response) => Html(messaging_twiml(&planner_reply_text(&response))).into_response(),
        Err(error) => {
            error!(error = %error, "twilio whatsapp webhook failed");
            Html(messaging_twiml(&format!("Planner error: {}", error))).into_response()
        }
    }
}

async fn twilio_voice_entry() -> impl IntoResponse {
    Html(voice_gather_twiml(
        "Welcome to Aruvi planner. Tell me what you want to plan after the tone.",
    ))
}

async fn twilio_voice_gather(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let config = match resolve_twilio_config(&state.app_state).await {
        Ok(config) => config,
        Err(error) => {
            return Html(voice_gather_twiml(&format!("Planner error: {}", error))).into_response()
        }
    };
    if let Err(error) = validate_twilio_signature(
        config.auth_token.as_deref(),
        config.webhook_base_url.as_deref(),
        "/webhooks/twilio/voice/gather",
        &params,
        &headers,
    ) {
        return (axum::http::StatusCode::UNAUTHORIZED, error).into_response();
    }
    let form: TwilioVoiceForm = serde_json::from_value(
        serde_json::to_value(&params).unwrap_or_default(),
    )
    .unwrap_or(TwilioVoiceForm {
        call_sid: None,
        from: None,
        speech_result: None,
    });
    let content = form
        .speech_result
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "I need more detail".to_string());
    let remote_user_id = form.from.unwrap_or_else(|| "unknown".to_string());
    let remote_conversation_id = form.call_sid.unwrap_or_else(|| remote_user_id.clone());
    match handle_inbound_message(
        &state.app_state,
        ChannelInboundMessage {
            channel: "twilio_voice".to_string(),
            remote_user_id,
            remote_conversation_id,
            content,
        },
    )
    .await
    {
        Ok(response) => Html(voice_gather_twiml(&planner_reply_text(&response))).into_response(),
        Err(error) => {
            error!(error = %error, "twilio voice webhook failed");
            Html(voice_gather_twiml(&format!("Planner error: {}", error))).into_response()
        }
    }
}

async fn twilio_voice_entry_with_prompt(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let prompt = params.get("prompt").cloned().unwrap_or_else(|| {
        "Welcome to Aruvi planner. Tell me what you want to plan after the tone.".to_string()
    });
    Html(voice_gather_twiml(&prompt))
}

pub async fn start_webhook_server(app_state: AppState) {
    let bind_config = match resolve_webhook_bind_config(&app_state).await {
        Ok(bind_config) => bind_config,
        Err(error) => {
            error!(error = %error, "failed to resolve webhook bind config");
            return;
        }
    };
    let host = bind_config.host;
    let port = bind_config.port;
    let bind_target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let address: SocketAddr = match bind_target.parse() {
        Ok(address) => address,
        Err(error) => {
            error!(error = %error, "invalid webhook bind address");
            return;
        }
    };

    let router = Router::new()
        .route("/health", get(healthcheck))
        .route("/remote", get(remote_app))
        .route("/remote/*path", get(remote_app))
        .route(
            "/api/mcp",
            get(mcp_http_get)
                .post(mcp_http_post)
                .delete(mcp_http_delete),
        )
        .route("/api/mobile/health", get(mobile_healthcheck))
        .route("/api/mobile/model-calls", get(mobile_list_model_calls))
        .route(
            "/api/mobile/model-calls/:call_id",
            get(mobile_get_model_call),
        )
        .route("/api/mobile/products", get(mobile_list_products))
        .route(
            "/api/mobile/products/:product_id/tree",
            get(mobile_get_product_tree),
        )
        .route(
            "/api/mobile/planner/sessions",
            post(mobile_create_planner_session),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id",
            post(mobile_update_planner_session),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/turn",
            post(mobile_submit_planner_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/voice-turn",
            post(mobile_submit_planner_voice_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/confirm",
            post(mobile_confirm_planner_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/clear",
            post(mobile_clear_planner_turn),
        )
        .route("/api/mobile/chat/completions", post(mobile_chat_completion))
        .route(
            "/api/mobile/planner-chat/sessions",
            post(mobile_create_planner_chat_session),
        )
        .route(
            "/api/mobile/planner-chat/sessions/:session_id/turn",
            post(mobile_submit_planner_chat_turn),
        )
        .route(
            "/api/mobile/speech/transcribe",
            post(mobile_transcribe_audio),
        )
        .route("/webhooks/twilio/whatsapp", post(twilio_whatsapp))
        .route(
            "/webhooks/twilio/voice",
            get(twilio_voice_entry_with_prompt).post(twilio_voice_entry),
        )
        .route("/webhooks/twilio/voice/gather", post(twilio_voice_gather))
        .with_state(WebhookState { app_state });

    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            error!(error = %error, "failed to bind webhook server");
            return;
        }
    };

    info!(address = %address, "webhook server listening");
    if let Err(error) = axum::serve(listener, router).await {
        error!(error = %error, "webhook server failed");
    }
}

const REMOTE_APP_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <title>Aruvi Studio Mobile</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101214;
      --surface: #171b20;
      --surface-2: #20262d;
      --surface-3: #12171d;
      --border: #343d46;
      --text: #f4f6f8;
      --muted: #a5b0bb;
      --blue: #2274a5;
      --green: #2f9d7e;
      --amber: #c9872b;
      --red: #8c3434;
    }

    * {
      box-sizing: border-box;
    }

    html,
    body,
    #app {
      min-height: 100%;
      margin: 0;
    }

    body {
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    button,
    input,
    textarea {
      font: inherit;
    }

    button {
      min-height: 42px;
      border: 0;
      border-radius: 8px;
      padding: 0 14px;
      color: #fff;
      background: var(--blue);
      font-weight: 750;
    }

    button.secondary {
      background: #29313a;
      color: #f0f4f8;
      border: 1px solid #46515d;
    }

    button.danger {
      background: var(--red);
    }

    button:disabled {
      opacity: 0.55;
    }

    input,
    textarea {
      width: 100%;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: #0e1216;
      color: var(--text);
      padding: 11px 12px;
    }

    textarea {
      min-height: 104px;
      resize: vertical;
    }

    .shell {
      min-height: 100vh;
      display: grid;
      grid-template-rows: auto auto 1fr;
    }

    .topbar {
      position: sticky;
      top: 0;
      z-index: 3;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 14px max(16px, env(safe-area-inset-left)) 12px max(16px, env(safe-area-inset-right));
      border-bottom: 1px solid var(--border);
      background: rgba(16, 18, 20, 0.96);
      backdrop-filter: blur(16px);
    }

    .brand {
      min-width: 0;
    }

    .brand h1 {
      margin: 0;
      font-size: 18px;
      line-height: 1.2;
    }

    .status,
    .meta,
    .empty,
    .error,
    .tree-meta,
    .tree-summary {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.45;
    }

    .status {
      margin-top: 4px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .ok {
      color: #68d8b5;
    }

    .warn {
      color: #f2bd6b;
    }

    .error {
      color: #ff9e9e;
    }

    .tabbar {
      position: sticky;
      top: 62px;
      z-index: 2;
      display: flex;
      gap: 6px;
      overflow-x: auto;
      padding: 8px max(16px, env(safe-area-inset-left));
      border-bottom: 1px solid var(--border);
      background: rgba(16, 18, 20, 0.94);
      scrollbar-width: none;
    }

    .tabbar::-webkit-scrollbar {
      display: none;
    }

    .tab-button {
      flex: 0 0 auto;
      min-height: 38px;
      background: transparent;
      color: var(--muted);
      border: 1px solid transparent;
      padding: 0 12px;
    }

    .tab-button.active {
      background: #22313c;
      color: #ffffff;
      border-color: #3e5365;
    }

    .main {
      padding: 14px max(16px, env(safe-area-inset-right)) max(18px, env(safe-area-inset-bottom)) max(16px, env(safe-area-inset-left));
      min-height: 0;
    }

    .tab-panel {
      display: none;
    }

    .tab-panel.active {
      display: block;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(0, 1.15fr) minmax(300px, 0.85fr);
      gap: 14px;
      align-items: start;
    }

    .panel {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--surface);
      min-height: 0;
      overflow: hidden;
    }

    .panel-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 12px 14px;
      border-bottom: 1px solid var(--border);
      background: var(--surface-2);
    }

    .panel-title {
      font-size: 13px;
      font-weight: 800;
      text-transform: uppercase;
      color: #d5dde5;
    }

    .panel-body {
      padding: 14px;
    }

    .conversation {
      display: flex;
      flex-direction: column;
      gap: 10px;
      min-height: 42vh;
      max-height: 58vh;
      overflow: auto;
      padding-right: 2px;
    }

    .bubble {
      max-width: 86%;
      padding: 11px 12px;
      border-radius: 8px;
      white-space: pre-wrap;
      line-height: 1.45;
      font-size: 14px;
    }

    .bubble.user {
      align-self: flex-end;
      background: #1d6f99;
    }

    .bubble.assistant {
      align-self: flex-start;
      background: #2a323a;
    }

    .composer,
    .settings,
    .stack,
    .tree,
    .product-list,
    .tool-list {
      display: grid;
      gap: 10px;
    }

    .composer {
      margin-top: 12px;
    }

    .side-stack {
      display: grid;
      gap: 14px;
      align-content: start;
    }

    .button-row {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }

    .field-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .label {
      display: block;
      margin-bottom: 5px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 700;
    }

    .tree-node,
    .product-card,
    .tool-card,
    .metric {
      border: 1px solid #394653;
      border-radius: 8px;
      padding: 10px;
      background: var(--surface-3);
      text-align: left;
      color: var(--text);
    }

    button.tree-node,
    button.product-card {
      min-height: auto;
      width: 100%;
    }

    .tree-node.selected,
    .product-card.selected {
      border-color: #55a7d4;
      background: #183548;
    }

    .tree-title,
    .product-title,
    .tool-title {
      font-weight: 800;
      line-height: 1.35;
    }

    .tree-meta,
    .tree-summary,
    .product-summary,
    .tool-meta,
    .empty {
      margin-top: 5px;
    }

    .product-summary,
    .tool-meta {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.45;
    }

    .metrics {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
    }

    .metric-value {
      font-size: 20px;
      font-weight: 850;
    }

    .metric-label {
      margin-top: 4px;
      color: var(--muted);
      font-size: 12px;
    }

    @media (max-width: 820px) {
      .topbar {
        align-items: flex-start;
      }

      .tabbar {
        top: 63px;
      }

      .split {
        grid-template-columns: 1fr;
      }

      .field-grid,
      .metrics {
        grid-template-columns: 1fr;
      }

      .conversation {
        min-height: 36vh;
        max-height: 50vh;
      }

      .bubble {
        max-width: 94%;
      }
    }
  </style>
</head>
<body>
  <div id="app">
    <div class="shell">
      <header class="topbar">
        <div class="brand">
          <h1>Aruvi Studio</h1>
          <div class="status" id="status">Disconnected</div>
        </div>
        <button class="secondary" id="healthButton" type="button">Check</button>
      </header>

      <nav class="tabbar" aria-label="Aruvi mobile sections">
        <button class="tab-button active" data-tab="planner" type="button">Planner</button>
        <button class="tab-button" data-tab="products" type="button">Products</button>
        <button class="tab-button" data-tab="chat" type="button">Chat</button>
        <button class="tab-button" data-tab="voice" type="button">Voice</button>
        <button class="tab-button" data-tab="activity" type="button">Activity</button>
      </nav>

      <main class="main">
        <section class="tab-panel active" id="tab-planner">
          <div class="split">
            <section class="panel">
              <div class="panel-header">
                <div class="panel-title">Planner</div>
                <div class="status" id="sessionStatus">No session</div>
              </div>
              <div class="panel-body">
                <div class="conversation" id="conversation"></div>
                <div class="composer">
                  <textarea id="composer" placeholder="Tell the planner what to change"></textarea>
                  <div class="button-row">
                    <button id="sendButton" type="button">Send</button>
                    <button class="secondary" id="confirmButton" type="button">Confirm</button>
                    <button class="danger" id="clearButton" type="button">Clear</button>
                  </div>
                </div>
              </div>
            </section>

            <aside class="side-stack">
              <section class="panel">
                <div class="panel-header">
                  <div class="panel-title">Connection</div>
                </div>
                <div class="panel-body settings">
                  <label>
                    <span class="label">Token</span>
                    <input id="tokenInput" type="password" autocomplete="current-password" placeholder="mobile.api_token" />
                  </label>
                  <div class="field-grid">
                    <label>
                      <span class="label">Provider</span>
                      <input id="providerInput" placeholder="optional" />
                    </label>
                    <label>
                      <span class="label">Model</span>
                      <input id="modelInput" placeholder="optional" />
                    </label>
                  </div>
                  <div class="button-row">
                    <button id="saveButton" type="button">Save</button>
                    <button class="secondary" id="copyLinkButton" type="button">Copy Setup Link</button>
                    <button class="secondary" id="newSessionButton" type="button">New Session</button>
                  </div>
                  <div class="empty" id="connectionMeta"></div>
                </div>
              </section>

              <section class="panel">
                <div class="panel-header">
                  <div class="panel-title">Draft Tree</div>
                </div>
                <div class="panel-body">
                  <div class="tree" id="draftTree"></div>
                </div>
              </section>
            </aside>
          </div>
        </section>

        <section class="tab-panel" id="tab-products">
          <div class="split">
            <section class="panel">
              <div class="panel-header">
                <div class="panel-title">Products</div>
                <button class="secondary" id="refreshProductsButton" type="button">Refresh</button>
              </div>
              <div class="panel-body">
                <div class="product-list" id="productList"></div>
              </div>
            </section>

            <section class="panel">
              <div class="panel-header">
                <div class="panel-title">Product Overview</div>
                <div class="status" id="productStatus">No product</div>
              </div>
              <div class="panel-body stack">
                <div class="metrics" id="productMetrics"></div>
                <div class="tree" id="productTree"></div>
              </div>
            </section>
          </div>
        </section>

        <section class="tab-panel" id="tab-chat">
          <section class="panel">
            <div class="panel-header">
              <div class="panel-title">Chat</div>
              <div class="status" id="chatSessionStatus">No session</div>
            </div>
            <div class="panel-body">
              <div class="conversation" id="chatConversation"></div>
              <div class="composer">
                <textarea id="chatComposer" placeholder="Ask Aruvi"></textarea>
                <div class="button-row">
                  <button id="chatSendButton" type="button">Send</button>
                </div>
              </div>
            </div>
          </section>
        </section>

        <section class="tab-panel" id="tab-voice">
          <section class="panel">
            <div class="panel-header">
              <div class="panel-title">Voice</div>
              <div class="status" id="voiceSessionStatus">No session</div>
            </div>
            <div class="panel-body">
              <div class="conversation" id="voiceConversation"></div>
              <div class="composer">
                <textarea id="voiceComposer" placeholder="Voice transcript"></textarea>
                <div class="button-row">
                  <button id="voiceRecordButton" type="button">Record</button>
                  <button id="voiceSendButton" type="button">Submit</button>
                  <button class="secondary" id="voiceSpeakToggle" type="button">Speak On</button>
                </div>
                <div class="empty" id="voiceMeta">Record sends WAV audio to the desktop speech settings, then speaks the reply on this device when supported.</div>
              </div>
            </div>
          </section>
        </section>

        <section class="tab-panel" id="tab-activity">
          <div class="split">
            <section class="panel">
              <div class="panel-header">
                <div class="panel-title">Bridge</div>
              </div>
              <div class="panel-body stack">
                <div class="metric">
                  <div class="metric-value" id="bridgeState">Offline</div>
                  <div class="metric-label" id="bridgeUrl">Not checked</div>
                </div>
                <div class="tool-list" id="healthDetails"></div>
              </div>
            </section>

            <section class="panel">
              <div class="panel-header">
                <div class="panel-title">File MCP Tools</div>
              </div>
              <div class="panel-body">
                <div class="tool-list" id="mcpToolList"></div>
              </div>
            </section>
          </div>
        </section>
      </main>
    </div>
  </div>

  <script>
    const storageKeys = {
      activeTab: "aruvi.remote.active_tab",
      token: "aruvi.remote.token",
      provider: "aruvi.remote.provider",
      model: "aruvi.remote.model",
      locale: "aruvi.remote.locale",
      session: "aruvi.remote.session",
      selectedNode: "aruvi.remote.selected_node",
      selectedProduct: "aruvi.remote.selected_product"
    };

    const mcpFileTools = [
      ["repositories.list", "List registered repositories"],
      ["repositories.trees.list", "List repository files"],
      ["repositories.files.read", "Read a repository file"],
      ["repositories.files.write", "Write a repository file"],
      ["repositories.files.get_sha256", "Get current file hash"],
      ["repositories.files.apply_patch", "Apply a unified patch"],
      ["repositories.workspaces.create_for_scope", "Create local workspace"]
    ];

    const state = {
      activeTab: localStorage.getItem(storageKeys.activeTab) || "planner",
      sessionId: localStorage.getItem(storageKeys.session) || "",
      selectedNodeId: localStorage.getItem(storageKeys.selectedNode) || "",
      selectedProductId: localStorage.getItem(storageKeys.selectedProduct) || "",
      draftTreeNodes: [],
      products: [],
      productTree: null,
      chatMessages: [],
      voiceMessages: [],
      voiceRecording: null,
      voiceRepliesEnabled: true,
      busy: false
    };

    const el = {
      status: document.getElementById("status"),
      healthButton: document.getElementById("healthButton"),
      tabs: Array.from(document.querySelectorAll(".tab-button")),
      panels: Array.from(document.querySelectorAll(".tab-panel")),
      sessionStatus: document.getElementById("sessionStatus"),
      chatSessionStatus: document.getElementById("chatSessionStatus"),
      voiceSessionStatus: document.getElementById("voiceSessionStatus"),
      conversation: document.getElementById("conversation"),
      composer: document.getElementById("composer"),
      sendButton: document.getElementById("sendButton"),
      confirmButton: document.getElementById("confirmButton"),
      clearButton: document.getElementById("clearButton"),
      tokenInput: document.getElementById("tokenInput"),
      providerInput: document.getElementById("providerInput"),
      modelInput: document.getElementById("modelInput"),
      saveButton: document.getElementById("saveButton"),
      copyLinkButton: document.getElementById("copyLinkButton"),
      newSessionButton: document.getElementById("newSessionButton"),
      connectionMeta: document.getElementById("connectionMeta"),
      draftTree: document.getElementById("draftTree"),
      refreshProductsButton: document.getElementById("refreshProductsButton"),
      productList: document.getElementById("productList"),
      productStatus: document.getElementById("productStatus"),
      productMetrics: document.getElementById("productMetrics"),
      productTree: document.getElementById("productTree"),
      chatConversation: document.getElementById("chatConversation"),
      chatComposer: document.getElementById("chatComposer"),
      chatSendButton: document.getElementById("chatSendButton"),
      voiceConversation: document.getElementById("voiceConversation"),
      voiceComposer: document.getElementById("voiceComposer"),
      voiceRecordButton: document.getElementById("voiceRecordButton"),
      voiceSendButton: document.getElementById("voiceSendButton"),
      voiceSpeakToggle: document.getElementById("voiceSpeakToggle"),
      voiceMeta: document.getElementById("voiceMeta"),
      bridgeState: document.getElementById("bridgeState"),
      bridgeUrl: document.getElementById("bridgeUrl"),
      healthDetails: document.getElementById("healthDetails"),
      mcpToolList: document.getElementById("mcpToolList")
    };

    function normalizeBaseUrl() {
      return window.location.origin;
    }

    function importSettingsFromQuery() {
      const params = new URLSearchParams(window.location.search);
      const token = params.get("token");
      const provider = params.get("providerId") || params.get("provider_id");
      const model = params.get("modelName") || params.get("model_name");
      const locale = params.get("locale");
      let imported = false;
      if (token) {
        localStorage.setItem(storageKeys.token, token);
        imported = true;
      }
      if (provider !== null) {
        localStorage.setItem(storageKeys.provider, provider);
        imported = true;
      }
      if (model !== null) {
        localStorage.setItem(storageKeys.model, model);
        imported = true;
      }
      if (locale !== null) {
        localStorage.setItem(storageKeys.locale, locale);
        imported = true;
      }
      if (imported && window.history.replaceState) {
        window.history.replaceState({}, document.title, window.location.pathname);
      }
    }

    function loadSettings() {
      importSettingsFromQuery();
      el.tokenInput.value = localStorage.getItem(storageKeys.token) || "";
      el.providerInput.value = localStorage.getItem(storageKeys.provider) || "";
      el.modelInput.value = localStorage.getItem(storageKeys.model) || "";
      el.connectionMeta.textContent = normalizeBaseUrl();
      updateSessionStatus();
    }

    function saveSettings() {
      localStorage.setItem(storageKeys.token, el.tokenInput.value.trim());
      localStorage.setItem(storageKeys.provider, el.providerInput.value.trim());
      localStorage.setItem(storageKeys.model, el.modelInput.value.trim());
      setStatus("Saved", "ok");
    }

    function setStatus(message, kind) {
      el.status.textContent = message;
      el.status.className = "status" + (kind ? " " + kind : "");
    }

    function setBusy(nextBusy) {
      state.busy = nextBusy;
      [
        el.sendButton,
        el.chatSendButton,
        el.voiceRecordButton,
        el.voiceSendButton,
        el.voiceSpeakToggle,
        el.refreshProductsButton,
        el.newSessionButton,
        el.healthButton,
        el.saveButton,
        el.copyLinkButton
      ].forEach((button) => {
        button.disabled = nextBusy;
      });
      if (state.voiceRecording) {
        el.voiceRecordButton.disabled = false;
      }
      el.confirmButton.disabled = nextBusy || !state.sessionId;
      el.clearButton.disabled = nextBusy || !state.sessionId;
    }

    function updateSessionStatus() {
      const label = state.sessionId ? state.sessionId.slice(0, 8) : "No session";
      el.sessionStatus.textContent = label;
      el.chatSessionStatus.textContent = label;
      el.voiceSessionStatus.textContent = label;
      setBusy(state.busy);
    }

    function switchTab(tabId) {
      state.activeTab = tabId;
      localStorage.setItem(storageKeys.activeTab, tabId);
      el.tabs.forEach((button) => button.classList.toggle("active", button.dataset.tab === tabId));
      el.panels.forEach((panel) => panel.classList.toggle("active", panel.id === "tab-" + tabId));
      if (tabId === "products" && !state.products.length) {
        void loadProducts();
      }
    }

    async function request(path, options = {}) {
      const token = el.tokenInput.value.trim() || localStorage.getItem(storageKeys.token) || "";
      if (!token) {
        throw new Error("mobile.api_token is required");
      }
      const response = await fetch(path, {
        method: options.method || "GET",
        headers: {
          "Content-Type": "application/json",
          "Authorization": "Bearer " + token
        },
        body: options.body === undefined ? undefined : JSON.stringify(options.body)
      });
      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || "Request failed with status " + response.status);
      }
      return response.json();
    }

    function appendMessage(target, role, content) {
      const bubble = document.createElement("div");
      bubble.className = "bubble " + role;
      bubble.textContent = content;
      target.appendChild(bubble);
      target.scrollTop = target.scrollHeight;
    }

    function speakOnDevice(text) {
      if (!state.voiceRepliesEnabled || !text || !("speechSynthesis" in window)) {
        return;
      }
      try {
        window.speechSynthesis.cancel();
        const utterance = new SpeechSynthesisUtterance(text);
        utterance.lang = localStorage.getItem(storageKeys.locale) || "en-US";
        window.speechSynthesis.speak(utterance);
      } catch {}
    }

    function setVoiceMeta(message) {
      el.voiceMeta.textContent = message;
    }

    function flattenFloat32(chunks) {
      const length = chunks.reduce((total, chunk) => total + chunk.length, 0);
      const output = new Float32Array(length);
      let offset = 0;
      chunks.forEach((chunk) => {
        output.set(chunk, offset);
        offset += chunk.length;
      });
      return output;
    }

    function encodeWav(samples, sampleRate) {
      const buffer = new ArrayBuffer(44 + samples.length * 2);
      const view = new DataView(buffer);
      const writeString = (offset, value) => {
        for (let i = 0; i < value.length; i += 1) {
          view.setUint8(offset + i, value.charCodeAt(i));
        }
      };
      writeString(0, "RIFF");
      view.setUint32(4, 36 + samples.length * 2, true);
      writeString(8, "WAVE");
      writeString(12, "fmt ");
      view.setUint32(16, 16, true);
      view.setUint16(20, 1, true);
      view.setUint16(22, 1, true);
      view.setUint32(24, sampleRate, true);
      view.setUint32(28, sampleRate * 2, true);
      view.setUint16(32, 2, true);
      view.setUint16(34, 16, true);
      writeString(36, "data");
      view.setUint32(40, samples.length * 2, true);
      let offset = 44;
      for (let i = 0; i < samples.length; i += 1) {
        const sample = Math.max(-1, Math.min(1, samples[i]));
        view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7fff, true);
        offset += 2;
      }
      return new Blob([view], { type: "audio/wav" });
    }

    function blobToBase64(blob) {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          const value = String(reader.result || "");
          resolve(value.includes(",") ? value.split(",")[1] : value);
        };
        reader.onerror = () => reject(reader.error || new Error("Failed to read audio"));
        reader.readAsDataURL(blob);
      });
    }

    async function transcribeVoiceBlob(blob) {
      const audio_bytes_base64 = await blobToBase64(blob);
      const response = await request("/api/mobile/speech/transcribe", {
        method: "POST",
        body: {
          audio_bytes_base64,
          mime_type: blob.type || "audio/wav",
          locale: localStorage.getItem(storageKeys.locale) || undefined
        }
      });
      return (response.transcript || "").trim();
    }

    async function startVoiceRecording() {
      if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
        throw new Error("Microphone capture is not available in this WebView.");
      }
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const AudioContextCtor = window.AudioContext || window.webkitAudioContext;
      const audioContext = new AudioContextCtor();
      const source = audioContext.createMediaStreamSource(stream);
      const processor = audioContext.createScriptProcessor(4096, 1, 1);
      const chunks = [];
      processor.onaudioprocess = (event) => {
        chunks.push(new Float32Array(event.inputBuffer.getChannelData(0)));
        const output = event.outputBuffer.getChannelData(0);
        output.fill(0);
      };
      source.connect(processor);
      processor.connect(audioContext.destination);
      state.voiceRecording = { stream, audioContext, source, processor, chunks, sampleRate: audioContext.sampleRate };
      el.voiceRecordButton.textContent = "Stop";
      setVoiceMeta("Listening...");
      setBusy(false);
    }

    async function stopVoiceRecording() {
      const recording = state.voiceRecording;
      if (!recording) {
        return;
      }
      state.voiceRecording = null;
      el.voiceRecordButton.textContent = "Record";
      recording.processor.disconnect();
      recording.source.disconnect();
      recording.stream.getTracks().forEach((track) => track.stop());
      await recording.audioContext.close().catch(() => undefined);
      const wav = encodeWav(flattenFloat32(recording.chunks), recording.sampleRate);
      setVoiceMeta("Transcribing...");
      const transcript = await transcribeVoiceBlob(wav);
      if (!transcript) {
        setVoiceMeta("No speech detected.");
        return;
      }
      el.voiceComposer.value = transcript;
      setVoiceMeta("Transcript ready.");
      await submitPrompt("voice");
    }

    async function toggleVoiceRecording() {
      try {
        if (state.voiceRecording) {
          await stopVoiceRecording();
        } else {
          await startVoiceRecording();
        }
      } catch (error) {
        state.voiceRecording = null;
        el.voiceRecordButton.textContent = "Record";
        setVoiceMeta(error.message || String(error));
        setStatus(error.message || String(error), "error");
      }
    }

    function renderDraftTree() {
      el.draftTree.innerHTML = "";
      if (!state.draftTreeNodes.length) {
        const empty = document.createElement("div");
        empty.className = "empty";
        empty.textContent = "No draft is staged.";
        el.draftTree.appendChild(empty);
        return;
      }

      function renderNode(node, depth) {
        const card = document.createElement("button");
        card.type = "button";
        card.className = "tree-node" + (node.id === state.selectedNodeId ? " selected" : "");
        card.style.marginLeft = Math.min(depth * 12, 36) + "px";
        card.onclick = () => {
          state.selectedNodeId = node.id;
          localStorage.setItem(storageKeys.selectedNode, state.selectedNodeId);
          renderDraftTree();
        };

        const title = document.createElement("div");
        title.className = "tree-title";
        title.textContent = node.label || "Untitled";
        card.appendChild(title);

        if (node.meta || node.node_type) {
          const meta = document.createElement("div");
          meta.className = "tree-meta";
          meta.textContent = node.meta || node.node_type;
          card.appendChild(meta);
        }

        if (node.summary) {
          const summary = document.createElement("div");
          summary.className = "tree-summary";
          summary.textContent = node.summary;
          card.appendChild(summary);
        }

        el.draftTree.appendChild(card);
        (node.children || []).forEach((child) => renderNode(child, depth + 1));
      }

      state.draftTreeNodes.forEach((node) => renderNode(node, 0));
    }

    function applyPlannerResponse(response, target) {
      state.sessionId = response.session_id || state.sessionId;
      localStorage.setItem(storageKeys.session, state.sessionId);
      state.selectedNodeId = response.selected_draft_node_id || state.selectedNodeId || "";
      if (state.selectedNodeId) {
        localStorage.setItem(storageKeys.selectedNode, state.selectedNodeId);
      }
      state.draftTreeNodes = response.draft_tree_nodes || [];
      const lines = [
        response.assistant_message,
        ...(response.execution_lines || []),
        ...(response.execution_errors && response.execution_errors.length ? ["Errors: " + response.execution_errors.join(" | ")] : [])
      ].filter(Boolean);
      appendMessage(target || el.conversation, "assistant", lines.join("\n"));
      renderDraftTree();
      updateSessionStatus();
    }

    async function ensureSession() {
      if (state.sessionId) {
        return state.sessionId;
      }
      const session = await request("/api/mobile/planner/sessions", {
        method: "POST",
        body: {
          provider_id: el.providerInput.value.trim() || undefined,
          model_name: el.modelInput.value.trim() || undefined
        }
      });
      state.sessionId = session.session_id;
      localStorage.setItem(storageKeys.session, state.sessionId);
      updateSessionStatus();
      return state.sessionId;
    }

    async function checkHealth() {
      try {
        setBusy(true);
        const health = await request("/api/mobile/health");
        setStatus(health.status === "ok" ? "Connected" : "Health: " + health.status, "ok");
        el.bridgeState.textContent = health.status === "ok" ? "Online" : health.status;
        el.bridgeState.className = "metric-value ok";
        el.bridgeUrl.textContent = normalizeBaseUrl();
        renderHealthDetails(health);
      } catch (error) {
        setStatus(error.message || String(error), "error");
        el.bridgeState.textContent = "Offline";
        el.bridgeState.className = "metric-value error";
        renderHealthDetails({ error: error.message || String(error) });
      } finally {
        setBusy(false);
      }
    }

    function renderHealthDetails(payload) {
      el.healthDetails.innerHTML = "";
      Object.entries(payload).forEach(([key, value]) => {
        const card = document.createElement("div");
        card.className = "tool-card";
        const title = document.createElement("div");
        title.className = "tool-title";
        title.textContent = key;
        const meta = document.createElement("div");
        meta.className = "tool-meta";
        meta.textContent = typeof value === "string" ? value : JSON.stringify(value);
        card.appendChild(title);
        card.appendChild(meta);
        el.healthDetails.appendChild(card);
      });
    }

    async function submitPrompt(mode) {
      const config = {
        planner: {
          box: el.composer,
          target: el.conversation,
          endpoint: "turn",
          status: "Planning..."
        },
        chat: {
          box: el.chatComposer,
          target: el.chatConversation,
          endpoint: "turn",
          status: "Thinking..."
        },
        voice: {
          box: el.voiceComposer,
          target: el.voiceConversation,
          endpoint: "voice-turn",
          status: "Submitting..."
        }
      }[mode];
      const prompt = config.box.value.trim();
      if (!prompt) {
        return;
      }
      saveSettings();
      appendMessage(config.target, "user", prompt);
      config.box.value = "";
      try {
        setBusy(true);
        setStatus(config.status, "warn");
        if (mode === "chat" || mode === "voice") {
          const historyKey = mode === "voice" ? "voiceMessages" : "chatMessages";
          const userMessage = { role: "user", content: prompt };
          const messages = [
            {
              role: "system",
              content: mode === "voice"
                ? "You are Aruvi Studio's mobile voice assistant. Reply conversationally in one or two short sentences for spoken playback."
                : "You are Aruvi Studio's mobile assistant. Keep replies concise and useful for a phone screen."
            },
            ...state[historyKey],
            userMessage
          ];
          const response = await request("/api/mobile/chat/completions", {
            method: "POST",
            body: {
              provider_id: el.providerInput.value.trim() || undefined,
              model_name: el.modelInput.value.trim() || undefined,
              messages,
              temperature: 0.7,
              max_tokens: 4096
            }
          });
          const assistantMessage = { role: "assistant", content: (response.content || "").trim() };
          state[historyKey] = [...state[historyKey], userMessage, assistantMessage].slice(-20);
          appendMessage(config.target, "assistant", assistantMessage.content || "(empty response)");
          if (mode === "voice") {
            speakOnDevice(assistantMessage.content);
            setVoiceMeta("Reply ready.");
          }
          setStatus("Connected", "ok");
          return;
        }
        const sessionId = await ensureSession();
        const response = await request("/api/mobile/planner/sessions/" + encodeURIComponent(sessionId) + "/" + config.endpoint, {
          method: "POST",
          body: {
            user_input: prompt,
            selected_draft_node_id: state.selectedNodeId || null
          }
        });
        applyPlannerResponse(response, config.target);
        setStatus("Connected", "ok");
      } catch (error) {
        appendMessage(config.target, "assistant", "Error: " + (error.message || String(error)));
        setStatus(error.message || String(error), "error");
      } finally {
        setBusy(false);
      }
    }

    async function confirmDraft() {
      if (!state.sessionId) {
        return;
      }
      try {
        setBusy(true);
        setStatus("Confirming...", "warn");
        const response = await request("/api/mobile/planner/sessions/" + encodeURIComponent(state.sessionId) + "/confirm", {
          method: "POST"
        });
        applyPlannerResponse(response, el.conversation);
        if (!response.draft_tree_nodes) {
          state.draftTreeNodes = [];
          state.selectedNodeId = "";
          localStorage.removeItem(storageKeys.selectedNode);
          renderDraftTree();
        }
        setStatus("Connected", "ok");
      } catch (error) {
        appendMessage(el.conversation, "assistant", "Error: " + (error.message || String(error)));
        setStatus(error.message || String(error), "error");
      } finally {
        setBusy(false);
      }
    }

    async function clearDraft() {
      if (!state.sessionId) {
        return;
      }
      try {
        setBusy(true);
        await request("/api/mobile/planner/sessions/" + encodeURIComponent(state.sessionId) + "/clear", {
          method: "POST"
        });
        state.draftTreeNodes = [];
        state.selectedNodeId = "";
        localStorage.removeItem(storageKeys.selectedNode);
        renderDraftTree();
        setStatus("Cleared", "ok");
      } catch (error) {
        setStatus(error.message || String(error), "error");
      } finally {
        setBusy(false);
      }
    }

    function newSession() {
      state.sessionId = "";
      state.selectedNodeId = "";
      state.draftTreeNodes = [];
      localStorage.removeItem(storageKeys.session);
      localStorage.removeItem(storageKeys.selectedNode);
      el.conversation.innerHTML = "";
      el.chatConversation.innerHTML = "";
      el.voiceConversation.innerHTML = "";
      state.chatMessages = [];
      state.voiceMessages = [];
      renderDraftTree();
      updateSessionStatus();
      setStatus("New session", "ok");
    }

    function makeSetupLink() {
      const params = new URLSearchParams();
      params.set("baseUrl", normalizeBaseUrl());
      const token = el.tokenInput.value.trim();
      if (token) params.set("token", token);
      const provider = el.providerInput.value.trim();
      if (provider) params.set("providerId", provider);
      const model = el.modelInput.value.trim();
      if (model) params.set("modelName", model);
      return "aruvi-planner-mobile://connect?" + params.toString();
    }

    async function copySetupLink() {
      saveSettings();
      const link = makeSetupLink();
      try {
        await navigator.clipboard.writeText(link);
        setStatus("Setup link copied", "ok");
      } catch {
        el.connectionMeta.textContent = link;
        setStatus("Setup link ready", "ok");
      }
    }

    async function loadProducts() {
      try {
        setBusy(true);
        setStatus("Loading products...", "warn");
        state.products = await request("/api/mobile/products");
        renderProducts();
        if (state.selectedProductId && state.products.some((product) => product.id === state.selectedProductId)) {
          await loadProductTree(state.selectedProductId);
        } else if (state.products[0]) {
          await loadProductTree(state.products[0].id);
        }
        setStatus("Connected", "ok");
      } catch (error) {
        setStatus(error.message || String(error), "error");
        renderProducts(error.message || String(error));
      } finally {
        setBusy(false);
      }
    }

    function renderProducts(errorMessage) {
      el.productList.innerHTML = "";
      if (errorMessage) {
        const error = document.createElement("div");
        error.className = "error";
        error.textContent = errorMessage;
        el.productList.appendChild(error);
        return;
      }
      if (!state.products.length) {
        const empty = document.createElement("div");
        empty.className = "empty";
        empty.textContent = "No products.";
        el.productList.appendChild(empty);
        return;
      }
      state.products.forEach((product) => {
        const card = document.createElement("button");
        card.type = "button";
        card.className = "product-card" + (product.id === state.selectedProductId ? " selected" : "");
        card.onclick = () => {
          void loadProductTree(product.id);
        };
        const title = document.createElement("div");
        title.className = "product-title";
        title.textContent = product.name;
        const summary = document.createElement("div");
        summary.className = "product-summary";
        summary.textContent = product.description || product.status || product.id;
        card.appendChild(title);
        card.appendChild(summary);
        el.productList.appendChild(card);
      });
    }

    async function loadProductTree(productId) {
      state.selectedProductId = productId;
      localStorage.setItem(storageKeys.selectedProduct, productId);
      renderProducts();
      el.productStatus.textContent = "Loading";
      state.productTree = await request("/api/mobile/products/" + encodeURIComponent(productId) + "/tree");
      renderProductTree();
    }

    function renderMetric(value, label) {
      const metric = document.createElement("div");
      metric.className = "metric";
      const valueEl = document.createElement("div");
      valueEl.className = "metric-value";
      valueEl.textContent = String(value);
      const labelEl = document.createElement("div");
      labelEl.className = "metric-label";
      labelEl.textContent = label;
      metric.appendChild(valueEl);
      metric.appendChild(labelEl);
      return metric;
    }

    function renderProductTree() {
      el.productMetrics.innerHTML = "";
      el.productTree.innerHTML = "";
      const tree = state.productTree;
      if (!tree) {
        el.productStatus.textContent = "No product";
        return;
      }
      const modules = tree.modules || [];
      const roots = tree.roots || [];
      const nodeCount = countNodes(roots);
      el.productStatus.textContent = tree.product ? tree.product.name : "Loaded";
      el.productMetrics.appendChild(renderMetric(modules.length, "Modules"));
      el.productMetrics.appendChild(renderMetric(nodeCount, "Nodes"));
      el.productMetrics.appendChild(renderMetric(tree.product && tree.product.status ? tree.product.status : "active", "Status"));

      if (!roots.length) {
        const empty = document.createElement("div");
        empty.className = "empty";
        empty.textContent = "No tree nodes.";
        el.productTree.appendChild(empty);
        return;
      }
      roots.forEach((node) => renderProductTreeNode(node, 0));
    }

    function countNodes(nodes) {
      return (nodes || []).reduce((total, node) => total + 1 + countNodes(node.children || []), 0);
    }

    function renderProductTreeNode(node, depth) {
      const card = document.createElement("div");
      card.className = "tree-node";
      card.style.marginLeft = Math.min(depth * 12, 36) + "px";
      const title = document.createElement("div");
      title.className = "tree-title";
      title.textContent = node.name || "Untitled";
      const meta = document.createElement("div");
      meta.className = "tree-meta";
      meta.textContent = [node.node_kind, node.node_type].filter(Boolean).join(" / ");
      card.appendChild(title);
      card.appendChild(meta);
      if (node.summary || node.description) {
        const summary = document.createElement("div");
        summary.className = "tree-summary";
        summary.textContent = node.summary || node.description;
        card.appendChild(summary);
      }
      el.productTree.appendChild(card);
      (node.children || []).forEach((child) => renderProductTreeNode(child, depth + 1));
    }

    function renderMcpTools() {
      el.mcpToolList.innerHTML = "";
      mcpFileTools.forEach(([name, description]) => {
        const card = document.createElement("div");
        card.className = "tool-card";
        const title = document.createElement("div");
        title.className = "tool-title";
        title.textContent = name;
        const meta = document.createElement("div");
        meta.className = "tool-meta";
        meta.textContent = description;
        card.appendChild(title);
        card.appendChild(meta);
        el.mcpToolList.appendChild(card);
      });
    }

    el.tabs.forEach((button) => {
      button.addEventListener("click", () => switchTab(button.dataset.tab));
    });
    el.saveButton.addEventListener("click", saveSettings);
    el.copyLinkButton.addEventListener("click", copySetupLink);
    el.healthButton.addEventListener("click", checkHealth);
    el.sendButton.addEventListener("click", () => submitPrompt("planner"));
    el.chatSendButton.addEventListener("click", () => submitPrompt("chat"));
    el.voiceRecordButton.addEventListener("click", () => toggleVoiceRecording());
    el.voiceSendButton.addEventListener("click", () => submitPrompt("voice"));
    el.voiceSpeakToggle.addEventListener("click", () => {
      state.voiceRepliesEnabled = !state.voiceRepliesEnabled;
      el.voiceSpeakToggle.textContent = state.voiceRepliesEnabled ? "Speak On" : "Speak Off";
      setVoiceMeta(state.voiceRepliesEnabled ? "Voice replies enabled." : "Voice replies disabled.");
    });
    el.confirmButton.addEventListener("click", confirmDraft);
    el.clearButton.addEventListener("click", clearDraft);
    el.newSessionButton.addEventListener("click", newSession);
    el.refreshProductsButton.addEventListener("click", loadProducts);
    [el.composer, el.chatComposer, el.voiceComposer].forEach((box) => {
      box.addEventListener("keydown", (event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
          event.preventDefault();
          if (box === el.composer) submitPrompt("planner");
          if (box === el.chatComposer) submitPrompt("chat");
          if (box === el.voiceComposer) submitPrompt("voice");
        }
      });
    });

    loadSettings();
    renderDraftTree();
    renderMcpTools();
    switchTab(state.activeTab);
    setBusy(false);
  </script>
</body>
</html>"##;

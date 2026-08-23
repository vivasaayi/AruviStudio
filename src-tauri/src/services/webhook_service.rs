use crate::mcp;
use crate::services::channel_service::{
    handle_inbound_message, resolve_twilio_config, ChannelInboundMessage,
};
use crate::services::webhook_bridge::{
    ensure_mcp_api_authorized, ensure_mcp_origin_allowed, resolve_webhook_bind_config,
};
pub use crate::services::webhook_bridge::{
    resolve_mcp_bridge_status, resolve_mobile_bridge_status, McpBridgeStatus, MobileBridgeStatus,
};
use crate::services::webhook_mobile_api::{
    mobile_clear_planner_turn, mobile_confirm_planner_turn, mobile_create_planner_session,
    mobile_get_model_call, mobile_get_product_tree, mobile_get_product_tree_summary,
    mobile_healthcheck, mobile_list_model_calls, mobile_list_products, mobile_submit_planner_turn,
    mobile_submit_planner_voice_turn, mobile_transcribe_audio, mobile_update_planner_session,
};
use crate::services::webhook_mobile_chat::mobile_chat_completion;
use crate::services::webhook_mobile_planner_chat::{
    mobile_create_planner_chat_session, mobile_submit_planner_chat_turn,
};
use crate::services::webhook_mobile_work::{
    mobile_approve_work_item, mobile_create_work_item, mobile_get_work_item,
    mobile_get_work_item_delivery, mobile_handle_workflow_action, mobile_list_work_items,
    mobile_start_workflow,
};
use crate::services::webhook_remote_app::REMOTE_APP_HTML;
use crate::services::webhook_twilio::{
    messaging_twiml, planner_reply_text, validate_twilio_signature, voice_gather_twiml,
};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{error, info};

#[derive(Clone)]
pub struct WebhookState {
    pub app_state: AppState,
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

async fn healthcheck() -> impl IntoResponse {
    "ok"
}

async fn remote_app() -> impl IntoResponse {
    Html(REMOTE_APP_HTML)
}

fn json_response(status: StatusCode, payload: Value) -> Response {
    (status, Json(payload)).into_response()
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

pub async fn start_webhook_server(app_state: AppState) -> Result<(), String> {
    let bind_config = resolve_webhook_bind_config(&app_state)
        .await
        .map_err(|error| format!("failed to resolve webhook bind config: {error}"))?;
    let host = bind_config.host;
    let port = bind_config.port;
    let bind_target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let address: SocketAddr = bind_target
        .parse()
        .map_err(|error| format!("invalid webhook bind address: {error}"))?;

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
            "/api/mobile/work-items",
            get(mobile_list_work_items).post(mobile_create_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id",
            get(mobile_get_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/approve",
            post(mobile_approve_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/workflow/start",
            post(mobile_start_workflow),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/delivery",
            get(mobile_get_work_item_delivery),
        )
        .route(
            "/api/mobile/workflows/:workflow_run_id/action",
            post(mobile_handle_workflow_action),
        )
        .route(
            "/api/mobile/products/:product_id/summary",
            get(mobile_get_product_tree_summary),
        )
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

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("failed to bind webhook server: {error}"))?;

    info!(address = %address, "webhook server listening");
    axum::serve(listener, router)
        .await
        .map_err(|error| format!("webhook server failed: {error}"))
}

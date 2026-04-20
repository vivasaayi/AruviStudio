use crate::persistence::{model_repo, settings_repo};
use crate::services::channel_service::{
    handle_inbound_message, resolve_twilio_config, ChannelInboundMessage,
};
use crate::services::planner_service::{
    clear_planner_pending, confirm_planner_plan, create_planner_session, submit_planner_turn,
    submit_planner_voice_turn, update_planner_session,
};
use crate::services::speech_service::{
    transcribe_audio_with_provider, SpeechToTextRequest, SpeechToTextResponse,
};
use crate::state::AppState;
use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use tracing::{error, info};

const MOBILE_API_TOKEN_KEY: &str = "mobile.api_token";
pub const MOBILE_BIND_HOST_KEY: &str = "mobile.bind_host";
pub const MOBILE_BIND_PORT_KEY: &str = "mobile.bind_port";
const SPEECH_PROVIDER_KEY: &str = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY: &str = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY: &str = "speech.locale";

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
    let configured_token = resolve_mobile_api_token(state).await.map_err(unavailable)?;
    let Some(configured_token) = configured_token.filter(|value| !value.trim().is_empty()) else {
        return Err(unavailable(
            "Mobile API token is not configured. Set mobile.api_token first.",
        ));
    };

    let provided_token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .or_else(|| {
            headers
                .get("x-aruvi-token")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
        });

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
        .route("/api/mobile/health", get(mobile_healthcheck))
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

use crate::mcp;
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
use tracing::{error, info};

const MOBILE_API_TOKEN_KEY: &str = "mobile.api_token";
const MCP_API_TOKEN_KEY: &str = "mcp.api_token";
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

const REMOTE_APP_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <title>Aruvi Remote</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101317;
      --panel: #181d23;
      --panel-2: #121820;
      --border: #2f3946;
      --text: #f4f8ff;
      --muted: #9aa7b8;
      --accent: #0e639c;
      --accent-2: #1b4f72;
      --danger: #6c2020;
      --ok: #56c7a9;
      --warn: #f2b66d;
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
      background: var(--accent);
      font-weight: 700;
    }

    button.secondary {
      background: #263241;
      color: #eef4ff;
      border: 1px solid #394657;
    }

    button.danger {
      background: var(--danger);
    }

    button:disabled {
      opacity: 0.55;
    }

    input,
    textarea {
      width: 100%;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: #0e1319;
      color: var(--text);
      padding: 11px 12px;
    }

    textarea {
      min-height: 100px;
      resize: vertical;
    }

    .shell {
      min-height: 100vh;
      display: grid;
      grid-template-rows: auto 1fr;
    }

    .topbar {
      position: sticky;
      top: 0;
      z-index: 2;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 14px max(16px, env(safe-area-inset-left)) 12px max(16px, env(safe-area-inset-left));
      border-bottom: 1px solid var(--border);
      background: rgba(16, 19, 23, 0.96);
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

    .status {
      margin-top: 4px;
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .main {
      display: grid;
      grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.85fr);
      gap: 14px;
      padding: 14px max(16px, env(safe-area-inset-right)) 18px max(16px, env(safe-area-inset-left));
      min-height: 0;
    }

    .panel {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
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
      background: var(--panel-2);
    }

    .panel-title {
      font-size: 13px;
      font-weight: 800;
      text-transform: uppercase;
      color: #cbd5e3;
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
      background: var(--accent);
    }

    .bubble.assistant {
      align-self: flex-start;
      background: #28313d;
    }

    .composer {
      display: grid;
      gap: 10px;
      margin-top: 12px;
    }

    .button-row,
    .field-grid {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }

    .field-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      margin-bottom: 12px;
    }

    .tree {
      display: grid;
      gap: 8px;
    }

    .tree-node {
      border: 1px solid #344253;
      border-radius: 8px;
      padding: 10px;
      background: #111923;
    }

    .tree-node.selected {
      border-color: #42a5db;
      background: #173650;
    }

    .tree-title {
      font-weight: 800;
      line-height: 1.35;
    }

    .tree-meta,
    .tree-summary,
    .empty,
    .error {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.45;
      margin-top: 5px;
    }

    .error {
      color: #ff9b9b;
    }

    .ok {
      color: var(--ok);
    }

    .warn {
      color: var(--warn);
    }

    .settings {
      display: grid;
      gap: 10px;
    }

    .label {
      display: block;
      margin-bottom: 5px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 700;
    }

    .side-stack {
      display: grid;
      gap: 14px;
      align-content: start;
    }

    @media (max-width: 820px) {
      .topbar {
        align-items: flex-start;
      }

      .main {
        grid-template-columns: 1fr;
      }

      .field-grid {
        grid-template-columns: 1fr;
      }

      .conversation {
        min-height: 36vh;
        max-height: 48vh;
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
          <h1>Aruvi Remote</h1>
          <div class="status" id="status">Disconnected</div>
        </div>
        <button class="secondary" id="healthButton" type="button">Check</button>
      </header>

      <main class="main">
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
      </main>
    </div>
  </div>

  <script>
    const storageKeys = {
      token: "aruvi.remote.token",
      provider: "aruvi.remote.provider",
      model: "aruvi.remote.model",
      session: "aruvi.remote.session",
      selectedNode: "aruvi.remote.selected_node"
    };

    const state = {
      sessionId: localStorage.getItem(storageKeys.session) || "",
      selectedNodeId: localStorage.getItem(storageKeys.selectedNode) || "",
      draftTreeNodes: [],
      busy: false
    };

    const el = {
      status: document.getElementById("status"),
      sessionStatus: document.getElementById("sessionStatus"),
      healthButton: document.getElementById("healthButton"),
      conversation: document.getElementById("conversation"),
      composer: document.getElementById("composer"),
      sendButton: document.getElementById("sendButton"),
      confirmButton: document.getElementById("confirmButton"),
      clearButton: document.getElementById("clearButton"),
      tokenInput: document.getElementById("tokenInput"),
      providerInput: document.getElementById("providerInput"),
      modelInput: document.getElementById("modelInput"),
      saveButton: document.getElementById("saveButton"),
      newSessionButton: document.getElementById("newSessionButton"),
      connectionMeta: document.getElementById("connectionMeta"),
      draftTree: document.getElementById("draftTree")
    };

    function normalizeBaseUrl() {
      return window.location.origin;
    }

    function loadSettings() {
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
      el.sendButton.disabled = nextBusy;
      el.confirmButton.disabled = nextBusy || !state.sessionId;
      el.clearButton.disabled = nextBusy || !state.sessionId;
      el.newSessionButton.disabled = nextBusy;
      el.healthButton.disabled = nextBusy;
    }

    function updateSessionStatus() {
      el.sessionStatus.textContent = state.sessionId ? state.sessionId.slice(0, 8) : "No session";
      setBusy(state.busy);
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

    function appendMessage(role, content) {
      const bubble = document.createElement("div");
      bubble.className = "bubble " + role;
      bubble.textContent = content;
      el.conversation.appendChild(bubble);
      el.conversation.scrollTop = el.conversation.scrollHeight;
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

    function applyPlannerResponse(response) {
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
      appendMessage("assistant", lines.join("\n"));
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
      } catch (error) {
        setStatus(error.message || String(error), "error");
      } finally {
        setBusy(false);
      }
    }

    async function sendTurn() {
      const prompt = el.composer.value.trim();
      if (!prompt) {
        return;
      }
      saveSettings();
      appendMessage("user", prompt);
      el.composer.value = "";
      try {
        setBusy(true);
        setStatus("Planning...", "warn");
        const sessionId = await ensureSession();
        const response = await request("/api/mobile/planner/sessions/" + encodeURIComponent(sessionId) + "/turn", {
          method: "POST",
          body: {
            user_input: prompt,
            selected_draft_node_id: state.selectedNodeId || null
          }
        });
        applyPlannerResponse(response);
        setStatus("Connected", "ok");
      } catch (error) {
        appendMessage("assistant", "Error: " + (error.message || String(error)));
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
        applyPlannerResponse(response);
        if (!response.draft_tree_nodes) {
          state.draftTreeNodes = [];
          state.selectedNodeId = "";
          localStorage.removeItem(storageKeys.selectedNode);
          renderDraftTree();
        }
        setStatus("Connected", "ok");
      } catch (error) {
        appendMessage("assistant", "Error: " + (error.message || String(error)));
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
      renderDraftTree();
      updateSessionStatus();
      setStatus("New session", "ok");
    }

    el.saveButton.addEventListener("click", saveSettings);
    el.healthButton.addEventListener("click", checkHealth);
    el.sendButton.addEventListener("click", sendTurn);
    el.confirmButton.addEventListener("click", confirmDraft);
    el.clearButton.addEventListener("click", clearDraft);
    el.newSessionButton.addEventListener("click", newSession);
    el.composer.addEventListener("keydown", (event) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        sendTurn();
      }
    });

    loadSettings();
    renderDraftTree();
    setBusy(false);
  </script>
</body>
</html>"##;

use crate::app_paths;
use crate::persistence::settings_repo;
use crate::state::AppState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use reqwest::Url;
use serde::Serialize;
use std::net::{IpAddr, UdpSocket};

const MOBILE_API_TOKEN_KEY: &str = "mobile.api_token";
const MCP_API_TOKEN_KEY: &str = "mcp.api_token";
pub const MOBILE_BIND_HOST_KEY: &str = "mobile.bind_host";
pub const MOBILE_BIND_PORT_KEY: &str = "mobile.bind_port";

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

pub(crate) async fn resolve_mobile_api_token(state: &AppState) -> Result<Option<String>, String> {
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

pub(crate) fn configured_token(token: Option<String>) -> Option<String> {
    token.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn provided_bearer_token(headers: &HeaderMap) -> Option<String> {
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

pub(crate) fn unavailable(message: impl Into<String>) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, message.into()).into_response()
}

fn forbidden(message: impl Into<String>) -> Response {
    (StatusCode::FORBIDDEN, message.into()).into_response()
}

pub(crate) async fn ensure_mobile_api_authorized(
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
        (
            app_paths::default_webhook_port(state.app_profile.as_deref()),
            if state.app_profile.is_some() {
                "profile-default".to_string()
            } else {
                "default".to_string()
            },
        )
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

pub(crate) async fn ensure_mcp_origin_allowed(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Response> {
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

pub(crate) async fn ensure_mcp_api_authorized(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Response> {
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

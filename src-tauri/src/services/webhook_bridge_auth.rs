use crate::persistence::settings_repo;
use crate::services::webhook_bridge::{
    classify_bind_scope, detect_primary_lan_ip, resolve_webhook_bind_config,
};
use crate::state::AppState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use reqwest::Url;

const MOBILE_API_TOKEN_KEY: &str = "mobile.api_token";
const MCP_API_TOKEN_KEY: &str = "mcp.api_token";

pub(crate) async fn resolve_mobile_api_token(state: &AppState) -> Result<Option<String>, String> {
    Ok(std::env::var("ARUVI_MOBILE_API_TOKEN")
        .ok()
        .or(settings_repo::get_setting(&state.db, MOBILE_API_TOKEN_KEY)
            .await
            .map_err(|error| error.to_string())?))
}

pub(crate) async fn resolve_mcp_api_token(state: &AppState) -> Result<Option<String>, String> {
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

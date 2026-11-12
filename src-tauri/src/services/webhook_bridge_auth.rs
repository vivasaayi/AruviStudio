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

fn origin_matches_request(origin: &str, host: &str, bind_host: &str, lan_ip: Option<&str>) -> bool {
    let Ok(origin_url) = Url::parse(origin) else { return false; };
    if !matches!(origin_url.scheme(), "http" | "https")
        || origin_url.origin().ascii_serialization() != origin
        || host.is_empty()
        || host.contains(['/', '@', '?', '#', '\\'])
    {
        return false;
    }
    let Ok(request_url) = Url::parse(&format!("{}://{}", origin_url.scheme(), host)) else { return false; };
    let Some(origin_host) = origin_url.host_str() else { return false; };
    origin_url.origin() == request_url.origin()
        && is_allowed_origin_host(origin_host, bind_host, lan_ip)
}

pub(crate) async fn ensure_mcp_origin_allowed(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Response> {
    let mut origins = headers.get_all(axum::http::header::ORIGIN).iter();
    let Some(origin_header) = origins.next() else {
        return Ok(());
    };
    if origins.next().is_some() {
        return Err(forbidden("MCP origin validation failed."));
    }
    let origin = origin_header.to_str().map_err(|_| forbidden("MCP origin validation failed."))?;
    if origin.trim().is_empty() || origin.eq_ignore_ascii_case("null") {
        return Err(forbidden("MCP origin validation failed."));
    }

    let bind_config = resolve_webhook_bind_config(state)
        .await
        .map_err(unavailable)?;
    let detected_lan_ip = detect_primary_lan_ip();
    let mut hosts = headers.get_all(axum::http::header::HOST).iter();
    let host = hosts.next().and_then(|value| value.to_str().ok()).unwrap_or("");
    if hosts.next().is_some() {
        return Err(forbidden("MCP origin validation failed."));
    }
    if !origin_matches_request(origin, host, &bind_config.host, detected_lan_ip.as_deref()) {
        return Err(forbidden("MCP origin validation failed."));
    }

    Ok(())
}

#[cfg(test)]
mod origin_tests {
    use super::origin_matches_request;

    #[test]
    fn validates_published_origin_and_rejects_cross_origin_requests() {
        assert!(origin_matches_request("http://127.0.0.1:55002", "127.0.0.1:55002", "0.0.0.0", None));
        assert!(origin_matches_request("http://[::1]:55002", "[::1]:55002", "0.0.0.0", None));
        assert!(!origin_matches_request("http://127.0.0.1:55003", "127.0.0.1:55002", "0.0.0.0", None));
        assert!(!origin_matches_request("http://evil.example:55002", "evil.example:55002", "0.0.0.0", None));
        assert!(!origin_matches_request("http://127.0.0.1:55002/path", "127.0.0.1:55002", "0.0.0.0", None));
        assert!(!origin_matches_request("http://127.0.0.1:55002", "", "0.0.0.0", None));
        assert!(!origin_matches_request("http://127.0.0.1:55002", "evil.example@127.0.0.1:55002", "0.0.0.0", None));
    }
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

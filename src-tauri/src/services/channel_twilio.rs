use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::state::AppState;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

const TWILIO_ACCOUNT_SID_KEY: &str = "twilio.account_sid";
const TWILIO_AUTH_TOKEN_KEY: &str = "twilio.auth_token";
const TWILIO_WHATSAPP_FROM_KEY: &str = "twilio.whatsapp_from";
const TWILIO_VOICE_FROM_KEY: &str = "twilio.voice_from";
const WEBHOOK_BASE_URL_KEY: &str = "twilio.webhook_base_url";

#[derive(Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: Option<String>,
    pub auth_token: Option<String>,
    pub whatsapp_from: Option<String>,
    pub voice_from: Option<String>,
    pub webhook_base_url: Option<String>,
}

impl std::fmt::Debug for TwilioConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwilioConfig")
            .field("account_sid_configured", &has_value(self.account_sid.as_deref()))
            .field("auth_token_configured", &has_value(self.auth_token.as_deref()))
            .field("whatsapp_from_configured", &has_value(self.whatsapp_from.as_deref()))
            .field("voice_from_configured", &has_value(self.voice_from.as_deref()))
            .field("webhook_base_url_configured", &has_value(self.webhook_base_url.as_deref()))
            .finish()
    }
}

async fn send_twilio_form(endpoint: &str, account_sid: &str, auth_token: &str, fields: &[(&str, String)], operation: &str) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(30))
        .build().map_err(|_| AppError::Provider("Could not initialize Twilio client".into()))?;
    let response = client.post(endpoint).basic_auth(account_sid, Some(auth_token)).form(fields).send().await
        .map_err(|error| AppError::Provider(format!("Twilio request failed: {}", error.without_url())))?;
    let status = response.status();
    if status != StatusCode::CREATED && status != StatusCode::OK {
        return Err(AppError::Provider(format!("Twilio {operation} failed: HTTP {}", status.as_u16())));
    }
    Ok(())
}

async fn get_env_or_setting(
    state: &AppState,
    env_key: &str,
    setting_key: &str,
) -> Result<Option<String>, AppError> {
    Ok(std::env::var(env_key)
        .ok()
        .or(settings_repo::get_setting(&state.db, setting_key).await?))
}

pub async fn resolve_twilio_config(state: &AppState) -> Result<TwilioConfig, AppError> {
    Ok(TwilioConfig {
        account_sid: get_env_or_setting(state, "ARUVI_TWILIO_ACCOUNT_SID", TWILIO_ACCOUNT_SID_KEY)
            .await?,
        auth_token: get_env_or_setting(state, "ARUVI_TWILIO_AUTH_TOKEN", TWILIO_AUTH_TOKEN_KEY)
            .await?,
        whatsapp_from: get_env_or_setting(
            state,
            "ARUVI_TWILIO_WHATSAPP_FROM",
            TWILIO_WHATSAPP_FROM_KEY,
        )
        .await?,
        voice_from: get_env_or_setting(state, "ARUVI_TWILIO_VOICE_FROM", TWILIO_VOICE_FROM_KEY)
            .await?,
        webhook_base_url: get_env_or_setting(state, "ARUVI_WEBHOOK_BASE_URL", WEBHOOK_BASE_URL_KEY)
            .await?,
    })
}

fn has_value(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .is_some_and(|candidate| !candidate.is_empty())
}

pub(crate) fn supports_whatsapp(config: &TwilioConfig) -> bool {
    has_value(config.account_sid.as_deref())
        && has_value(config.auth_token.as_deref())
        && has_value(config.whatsapp_from.as_deref())
}

pub(crate) fn supports_voice(config: &TwilioConfig) -> bool {
    has_value(config.account_sid.as_deref())
        && has_value(config.auth_token.as_deref())
        && has_value(config.voice_from.as_deref())
        && has_value(config.webhook_base_url.as_deref())
}

pub async fn send_whatsapp_message(
    state: &AppState,
    to: String,
    content: String,
) -> Result<(), AppError> {
    let config = resolve_twilio_config(state).await?;
    let account_sid = config
        .account_sid
        .ok_or_else(|| AppError::Validation("Twilio account sid is not configured".to_string()))?;
    let auth_token = config
        .auth_token
        .ok_or_else(|| AppError::Validation("Twilio auth token is not configured".to_string()))?;
    let from = config.whatsapp_from.ok_or_else(|| {
        AppError::Validation("Twilio WhatsApp sender is not configured".to_string())
    })?;

    let endpoint = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );
    send_twilio_form(&endpoint, &account_sid, &auth_token, &[("From", from), ("To", to), ("Body", content)], "WhatsApp send").await
}

pub async fn start_voice_call(
    state: &AppState,
    to: String,
    initial_prompt: Option<String>,
) -> Result<(), AppError> {
    let config = resolve_twilio_config(state).await?;
    let account_sid = config
        .account_sid
        .ok_or_else(|| AppError::Validation("Twilio account sid is not configured".to_string()))?;
    let auth_token = config
        .auth_token
        .ok_or_else(|| AppError::Validation("Twilio auth token is not configured".to_string()))?;
    let from = config.voice_from.ok_or_else(|| {
        AppError::Validation("Twilio voice caller id is not configured".to_string())
    })?;
    let webhook_base_url = config.webhook_base_url.ok_or_else(|| {
        AppError::Validation("Twilio webhook base url is not configured".to_string())
    })?;

    let voice_url = if let Some(prompt) = initial_prompt {
        format!(
            "{}/webhooks/twilio/voice?prompt={}",
            webhook_base_url.trim_end_matches('/'),
            urlencoding::encode(&prompt)
        )
    } else {
        format!(
            "{}/webhooks/twilio/voice",
            webhook_base_url.trim_end_matches('/')
        )
    };

    let endpoint = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Calls.json",
        account_sid
    );
    send_twilio_form(&endpoint, &account_sid, &auth_token, &[("From", from), ("To", to), ("Url", voice_url)], "voice call").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, thread};

    #[test]
    fn debug_redacts_all_config_values() {
        let config = TwilioConfig { account_sid: Some("private-sid".into()), auth_token: Some("private-token".into()), whatsapp_from: Some("private-whatsapp".into()), voice_from: Some("private-voice".into()), webhook_base_url: Some("private-url".into()) };
        let debug = format!("{config:?}");
        assert!(debug.contains("auth_token_configured: true"));
        assert!(!debug.contains("private-"));
    }

    #[tokio::test]
    async fn connection_failure_does_not_expose_endpoint_or_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let error = send_twilio_form(&format!("http://{address}/private-account-path"), "private-sid", "private-token", &[("Body", "private-message".into())], "fixture").await.unwrap_err().to_string();
        assert!(error.contains("Twilio request failed"));
        assert!(!error.contains("private-"));
    }

    #[tokio::test]
    async fn outbound_form_redacts_errors_and_refuses_redirects() {
        for status in ["201 Created", "429 Too Many Requests", "307 Temporary Redirect"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let redirect = TcpListener::bind("127.0.0.1:0").unwrap();
            redirect.set_nonblocking(true).unwrap();
            let redirect_address = redirect.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut data = Vec::new(); let mut buffer = [0; 4096];
                let end = loop { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]); if let Some(i) = data.windows(4).position(|v| v == b"\r\n\r\n") { break i + 4; } };
                let headers = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
                assert!(headers.contains("authorization: basic "));
                let size: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
                while data.len() < end + size { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]); }
                assert!(String::from_utf8_lossy(&data[end..]).contains("Body=fixture+message"));
                let body = "private-provider-detail";
                write!(socket, "HTTP/1.1 {status}\r\nLocation: http://{redirect_address}/redirect\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let endpoint = format!("http://{address}/private-account-path");
            let result = send_twilio_form(&endpoint, "private-sid", "private-token", &[("Body", "fixture message".into())], "fixture").await;
            if status.starts_with("201") { assert!(result.is_ok()); } else {
                let error = result.unwrap_err().to_string();
                assert!(error.contains(status.split_whitespace().next().unwrap()));
                assert!(!error.contains("private-"));
            }
            server.join().unwrap();
            assert!(matches!(redirect.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
        }
    }
}

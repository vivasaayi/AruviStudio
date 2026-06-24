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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: Option<String>,
    pub auth_token: Option<String>,
    pub whatsapp_from: Option<String>,
    pub voice_from: Option<String>,
    pub webhook_base_url: Option<String>,
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
    let response = reqwest::Client::new()
        .post(endpoint)
        .basic_auth(account_sid, Some(auth_token))
        .form(&[("From", from), ("To", to), ("Body", content)])
        .send()
        .await
        .map_err(|error| AppError::Provider(format!("Twilio request failed: {}", error)))?;

    if response.status() != StatusCode::CREATED && response.status() != StatusCode::OK {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Provider(format!(
            "Twilio WhatsApp send failed: {}",
            body
        )));
    }

    Ok(())
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
    let response = reqwest::Client::new()
        .post(endpoint)
        .basic_auth(account_sid, Some(auth_token))
        .form(&[("From", from), ("To", to), ("Url", voice_url)])
        .send()
        .await
        .map_err(|error| AppError::Provider(format!("Twilio request failed: {}", error)))?;

    if response.status() != StatusCode::CREATED && response.status() != StatusCode::OK {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Provider(format!(
            "Twilio voice call start failed: {}",
            body
        )));
    }

    Ok(())
}

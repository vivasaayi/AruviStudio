use crate::error::AppError;
use crate::persistence::{planner_repo, settings_repo};
use crate::services::planner_service::{
    confirm_planner_plan, create_planner_session, submit_planner_turn, PlannerTurnResponse,
};
use crate::state::AppState;
use chrono::{Local, NaiveTime};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

const DEFAULT_PROVIDER_SETTING_KEY: &str = "planner.default_provider_id";
const DEFAULT_MODEL_SETTING_KEY: &str = "planner.default_model_name";
const CHANNEL_PREFERENCE_KEY: &str = "planner.channel_preference";
const ESCALATE_TO_CALL_ON_AMBIGUITY_KEY: &str = "planner.escalate_to_call_on_ambiguity";
const CALL_QUIET_HOURS_START_KEY: &str = "planner.call_quiet_hours_start";
const CALL_QUIET_HOURS_END_KEY: &str = "planner.call_quiet_hours_end";
const TWILIO_ACCOUNT_SID_KEY: &str = "twilio.account_sid";
const TWILIO_AUTH_TOKEN_KEY: &str = "twilio.auth_token";
const TWILIO_WHATSAPP_FROM_KEY: &str = "twilio.whatsapp_from";
const TWILIO_VOICE_FROM_KEY: &str = "twilio.voice_from";
const WEBHOOK_BASE_URL_KEY: &str = "twilio.webhook_base_url";

#[derive(Debug, Clone)]
pub struct TwilioConfig {
    pub account_sid: Option<String>,
    pub auth_token: Option<String>,
    pub whatsapp_from: Option<String>,
    pub voice_from: Option<String>,
    pub webhook_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInboundMessage {
    pub channel: String,
    pub remote_user_id: String,
    pub remote_conversation_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOutboundMessage {
    pub channel: String,
    pub remote_user_id: String,
    pub remote_conversation_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Whatsapp,
    Voice,
}

impl ChannelKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Whatsapp => "whatsapp",
            Self::Voice => "voice",
        }
    }

    fn from_input(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "whatsapp" => Some(Self::Whatsapp),
            "voice" | "call" | "phone" => Some(Self::Voice),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelPreference {
    Whatsapp,
    Voice,
    Hybrid,
}

impl ChannelPreference {
    fn from_setting(value: Option<&str>) -> Self {
        match value
            .unwrap_or("hybrid")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "whatsapp" => Self::Whatsapp,
            "voice" => Self::Voice,
            _ => Self::Hybrid,
        }
    }
}

#[derive(Debug, Clone)]
struct ChannelPolicy {
    preference: ChannelPreference,
    escalate_to_call_on_ambiguity: bool,
    call_quiet_hours_start: Option<String>,
    call_quiet_hours_end: Option<String>,
    in_call_quiet_hours: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContactRequest {
    pub to: String,
    pub content: String,
    pub preferred_channel: Option<String>,
    pub allow_after_hours: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContactResult {
    pub channel: String,
    pub status: String,
    pub reason: String,
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

async fn resolve_default_planner_model(
    state: &AppState,
) -> Result<(Option<String>, Option<String>), AppError> {
    let provider_id = std::env::var("ARUVI_PLANNER_PROVIDER_ID")
        .ok()
        .or(settings_repo::get_setting(&state.db, DEFAULT_PROVIDER_SETTING_KEY).await?);
    let model_name = std::env::var("ARUVI_PLANNER_MODEL_NAME")
        .ok()
        .or(settings_repo::get_setting(&state.db, DEFAULT_MODEL_SETTING_KEY).await?);
    Ok((provider_id, model_name))
}

fn parse_hhmm(value: Option<&str>) -> Option<NaiveTime> {
    value
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .and_then(|candidate| NaiveTime::parse_from_str(candidate, "%H:%M").ok())
}

fn is_in_quiet_hours(start: Option<NaiveTime>, end: Option<NaiveTime>) -> bool {
    let (Some(start), Some(end)) = (start, end) else {
        return false;
    };
    if start == end {
        return false;
    }
    let now = Local::now().time();
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

async fn resolve_channel_policy(state: &AppState) -> Result<ChannelPolicy, AppError> {
    let preference = ChannelPreference::from_setting(
        settings_repo::get_setting(&state.db, CHANNEL_PREFERENCE_KEY)
            .await?
            .as_deref(),
    );
    let escalate_to_call_on_ambiguity =
        settings_repo::get_bool_setting(&state.db, ESCALATE_TO_CALL_ON_AMBIGUITY_KEY, true).await?;
    let call_quiet_hours_start =
        settings_repo::get_setting(&state.db, CALL_QUIET_HOURS_START_KEY).await?;
    let call_quiet_hours_end =
        settings_repo::get_setting(&state.db, CALL_QUIET_HOURS_END_KEY).await?;
    let in_call_quiet_hours = is_in_quiet_hours(
        parse_hhmm(call_quiet_hours_start.as_deref()),
        parse_hhmm(call_quiet_hours_end.as_deref()),
    );

    Ok(ChannelPolicy {
        preference,
        escalate_to_call_on_ambiguity,
        call_quiet_hours_start,
        call_quiet_hours_end,
        in_call_quiet_hours,
    })
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

async fn ensure_channel_session(
    state: &AppState,
    channel: &str,
    remote_user_id: &str,
    remote_conversation_id: &str,
) -> Result<String, AppError> {
    if let Some(binding) =
        planner_repo::get_channel_binding(&state.db, channel, remote_conversation_id).await?
    {
        return Ok(binding.planner_session_id);
    }

    let (provider_id, model_name) = resolve_default_planner_model(state).await?;
    let session = create_planner_session(
        state.planner_service.clone(),
        &state.db,
        provider_id,
        model_name,
    )
    .await?;
    planner_repo::create_channel_binding(
        &state.db,
        &uuid::Uuid::new_v4().to_string(),
        channel,
        remote_user_id,
        remote_conversation_id,
        &session.session_id,
    )
    .await?;
    Ok(session.session_id)
}

pub async fn handle_inbound_message(
    state: &AppState,
    message: ChannelInboundMessage,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = ensure_channel_session(
        state,
        &message.channel,
        &message.remote_user_id,
        &message.remote_conversation_id,
    )
    .await?;

    if matches!(
        message.content.trim().to_lowercase().as_str(),
        "yes" | "confirm" | "go ahead"
    ) {
        return confirm_planner_plan(state.planner_service.clone(), state, session_id).await;
    }

    submit_planner_turn(
        state.planner_service.clone(),
        state,
        session_id,
        message.content,
    )
    .await
}

fn has_value(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .is_some_and(|candidate| !candidate.is_empty())
}

fn supports_whatsapp(config: &TwilioConfig) -> bool {
    has_value(config.account_sid.as_deref())
        && has_value(config.auth_token.as_deref())
        && has_value(config.whatsapp_from.as_deref())
}

fn supports_voice(config: &TwilioConfig) -> bool {
    has_value(config.account_sid.as_deref())
        && has_value(config.auth_token.as_deref())
        && has_value(config.voice_from.as_deref())
        && has_value(config.webhook_base_url.as_deref())
}

fn normalize_whatsapp_destination(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.to_ascii_lowercase().starts_with("whatsapp:") {
        trimmed.to_string()
    } else {
        format!("whatsapp:{trimmed}")
    }
}

fn normalize_voice_destination(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.to_ascii_lowercase().starts_with("whatsapp:") {
        trimmed["whatsapp:".len()..].to_string()
    } else {
        trimmed.to_string()
    }
}

fn looks_like_whatsapp_destination(value: &str) -> bool {
    value.trim().to_ascii_lowercase().starts_with("whatsapp:")
}

fn should_escalate_to_call(content: &str) -> bool {
    let normalized = content.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    let question_marks = normalized.matches('?').count();
    let word_count = normalized.split_whitespace().count();
    let ambiguity_markers = [
        "discuss",
        "brainstorm",
        "talk through",
        "walk through",
        "clarify",
        "not sure",
        "unsure",
        "tradeoff",
        "trade-off",
        "options",
        "decide",
        "decision",
        "prioritize",
        "scope",
        "plan this",
        "strategy",
        "complex",
        "ambiguous",
    ];

    question_marks > 1
        || word_count > 30
        || normalized.len() > 180
        || ambiguity_markers
            .iter()
            .any(|marker| normalized.contains(marker))
}

fn quiet_hours_summary(policy: &ChannelPolicy) -> String {
    match (
        policy.call_quiet_hours_start.as_deref(),
        policy.call_quiet_hours_end.as_deref(),
    ) {
        (Some(start), Some(end)) if !start.trim().is_empty() && !end.trim().is_empty() => {
            format!("call quiet hours are active between {start} and {end}")
        }
        _ => "call quiet hours are active".to_string(),
    }
}

pub async fn route_planner_contact(
    state: &AppState,
    request: PlannerContactRequest,
) -> Result<PlannerContactResult, AppError> {
    let destination = request.to.trim();
    if destination.is_empty() {
        return Err(AppError::Validation(
            "Destination cannot be empty".to_string(),
        ));
    }

    let content = request.content.trim();
    if content.is_empty() {
        return Err(AppError::Validation(
            "Opening message cannot be empty".to_string(),
        ));
    }

    let policy = resolve_channel_policy(state).await?;
    let config = resolve_twilio_config(state).await?;
    let whatsapp_available = supports_whatsapp(&config);
    let voice_available = supports_voice(&config);
    let explicit_channel = request
        .preferred_channel
        .as_deref()
        .and_then(ChannelKind::from_input);
    let allow_after_hours = request.allow_after_hours.unwrap_or(false);
    let discussion_worthy = should_escalate_to_call(content);
    let mut reason_parts: Vec<String> = Vec::new();

    let mut selected_channel = if let Some(channel) = explicit_channel {
        reason_parts.push(format!("explicitly requested {}", channel.as_str()));
        channel
    } else {
        match policy.preference {
            ChannelPreference::Whatsapp => {
                reason_parts.push("policy prefers WhatsApp for routine outreach".to_string());
                ChannelKind::Whatsapp
            }
            ChannelPreference::Voice => {
                reason_parts.push("policy prefers voice calls".to_string());
                ChannelKind::Voice
            }
            ChannelPreference::Hybrid => {
                if policy.escalate_to_call_on_ambiguity && discussion_worthy {
                    reason_parts.push(
                        "hybrid policy escalated this request to a call because it looks exploratory or ambiguous"
                            .to_string(),
                    );
                    ChannelKind::Voice
                } else {
                    reason_parts.push(
                        "hybrid policy kept this on WhatsApp as a routine update".to_string(),
                    );
                    ChannelKind::Whatsapp
                }
            }
        }
    };

    if selected_channel == ChannelKind::Voice && !allow_after_hours && policy.in_call_quiet_hours {
        if whatsapp_available {
            selected_channel = ChannelKind::Whatsapp;
            reason_parts.push(format!(
                "{} so the planner fell back to WhatsApp",
                quiet_hours_summary(&policy)
            ));
        } else {
            return Ok(PlannerContactResult {
                channel: "voice".to_string(),
                status: "blocked".to_string(),
                reason: format!(
                    "{} and WhatsApp fallback is not configured",
                    quiet_hours_summary(&policy)
                ),
            });
        }
    }

    if selected_channel == ChannelKind::Voice && looks_like_whatsapp_destination(destination) {
        if whatsapp_available {
            selected_channel = ChannelKind::Whatsapp;
            reason_parts.push(
                "destination is WhatsApp-formatted, so the planner stayed on WhatsApp".to_string(),
            );
        } else {
            return Err(AppError::Validation(
                "Voice calls need a phone number like +15551234567, not a WhatsApp-prefixed destination"
                    .to_string(),
            ));
        }
    }

    if selected_channel == ChannelKind::Whatsapp && !whatsapp_available {
        if voice_available {
            selected_channel = ChannelKind::Voice;
            reason_parts
                .push("WhatsApp is not configured, so the planner fell back to voice".to_string());
        } else {
            return Err(AppError::Validation(
                "Twilio WhatsApp is not configured and there is no voice fallback available"
                    .to_string(),
            ));
        }
    }

    if selected_channel == ChannelKind::Voice && !voice_available {
        if whatsapp_available {
            selected_channel = ChannelKind::Whatsapp;
            reason_parts.push(
                "voice calling is not configured, so the planner fell back to WhatsApp".to_string(),
            );
        } else {
            return Err(AppError::Validation(
                "Twilio voice is not configured and there is no WhatsApp fallback available"
                    .to_string(),
            ));
        }
    }

    match selected_channel {
        ChannelKind::Whatsapp => {
            send_whatsapp_message(
                state,
                normalize_whatsapp_destination(destination).to_string(),
                content.to_string(),
            )
            .await?;
        }
        ChannelKind::Voice => {
            start_voice_call(
                state,
                normalize_voice_destination(destination).to_string(),
                Some(content.to_string()),
            )
            .await?;
        }
    }

    Ok(PlannerContactResult {
        channel: selected_channel.as_str().to_string(),
        status: "sent".to_string(),
        reason: reason_parts.join(". "),
    })
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

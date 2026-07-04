use crate::error::AppError;
use crate::persistence::{planner_repo, settings_repo};
use crate::services::channel_contact_policy::{
    looks_like_whatsapp_destination, normalize_voice_destination, normalize_whatsapp_destination,
    quiet_hours_summary, resolve_channel_policy, select_initial_channel, ChannelKind,
};
use crate::services::channel_twilio::{supports_voice, supports_whatsapp};
use crate::services::planner_service::{
    create_planner_session, submit_planner_voice_turn, PlannerTurnResponse,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};

pub use crate::services::channel_twilio::{
    resolve_twilio_config, send_whatsapp_message, start_voice_call,
};

const DEFAULT_PROVIDER_SETTING_KEY: &str = "planner.default_provider_id";
const DEFAULT_MODEL_SETTING_KEY: &str = "planner.default_model_name";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInboundMessage {
    pub channel: String,
    pub remote_user_id: String,
    pub remote_conversation_id: String,
    pub content: String,
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
    submit_planner_voice_turn(
        state.planner_service.clone(),
        state,
        session_id,
        message.content,
        None,
        None,
    )
    .await
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
    let mut reason_parts: Vec<String> = Vec::new();
    let mut selected_channel =
        select_initial_channel(&policy, explicit_channel, content, &mut reason_parts);

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

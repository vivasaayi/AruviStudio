use crate::error::AppError;
use crate::persistence::settings_repo;
use crate::state::AppState;
use chrono::{Local, NaiveTime};
use serde::{Deserialize, Serialize};

const CHANNEL_PREFERENCE_KEY: &str = "planner.channel_preference";
const ESCALATE_TO_CALL_ON_AMBIGUITY_KEY: &str = "planner.escalate_to_call_on_ambiguity";
const CALL_QUIET_HOURS_START_KEY: &str = "planner.call_quiet_hours_start";
const CALL_QUIET_HOURS_END_KEY: &str = "planner.call_quiet_hours_end";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Whatsapp,
    Voice,
}

impl ChannelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Whatsapp => "whatsapp",
            Self::Voice => "voice",
        }
    }

    pub fn from_input(value: &str) -> Option<Self> {
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
pub struct ChannelPolicy {
    preference: ChannelPreference,
    escalate_to_call_on_ambiguity: bool,
    call_quiet_hours_start: Option<String>,
    call_quiet_hours_end: Option<String>,
    pub in_call_quiet_hours: bool,
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

pub async fn resolve_channel_policy(state: &AppState) -> Result<ChannelPolicy, AppError> {
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

pub fn select_initial_channel(
    policy: &ChannelPolicy,
    explicit_channel: Option<ChannelKind>,
    content: &str,
    reason_parts: &mut Vec<String>,
) -> ChannelKind {
    if let Some(channel) = explicit_channel {
        reason_parts.push(format!("explicitly requested {}", channel.as_str()));
        return channel;
    }

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
            if policy.escalate_to_call_on_ambiguity && should_escalate_to_call(content) {
                reason_parts.push(
                    "hybrid policy escalated this request to a call because it looks exploratory or ambiguous"
                        .to_string(),
                );
                ChannelKind::Voice
            } else {
                reason_parts
                    .push("hybrid policy kept this on WhatsApp as a routine update".to_string());
                ChannelKind::Whatsapp
            }
        }
    }
}

pub fn normalize_whatsapp_destination(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.to_ascii_lowercase().starts_with("whatsapp:") {
        trimmed.to_string()
    } else {
        format!("whatsapp:{trimmed}")
    }
}

pub fn normalize_voice_destination(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.to_ascii_lowercase().starts_with("whatsapp:") {
        trimmed["whatsapp:".len()..].to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn looks_like_whatsapp_destination(value: &str) -> bool {
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

pub fn quiet_hours_summary(policy: &ChannelPolicy) -> String {
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

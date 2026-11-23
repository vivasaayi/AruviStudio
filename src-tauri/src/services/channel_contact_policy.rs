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
    is_in_quiet_hours_at(start, end, Local::now().time())
}

fn is_in_quiet_hours_at(start: Option<NaiveTime>, end: Option<NaiveTime>, now: NaiveTime) -> bool {
    let (Some(start), Some(end)) = (start, end) else {
        return false;
    };
    if start == end {
        return false;
    }
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

#[cfg(test)]
mod quiet_hours_tests {
    use super::*;

    fn time(value: &str) -> NaiveTime {
        parse_hhmm(Some(value)).unwrap()
    }

    #[test]
    fn quiet_hours_include_start_exclude_end_and_cross_midnight() {
        let within = |start, end, now| is_in_quiet_hours_at(Some(time(start)), Some(time(end)), time(now));
        assert!(!within("09:00", "17:00", "08:59"));
        assert!(within("09:00", "17:00", "09:00"));
        assert!(within("09:00", "17:00", "16:59"));
        assert!(!within("09:00", "17:00", "17:00"));
        assert!(within("22:00", "06:00", "23:59"));
        assert!(within("22:00", "06:00", "00:00"));
        assert!(within("22:00", "06:00", "05:59"));
        assert!(!within("22:00", "06:00", "06:00"));
        assert!(!within("12:00", "12:00", "12:00"));
        assert!(!is_in_quiet_hours_at(None, Some(time("06:00")), time("01:00")));
        assert!(parse_hhmm(Some("24:00")).is_none());
        assert!(parse_hhmm(Some("23:60")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn quiet_hours_dst_child() {
        use chrono::{DateTime, Utc};
        if std::env::var_os("ARUVI_QUIET_HOURS_DST_CHILD").is_none() { return; }
        let local_time = |epoch| DateTime::<Utc>::from_timestamp(epoch, 0).unwrap().with_timezone(&Local).time();
        // Both occurrences of 01:30 on the fall-back day are inside the window.
        for epoch in [1_730_611_800, 1_730_615_400] {
            assert_eq!(local_time(epoch), time("01:30"));
            assert!(is_in_quiet_hours_at(Some(time("01:00")), Some(time("02:00")), local_time(epoch)));
        }
        // The spring-forward jump moves straight from 01:30 to 03:30.
        assert_eq!(local_time(1_710_052_200), time("01:30"));
        assert_eq!(local_time(1_710_055_800), time("03:30"));
        assert!(!is_in_quiet_hours_at(Some(time("02:00")), Some(time("03:00")), local_time(1_710_055_800)));
    }

    #[cfg(unix)]
    #[test]
    fn quiet_hours_use_local_dst_in_a_separate_process() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "services::channel_contact_policy::quiet_hours_tests::quiet_hours_dst_child"])
            .env("ARUVI_QUIET_HOURS_DST_CHILD", "1")
            .env("TZ", "EST5EDT,M3.2.0,M11.1.0")
            .status().unwrap();
        assert!(status.success());
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

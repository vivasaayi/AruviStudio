use crate::error::AppError;
use crate::services::channel_service::{self, PlannerContactRequest};
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "send_twilio_whatsapp_message" => {
            channel_service::send_whatsapp_message(
                state,
                args.required_string(&["to"], "to")?,
                args.required_string(&["content"], "content")?,
            )
            .await?;
            Ok(action_ok("send_twilio_whatsapp_message"))
        }
        "start_twilio_voice_call" => {
            channel_service::start_voice_call(
                state,
                args.required_string(&["to"], "to")?,
                args.optional_string(&["initial_prompt", "initialPrompt"])?,
            )
            .await?;
            Ok(action_ok("start_twilio_voice_call"))
        }
        "route_planner_contact" => action_result(
            "route_planner_contact",
            channel_service::route_planner_contact(
                state,
                PlannerContactRequest {
                    to: args.required_string(&["to"], "to")?,
                    content: args.required_string(&["content"], "content")?,
                    preferred_channel: args
                        .optional_string(&["preferred_channel", "preferredChannel"])?,
                    allow_after_hours: args
                        .optional_bool(&["allow_after_hours", "allowAfterHours"])?,
                },
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_channels action: {other}"
        ))),
    }
}

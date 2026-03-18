use crate::error::AppError;
use crate::services::channel_service::{
    route_planner_contact, send_whatsapp_message, start_voice_call, PlannerContactRequest,
    PlannerContactResult,
};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn send_twilio_whatsapp_message(
    state: State<'_, AppState>,
    to: String,
    content: String,
) -> Result<(), AppError> {
    send_whatsapp_message(&state, to, content).await
}

#[tauri::command]
pub async fn start_twilio_voice_call(
    state: State<'_, AppState>,
    to: String,
    initial_prompt: Option<String>,
) -> Result<(), AppError> {
    start_voice_call(&state, to, initial_prompt).await
}

#[tauri::command]
pub async fn route_planner_contact_command(
    state: State<'_, AppState>,
    to: String,
    content: String,
    preferred_channel: Option<String>,
    allow_after_hours: Option<bool>,
) -> Result<PlannerContactResult, AppError> {
    route_planner_contact(
        &state,
        PlannerContactRequest {
            to,
            content,
            preferred_channel,
            allow_after_hours,
        },
    )
    .await
}

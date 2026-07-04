use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::persistence::{model_repo, settings_repo};
use crate::services::speech_service::{
    looks_like_transcription_model, speak_text_natively, transcribe_audio_with_provider,
    SpeechToTextRequest, TextToSpeechRequest,
};
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "transcribe_audio" => {
            let provider_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_provider_id").await?;
            let model_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_model_name").await?;
            let provider_id = args
                .optional_string(&["provider_id", "providerId"])?
                .filter(|value| !value.trim().is_empty())
                .or(provider_setting)
                .ok_or_else(|| {
                    AppError::Validation("A speech transcription provider is required".to_string())
                })?;
            let requested_model_name = args
                .optional_string(&["model_name", "modelName"])?
                .filter(|value| !value.trim().is_empty())
                .or(model_setting);
            let provider_models = model_repo::list_model_definitions(&state.db)
                .await?
                .into_iter()
                .filter(|model| model.provider_id == provider_id);
            let speech_models = provider_models
                .filter(model_supports_transcription)
                .collect::<Vec<_>>();
            let model_name = if let Some(model_name) = requested_model_name {
                let known_model = speech_models.iter().any(|model| model.name == model_name);
                if known_model || looks_like_transcription_model(&model_name) {
                    model_name
                } else {
                    return Err(AppError::Validation(format!(
                        "Configured speech model '{}' does not look like a transcription model for this provider. Choose a Whisper/STT model in Settings.",
                        model_name
                    )));
                }
            } else if let Some(model) = speech_models.first() {
                model.name.clone()
            } else {
                "whisper-1".to_string()
            };
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let transcript = transcribe_audio_with_provider(
                &provider,
                &model_name,
                SpeechToTextRequest {
                    audio_bytes_base64: args.required_string(
                        &["audio_bytes_base64", "audioBytesBase64"],
                        "audio_bytes_base64",
                    )?,
                    mime_type: args.required_string(&["mime_type", "mimeType"], "mime_type")?,
                    locale: args
                        .optional_string(&["locale"])?
                        .or(settings_repo::get_setting(&state.db, "speech.locale").await?),
                },
            )
            .await?;
            action_result("transcribe_audio", transcript)
        }
        "speak_text_natively" => {
            let voice = args
                .optional_string(&["voice"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.native_voice").await?);
            let locale = args
                .optional_string(&["locale"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.locale").await?);
            speak_text_natively(TextToSpeechRequest {
                text: args.required_string(&["text"], "text")?,
                voice,
                locale,
            })?;
            Ok(action_ok("speak_text_natively"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_speech action: {other}"
        ))),
    }
}

fn model_supports_transcription(model: &ModelDefinition) -> bool {
    model.enabled
        && (model
            .capability_tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "speech_to_text" | "transcription" | "audio"))
            || looks_like_transcription_model(&model.name))
}

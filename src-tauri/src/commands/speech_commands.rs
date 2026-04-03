use crate::error::AppError;
use crate::persistence::{model_repo, settings_repo};
use crate::services::speech_service::{
    looks_like_transcription_model, speak_text_natively, transcribe_audio_with_provider,
    SpeechToTextRequest, SpeechToTextResponse, TextToSpeechRequest,
};
use crate::state::AppState;
use tauri::State;

const SPEECH_PROVIDER_KEY: &str = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY: &str = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY: &str = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY: &str = "speech.native_voice";

fn model_supports_transcription(model: &crate::domain::model::ModelDefinition) -> bool {
    model.enabled
        && (model
            .capability_tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "speech_to_text" | "transcription" | "audio"))
            || looks_like_transcription_model(&model.name))
}

#[tauri::command]
pub async fn transcribe_audio_command(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    providerId: Option<String>,
    model_name: Option<String>,
    modelName: Option<String>,
    audio_bytes_base64: String,
    audioBytesBase64: Option<String>,
    mime_type: String,
    mimeType: Option<String>,
    locale: Option<String>,
) -> Result<SpeechToTextResponse, AppError> {
    let provider_setting = settings_repo::get_setting(&state.db, SPEECH_PROVIDER_KEY).await?;
    let model_setting = settings_repo::get_setting(&state.db, SPEECH_MODEL_KEY).await?;
    let provider_id = provider_id
        .or(providerId)
        .filter(|value| !value.trim().is_empty())
        .or(provider_setting)
        .ok_or_else(|| {
            AppError::Validation("A speech transcription provider is required".to_string())
        })?;
    let requested_model_name = model_name
        .or(modelName)
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
    let request = SpeechToTextRequest {
        audio_bytes_base64: audioBytesBase64.unwrap_or(audio_bytes_base64),
        mime_type: mimeType.unwrap_or(mime_type),
        locale: locale.or(settings_repo::get_setting(&state.db, SPEECH_LOCALE_KEY).await?),
    };
    let provider = model_repo::get_provider(&state.db, &provider_id).await?;
    transcribe_audio_with_provider(&provider, &model_name, request).await
}

#[tauri::command]
pub async fn speak_text_natively_command(
    state: State<'_, AppState>,
    text: String,
    voice: Option<String>,
    locale: Option<String>,
) -> Result<(), AppError> {
    let resolved_voice = voice
        .filter(|value| !value.trim().is_empty())
        .or(settings_repo::get_setting(&state.db, SPEECH_NATIVE_VOICE_KEY).await?);
    let resolved_locale = locale
        .filter(|value| !value.trim().is_empty())
        .or(settings_repo::get_setting(&state.db, SPEECH_LOCALE_KEY).await?);
    speak_text_natively(TextToSpeechRequest {
        text,
        voice: resolved_voice,
        locale: resolved_locale,
    })
}

use crate::error::AppError;
use crate::persistence::{model_repo, settings_repo};
use crate::services::speech_service::{
    speak_text_natively, transcribe_audio_with_provider, SpeechToTextRequest,
    SpeechToTextResponse, TextToSpeechRequest,
};
use crate::state::AppState;
use tauri::State;

const SPEECH_PROVIDER_KEY: &str = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY: &str = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY: &str = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY: &str = "speech.native_voice";

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
    let provider_id = provider_id
        .or(providerId)
        .filter(|value| !value.trim().is_empty())
        .or(settings_repo::get_setting(&state.db, SPEECH_PROVIDER_KEY).await?)
        .ok_or_else(|| {
            AppError::Validation("A speech transcription provider is required".to_string())
        })?;
    let model_name = model_name
        .or(modelName)
        .filter(|value| !value.trim().is_empty())
        .or(settings_repo::get_setting(&state.db, SPEECH_MODEL_KEY).await?)
        .unwrap_or_else(|| "whisper-1".to_string());
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

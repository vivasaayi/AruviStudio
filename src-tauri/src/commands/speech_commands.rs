use crate::error::AppError;
use crate::persistence::{model_repo, settings_repo};
use crate::services::speech_service::{
    compatible_speech_model_default, configured_transcription_mode, default_transcription_model, resolve_transcription_defaults, speak_text_natively, transcribe_audio_with_provider_mode,
    SpeechToTextRequest, SpeechToTextResponse, TextToSpeechRequest,
};
use crate::state::AppState;
use tauri::State;

const SPEECH_LOCALE_KEY: &str = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY: &str = "speech.native_voice";

#[tauri::command]
pub async fn transcribe_audio_command(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    model_name: Option<String>,
    audio_bytes_base64: String,
    mime_type: String,
    locale: Option<String>,
) -> Result<SpeechToTextResponse, AppError> {
    let (provider_setting, model_setting, locale_setting) = resolve_transcription_defaults(&state.db).await?;
    let default_model = compatible_speech_model_default(provider_id.as_deref(), provider_setting.as_deref(), model_setting);
    let provider_id = provider_id
        .filter(|value| !value.trim().is_empty())
        .or(provider_setting.filter(|value| !value.trim().is_empty()))
        .ok_or_else(|| {
            AppError::Validation("A speech transcription provider is required".to_string())
        })?;
    let requested_model_name = model_name
        .filter(|value| !value.trim().is_empty())
        .or(default_model);
    let model_name = match requested_model_name {
        Some(model_name) => model_name,
        None => default_transcription_model(&state.db, &provider_id).await?,
    };
    let request = SpeechToTextRequest {
        audio_bytes_base64,
        mime_type,
        locale: locale.filter(|value| !value.trim().is_empty()).or(locale_setting),
    };
    let provider = model_repo::get_provider(&state.db, &provider_id).await?;
    let mode = configured_transcription_mode(&state.db, &provider, &model_name).await?;
    transcribe_audio_with_provider_mode(&provider, &model_name, request, mode).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::io::{Read, Write};
    use tauri::Manager;

    #[tokio::test]
    async fn desktop_transcribes_with_explicit_custom_model_name() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
            let len: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < end + len { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]); }
            assert!(bytes[end..end + len].windows(b"custom-voice-v2".len()).any(|part| part == b"custom-voice-v2"));
            assert!(bytes[end..end + len].windows(b"name=\"languages[]\"\r\n\r\nta\r\n".len()).any(|part| part == b"name=\"languages[]\"\r\n\r\nta\r\n"));
            let response = r#"{"text":"desktop custom transcript"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        });
        let app = crate::commands::test_helpers::make_test_app("desktop_custom_speech").await;
        let state = app.state::<AppState>();
        model_repo::create_provider(&state.db, "custom-speech", "Custom speech", "openai_compatible", &format!("http://{address}"), None).await.unwrap();
        settings_repo::set_setting(&state.db, SPEECH_LOCALE_KEY, "ta-IN").await.unwrap();
        settings_repo::set_setting(&state.db, &crate::services::speech_service::transcription_language_mode_key("custom-speech"), "array").await.unwrap();
        let response = transcribe_audio_command(state, Some("custom-speech".into()), Some("custom-voice-v2".into()),
            base64::engine::general_purpose::STANDARD.encode(b"synthetic audio"), "audio/wav".into(), Some("  ".into())).await.unwrap();
        assert_eq!(response.transcript, "desktop custom transcript");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn desktop_transcription_environment_fixture() {
        if std::env::var_os("ARUVI_SPEECH_TEST_CHILD").is_none() { return; }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new(); let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]);
                if let Some(i) = data.windows(4).position(|part| part == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
            let len: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < end + len { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]); }
            let body = String::from_utf8_lossy(&data[end..end+len]);
            assert!(body.contains("name=\"model\"\r\n\r\nenv-model\r\n"));
            assert!(body.contains("name=\"language\"\r\n\r\nta\r\n"));
            let reply = r#"{"text":"environment fixture"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let app = crate::commands::test_helpers::make_test_app("desktop_speech_environment").await;
        let state = app.state::<AppState>();
        model_repo::create_provider(&state.db, "env-provider", "Environment fixture", "openai_compatible", &format!("http://{address}"), None).await.unwrap();
        settings_repo::set_setting(&state.db, "speech.transcription_provider_id", "saved-provider").await.unwrap();
        settings_repo::set_setting(&state.db, "speech.transcription_model_name", "saved-model").await.unwrap();
        settings_repo::set_setting(&state.db, "speech.locale", "fr-FR").await.unwrap();
        let response = transcribe_audio_command(state, None, None, base64::engine::general_purpose::STANDARD.encode(b"synthetic audio"), "audio/wav".into(), None).await.unwrap();
        assert_eq!(response.transcript, "environment fixture");
        server.join().unwrap();
    }

    #[test]
    fn desktop_transcription_environment_overrides_saved_defaults() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "commands::speech_commands::tests::desktop_transcription_environment_fixture", "--nocapture"])
            .env("ARUVI_SPEECH_TEST_CHILD", "1")
            .env("ARUVI_SPEECH_PROVIDER_ID", "env-provider")
            .env("ARUVI_SPEECH_MODEL_NAME", "env-model")
            .env("ARUVI_SPEECH_LOCALE", "ta-IN")
            .status().unwrap();
        assert!(status.success());
    }
}

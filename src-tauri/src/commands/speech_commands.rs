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
    fallback_locale: Option<String>,
    fallback_provider_id: Option<String>,
    fallback_model_name: Option<String>,
) -> Result<SpeechToTextResponse, AppError> {
    let resolved = resolve_desktop_transcription(&state, provider_id, model_name, locale, fallback_locale, fallback_provider_id, fallback_model_name).await?;
    let request = SpeechToTextRequest { audio_bytes_base64, mime_type, locale: resolved.locale };
    let mode = configured_transcription_mode(&state.db, &resolved.provider, &resolved.model_name).await?;
    transcribe_audio_with_provider_mode(&resolved.provider, &resolved.model_name, request, mode).await
}

struct ResolvedDesktopTranscription {
    provider: crate::domain::model::ModelProvider,
    model_name: String,
    locale: Option<String>,
}

async fn resolve_desktop_transcription(
    state: &AppState,
    provider_id: Option<String>, model_name: Option<String>, locale: Option<String>,
    fallback_locale: Option<String>, fallback_provider_id: Option<String>, fallback_model_name: Option<String>,
) -> Result<ResolvedDesktopTranscription, AppError> {
    let (provider_setting, model_setting, locale_setting) = resolve_transcription_defaults(&state.db).await?;
    let fallback_provider = fallback_provider_id.filter(|value| !value.trim().is_empty());
    let configured_provider = provider_setting.filter(|value| !value.trim().is_empty());
    let provider_id = provider_id
        .filter(|value| !value.trim().is_empty())
        .or_else(|| configured_provider.clone())
        .or_else(|| fallback_provider.clone())
        .ok_or_else(|| {
            AppError::Validation("A speech transcription provider is required".to_string())
        })?;
    let requested_model_name = select_desktop_transcription_model(&provider_id, configured_provider.as_deref(), model_name, model_setting, fallback_provider.as_deref(), fallback_model_name);
    let model_name = match requested_model_name {
        Some(model_name) => model_name,
        None => default_transcription_model(&state.db, &provider_id).await?,
    };
    let provider = model_repo::get_provider(&state.db, &provider_id).await?;
    Ok(ResolvedDesktopTranscription { provider, model_name, locale: select_desktop_transcription_locale(locale, locale_setting, fallback_locale) })
}

#[derive(serde::Serialize)]
pub struct DesktopTranscriptionTarget { provider_id: String, model_name: String }

/// Resolve configuration before opening the microphone; this does not contact
/// the provider or promise that the selected model is available.
#[tauri::command]
pub async fn get_desktop_transcription_target(state: State<'_, AppState>, fallback_locale: Option<String>) -> Result<DesktopTranscriptionTarget, AppError> {
    let resolved = resolve_desktop_transcription(&state, None, None, None, fallback_locale, None, None).await?;
    crate::services::speech_service::validate_transcription_target(&resolved.provider, &resolved.model_name)?;
    configured_transcription_mode(&state.db, &resolved.provider, &resolved.model_name).await?;
    Ok(DesktopTranscriptionTarget { provider_id: resolved.provider.id, model_name: resolved.model_name })
}

fn select_desktop_transcription_locale(explicit: Option<String>, configured: Option<String>, fallback: Option<String>) -> Option<String> {
    explicit.filter(|value| !value.trim().is_empty()).or(configured).or_else(|| fallback.filter(|value| !value.trim().is_empty()))
}

fn select_desktop_transcription_model(provider_id: &str, configured_provider: Option<&str>, explicit: Option<String>, configured: Option<String>, fallback_provider: Option<&str>, fallback: Option<String>) -> Option<String> {
    let configured = compatible_speech_model_default(Some(provider_id), configured_provider, configured);
    let fallback = if fallback_provider == Some(provider_id) { fallback.filter(|value| !value.trim().is_empty()) } else { None };
    explicit.filter(|value| !value.trim().is_empty()).or(configured).or(fallback)
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

    #[test]
    fn desktop_transcription_locale_prefers_explicit_then_configured_then_planner_fallback() {
        let select = super::select_desktop_transcription_locale;
        assert_eq!(select(Some("fr-FR".into()), Some("ta-IN".into()), Some("en-US".into())).as_deref(), Some("fr-FR"));
        assert_eq!(select(None, Some("ta-IN".into()), Some("en-US".into())).as_deref(), Some("ta-IN"));
        assert_eq!(select(None, Some("".into()), Some("en-US".into())).as_deref(), Some(""));
        assert_eq!(select(None, None, Some("en-US".into())).as_deref(), Some("en-US"));
        assert_eq!(select(None, None, None), None);
    }

    #[test]
    fn desktop_transcription_model_fallback_stays_with_its_provider() {
        let select = super::select_desktop_transcription_model;
        assert_eq!(select("env", Some("env"), None, Some("configured".into()), Some("planner"), Some("planner-model".into())).as_deref(), Some("configured"));
        assert_eq!(select("env", Some("env"), None, None, Some("planner"), Some("planner-model".into())), None);
        assert_eq!(select("planner", None, None, None, Some("planner"), Some("planner-model".into())).as_deref(), Some("planner-model"));
        assert_eq!(select("env", Some("env"), Some("explicit".into()), Some("configured".into()), Some("env"), Some("fallback".into())).as_deref(), Some("explicit"));
    }

    #[test]
    fn desktop_ipc_transcription_accepts_legacy_payload_without_fallback_fields() {
        use tauri::test::{mock_builder, mock_context, noop_assets, INVOKE_KEY};
        if std::env::var_os("ARUVI_SPEECH_IPC_CHILD").is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "commands::speech_commands::tests::desktop_ipc_transcription_accepts_legacy_payload_without_fallback_fields", "--nocapture"])
                .env("ARUVI_SPEECH_IPC_CHILD", "1").env_remove("ARUVI_SPEECH_PROVIDER_ID").env_remove("ARUVI_SPEECH_MODEL_NAME").env_remove("ARUVI_SPEECH_LOCALE").status().unwrap();
            assert!(status.success()); return;
        }
        let directory = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let state = runtime.block_on(async {
            let db = crate::persistence::db::create_pool(&format!("sqlite:{}", directory.path().join("speech.db").display())).await.unwrap();
            crate::state::AppState::new(db, directory.path().to_path_buf()).await.unwrap()
        });
        let app = mock_builder().manage(state).invoke_handler(tauri::generate_handler![super::transcribe_audio_command, super::get_desktop_transcription_target])
            .build(mock_context(noop_assets())).unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        let error = tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
            cmd: "transcribe_audio_command".into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(serde_json::json!({"providerId":"legacy-missing", "modelName":"fixture", "audioBytesBase64":"YQ==", "mimeType":"audio/wav"})),
            headers: Default::default(), invoke_key: INVOKE_KEY.to_string(),
        }).unwrap_err();
        assert!(format!("{error:?}").contains("Provider legacy-missing not found"), "{error:?}");
        let error = tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
            cmd: "transcribe_audio_command".into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(serde_json::json!({"fallbackProviderId":"fallback-missing", "fallbackModelName":"fixture", "fallbackLocale":"en-US", "audioBytesBase64":"YQ==", "mimeType":"audio/wav"})),
            headers: Default::default(), invoke_key: INVOKE_KEY.to_string(),
        }).unwrap_err();
        assert!(format!("{error:?}").contains("Provider fallback-missing not found"), "{error:?}");
        let no_provider = tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
            cmd: "get_desktop_transcription_target".into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(), body: tauri::ipc::InvokeBody::Json(serde_json::json!({"fallbackLocale":"en-US"})),
            headers: Default::default(), invoke_key: INVOKE_KEY.to_string(),
        }).unwrap_err();
        assert!(format!("{no_provider:?}").contains("speech transcription provider is required"), "{no_provider:?}");
        runtime.block_on(async {
            let state = app.state::<AppState>();
            model_repo::create_provider(&state.db, "configured", "Configured", "openai_compatible", "http://127.0.0.1:1234/v1", None).await.unwrap();
            settings_repo::set_setting(&state.db, "speech.transcription_provider_id", "configured").await.unwrap();
        });
        let target = tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
            cmd: "get_desktop_transcription_target".into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(), body: tauri::ipc::InvokeBody::Json(serde_json::json!({"fallbackLocale":"en-US"})),
            headers: Default::default(), invoke_key: INVOKE_KEY.to_string(),
        }).unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(target, serde_json::json!({"provider_id":"configured", "model_name":"whisper-1"}));
    }

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
            base64::engine::general_purpose::STANDARD.encode(b"synthetic audio"), "audio/wav".into(), Some("  ".into()), None, None, None).await.unwrap();
        assert_eq!(response.transcript, "desktop custom transcript");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn desktop_transcription_environment_fixture() {
        if std::env::var_os("ARUVI_SPEECH_TEST_CHILD").is_none() { return; }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
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
            }
        });
        let app = crate::commands::test_helpers::make_test_app("desktop_speech_environment").await;
        let state = app.state::<AppState>();
        model_repo::create_provider(&state.db, "env-provider", "Environment fixture", "openai_compatible", &format!("http://{address}"), None).await.unwrap();
        settings_repo::set_setting(&state.db, "speech.transcription_provider_id", "saved-provider").await.unwrap();
        settings_repo::set_setting(&state.db, "speech.transcription_model_name", "saved-model").await.unwrap();
        settings_repo::set_setting(&state.db, "speech.locale", "fr-FR").await.unwrap();
        let target = get_desktop_transcription_target(app.state::<AppState>(), Some("en-US".into())).await.unwrap();
        assert_eq!(target.provider_id, "env-provider"); assert_eq!(target.model_name, "env-model");
        let response = transcribe_audio_command(state, None, None, base64::engine::general_purpose::STANDARD.encode(b"synthetic audio"), "audio/wav".into(), None, Some("en-US".into()), Some("stale-planner-provider".into()), Some("stale-planner-model".into())).await.unwrap();
        assert_eq!(response.transcript, "environment fixture");
        let response = transcribe_audio_command(app.state::<AppState>(), None, None, base64::engine::general_purpose::STANDARD.encode(b"synthetic audio"), "audio/wav".into(), None, Some("en-US".into()), None, None).await.unwrap();
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

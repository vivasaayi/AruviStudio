use crate::domain::model::{ModelProvider, ProviderType};
use crate::error::AppError;
use base64::Engine;
use hound::{SampleFormat, WavReader};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const STUDIO_TRANSCRIPTION_MAX_BYTES: usize = 32 * 1024 * 1024;
const DEEPSEEK_TRANSCRIPTION_MAX_BYTES: usize = 8 * 1024 * 1024;
const LOCAL_WHISPER_MAX_DURATION_SECONDS: u64 = 10 * 60;
const DEEPSEEK_TRANSCRIPTION: &str = "deepseek-voice-transcription";
const DEEPSEEK_SYNTHESIS: &str = "deepseek-voice-synthesis";

pub(crate) fn is_voice_only_selector(name: &str) -> bool {
    let name = name.trim();
    name.eq_ignore_ascii_case(DEEPSEEK_TRANSCRIPTION) || name.eq_ignore_ascii_case(DEEPSEEK_SYNTHESIS)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToTextRequest {
    pub audio_bytes_base64: String,
    pub mime_type: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToTextResponse {
    pub transcript: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechRequest {
    pub text: String,
    pub voice: Option<String>,
    pub locale: Option<String>,
}

pub async fn synthesize_remote_speech(
    provider: &ModelProvider,
    model: String,
    input: String,
    voice: Option<String>,
    speed: Option<f32>,
) -> Result<Vec<u8>, AppError> {
    if !provider.enabled { return Err(AppError::Validation("Selected provider is disabled".into())); }
    if model.trim().eq_ignore_ascii_case(DEEPSEEK_TRANSCRIPTION) {
        return Err(AppError::Validation("The DeepSeek transcription selector cannot generate speech".into()));
    }
    if provider.protocol != "openai_compatible" {
        return Err(AppError::Validation("Selected provider protocol does not support remote speech synthesis".into()));
    }
    if model.trim().eq_ignore_ascii_case(DEEPSEEK_SYNTHESIS) {
        let client = crate::services::model_service::create_deepseek_voice_client(provider)?;
        return client.generate_speech(aruvi_model_client::SpeechRequest {
            model, input, voice, speed, response_format: "wav".into(),
        }).await.map_err(|error| match error {
            aruvi_model_client::ClientError::InvalidInput(message) => AppError::Validation(message),
            other => AppError::from(other),
        });
    }
    let client = crate::services::model_service::create_shared_gateway(provider)?;
    client.generate_speech(aruvi_model_client::SpeechRequest { model, input, voice, speed, response_format: "wav".into() }).await
        .map_err(|error| match error {
            aruvi_model_client::ClientError::InvalidInput(message) => AppError::Validation(message),
            other => AppError::from(other),
        })
}

pub async fn transcribe_audio_with_provider(
    provider: &ModelProvider,
    model_name: &str,
    request: SpeechToTextRequest,
) -> Result<SpeechToTextResponse, AppError> {
    transcribe_audio_with_provider_mode(provider, model_name, request, aruvi_model_client::openai_compatible::TranscriptionLanguageMode::Auto).await
}

pub(crate) fn transcription_language_mode_key(provider_id: &str) -> String {
    format!("model.provider.{provider_id}.transcription_language_mode")
}

pub(crate) fn parse_transcription_language_mode(value: Option<&str>) -> Result<aruvi_model_client::openai_compatible::TranscriptionLanguageMode, AppError> {
    use aruvi_model_client::openai_compatible::TranscriptionLanguageMode;
    match value {
        None | Some("auto") => Ok(TranscriptionLanguageMode::Auto),
        Some("singular") => Ok(TranscriptionLanguageMode::Singular),
        Some("array") => Ok(TranscriptionLanguageMode::Array),
        _ => Err(AppError::Validation("Invalid transcription language mode".into())),
    }
}

pub(crate) async fn configured_transcription_mode(db: &sqlx::SqlitePool, provider: &ModelProvider, model_name: &str) -> Result<aruvi_model_client::openai_compatible::TranscriptionLanguageMode, AppError> {
    if !matches!(provider.provider_type, ProviderType::OpenaiCompatible) || provider.protocol != "openai_compatible" || model_name.trim().eq_ignore_ascii_case(DEEPSEEK_TRANSCRIPTION) {
        return Ok(aruvi_model_client::openai_compatible::TranscriptionLanguageMode::Auto);
    }
    let raw = crate::persistence::settings_repo::get_setting(db, &transcription_language_mode_key(&provider.id)).await?;
    parse_transcription_language_mode(raw.as_deref())
}

pub(crate) async fn transcribe_audio_with_provider_mode(
    provider: &ModelProvider,
    model_name: &str,
    request: SpeechToTextRequest,
    language_mode: aruvi_model_client::openai_compatible::TranscriptionLanguageMode,
) -> Result<SpeechToTextResponse, AppError> {
    if !provider.enabled {
        return Err(AppError::Validation("Selected provider is disabled".into()));
    }
    if model_name.trim().eq_ignore_ascii_case(DEEPSEEK_SYNTHESIS) {
        return Err(AppError::Validation("The DeepSeek synthesis selector cannot transcribe audio".into()));
    }
    if matches!(provider.provider_type, ProviderType::LocalRuntime) {
        return transcribe_audio_locally(provider, request).await;
    }

    if provider.protocol != "openai_compatible" {
        return Err(AppError::Validation("Selected provider protocol does not support remote transcription".into()));
    }
    let deepseek = model_name.trim().eq_ignore_ascii_case(DEEPSEEK_TRANSCRIPTION);
    let limit = if deepseek { DEEPSEEK_TRANSCRIPTION_MAX_BYTES } else { STUDIO_TRANSCRIPTION_MAX_BYTES };
    let file_bytes = decode_transcription_audio(&request.audio_bytes_base64, limit)?;
    if deepseek {
        let client = crate::services::model_service::create_deepseek_voice_client(provider)?;
        let filename = format!("planner-recording.{}", mime_extension_for_mime_type(&request.mime_type));
        return client.transcribe(aruvi_model_client::TranscriptionRequest {
            model: model_name.into(), audio: file_bytes, filename, mime_type: request.mime_type, language: None,
        }).await
            .map(|transcript| SpeechToTextResponse { transcript })
            .map_err(|error| match error {
                aruvi_model_client::ClientError::InvalidInput(message) => AppError::Validation(message),
                other => AppError::from(other),
            });
    }
    let client = crate::services::model_service::create_transcription_gateway(provider, language_mode)?;
    let result = client.transcribe(aruvi_model_client::TranscriptionRequest {
        model: model_name.to_string(),
        audio: file_bytes,
        filename: format!("planner-recording.{}", mime_extension_for_mime_type(&request.mime_type)),
        mime_type: request.mime_type,
        language: provider_transcription_language(provider, model_name, language_mode, request.locale.as_deref()),
    }).await;
    match result {
        Ok(transcript) => Ok(SpeechToTextResponse { transcript }),
        Err(aruvi_model_client::ClientError::UnsupportedTranscription) => Err(AppError::Validation(format!(
            "Provider '{}' with model '{}' does not appear to support OpenAI-compatible multipart audio transcription uploads. Configure a speech-capable provider/model, such as a Whisper or STT endpoint, in Settings.",
            provider.name, model_name
        ))),
        Err(aruvi_model_client::ClientError::InvalidInput(message)) => Err(AppError::Validation(message)),
        Err(error) => Err(AppError::from(error)),
    }
}

pub fn speak_text_natively(request: TextToSpeechRequest) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        if request.text.trim().is_empty() {
            return Ok(());
        }

        let mut command = Command::new("/usr/bin/say");
        if let Some(voice) = request.voice.filter(|value| !value.trim().is_empty()) {
            command.arg("-v").arg(voice);
        }
        command.arg(request.text);

        let output = command
            .output()
            .map_err(|error| AppError::Internal(format!("Failed to invoke macOS say: {error}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AppError::Internal(format!(
                "macOS say failed: {}",
                if stderr.is_empty() {
                    "unknown error".to_string()
                } else {
                    stderr
                }
            )));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        Err(AppError::Validation(
            "Native speech output is currently implemented for macOS only".to_string(),
        ))
    }
}

fn mime_extension_for_mime_type(mime_type: &str) -> &'static str {
    match mime_type {
        "audio/webm" | "audio/webm;codecs=opus" => "webm",
        "audio/mp4" | "audio/m4a" => "m4a",
        "audio/wav" | "audio/wave" => "wav",
        "audio/mpeg" => "mp3",
        _ => "bin",
    }
}

pub fn looks_like_transcription_model(name: &str) -> bool {
    let normalized = name.trim().to_lowercase();
    normalized == DEEPSEEK_TRANSCRIPTION
        || normalized.contains("whisper")
        || normalized.contains("transcrib")
        || normalized.contains("speech")
        || normalized.contains("stt")
        || normalized.contains("audio")
}

fn transcription_model_rank(model: &crate::domain::model::ModelDefinition) -> Option<u8> {
    if !model.enabled { return None; }
    if model.name.trim().eq_ignore_ascii_case(DEEPSEEK_SYNTHESIS) { return None; }
    let tags: Vec<_> = model.capability_tags.iter().map(|tag| tag.trim().to_ascii_lowercase().replace('-', "_")).collect();
    if tags.iter().any(|tag| matches!(tag.as_str(), "speech_to_text" | "transcription" | "stt" | "asr")) { return Some(0); }
    let name = model.name.to_ascii_lowercase();
    let synthesis_name = name.split(|c: char| !c.is_ascii_alphanumeric()).any(|part| matches!(part, "tts" | "synthesis" | "synthesizer"))
        || name.contains("text-to-speech") || name.contains("text_to_speech");
    let synthesis_tag = tags.iter().any(|tag| matches!(tag.as_str(), "tts" | "text_to_speech" | "speech_synthesis" | "synthesis"));
    if synthesis_name || synthesis_tag { return None; }
    (tags.iter().any(|tag| tag == "audio") || looks_like_transcription_model(&name)).then_some(1)
}

pub(crate) fn compatible_speech_model_default(requested_provider: Option<&str>, default_provider: Option<&str>, default_model: Option<String>) -> Option<String> {
    let requested_provider = requested_provider.filter(|value| !value.trim().is_empty());
    let default_provider = default_provider.filter(|value| !value.trim().is_empty());
    if requested_provider.zip(default_provider).is_some_and(|(requested, default)| requested != default) {
        None
    } else {
        default_model.filter(|value| !value.trim().is_empty())
    }
}

/// Explicit request values are handled by callers. Environment values retain
/// the browser API's existing precedence, including an explicitly blank value.
pub(crate) async fn resolve_transcription_defaults(db: &sqlx::SqlitePool) -> Result<(Option<String>, Option<String>, Option<String>), AppError> {
    use crate::persistence::settings_repo::get_setting;
    let saved_provider = get_setting(db, "speech.transcription_provider_id").await?;
    let saved_model = get_setting(db, "speech.transcription_model_name").await?;
    let saved_locale = get_setting(db, "speech.locale").await?;
    let env_provider = std::env::var("ARUVI_SPEECH_PROVIDER_ID").ok();
    let env_model = std::env::var("ARUVI_SPEECH_MODEL_NAME").ok();
    let model = compatible_environment_speech_model(env_provider.as_deref(), saved_provider.as_deref(), env_model, saved_model);
    Ok((
        env_provider.or(saved_provider),
        model,
        std::env::var("ARUVI_SPEECH_LOCALE").ok().or(saved_locale),
    ))
}

fn compatible_environment_speech_model(env_provider: Option<&str>, saved_provider: Option<&str>, env_model: Option<String>, saved_model: Option<String>) -> Option<String> {
    if env_model.is_some() { return env_model; }
    let env_provider = env_provider.filter(|value| !value.trim().is_empty());
    let saved_provider = saved_provider.filter(|value| !value.trim().is_empty());
    if env_provider.zip(saved_provider).is_some_and(|(env, saved)| env != saved) { None } else { saved_model }
}

pub(crate) async fn default_transcription_model(db: &sqlx::SqlitePool, provider_id: &str) -> Result<String, AppError> {
    let registered = crate::persistence::model_repo::list_model_definitions(db).await?
        .into_iter().filter(|model| model.provider_id == provider_id)
        .filter_map(|model| transcription_model_rank(&model).map(|rank| (rank, model)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, model)| model)
        .map(|model| model.name);
    if let Some(model) = registered { return Ok(model); }
    let provider = crate::persistence::model_repo::get_provider(db, provider_id).await?;
    Ok(implicit_transcription_model(&provider).to_string())
}

fn implicit_transcription_model(provider: &crate::domain::model::ModelProvider) -> &'static str {
    let official_openai = provider.provider_type.as_str() == "openai_compatible" && provider.protocol == "openai_compatible"
        && aruvi_model_client::openai_compatible::is_official_openai_base_url(&provider.base_url)
        && reqwest::Url::parse(&provider.base_url).is_ok_and(|url| matches!(url.path(), "" | "/" | "/v1" | "/v1/"));
    if official_openai { "gpt-transcribe" } else { "whisper-1" }
}

pub fn resolve_local_runtime_model_path(raw_value: &str) -> Result<PathBuf, AppError> {
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Local runtime provider requires a model path in the Base URL field.".to_string(),
        ));
    }

    let resolved = if let Some(rest) = trimmed.strip_prefix("file://") {
        let decoded = urlencoding::decode(rest)
            .map_err(|error| AppError::Validation(format!("Invalid local model path: {error}")))?;
        PathBuf::from(decoded.into_owned())
    } else {
        PathBuf::from(trimmed)
    };

    if !resolved.is_absolute() {
        return Err(AppError::Validation(format!(
            "Local runtime model path must be absolute: {}",
            resolved.display()
        )));
    }

    if !resolved.is_file() {
        return Err(AppError::Validation(format!(
            "Local runtime model file not found or not a regular file: {}",
            resolved.display()
        )));
    }

    Ok(resolved)
}

async fn transcribe_audio_locally(
    provider: &ModelProvider,
    request: SpeechToTextRequest,
) -> Result<SpeechToTextResponse, AppError> {
    let mime_type = request.mime_type.trim().to_lowercase();
    if !mime_type.starts_with("audio/wav") && !mime_type.starts_with("audio/x-wav") {
        return Err(AppError::Validation(
            "Local Whisper transcription currently expects WAV audio. Desktop voice capture now records WAV automatically; other clients should use hosted STT or upload WAV.".to_string(),
        ));
    }

    let model_path = resolve_local_runtime_model_path(&provider.base_url)?;
    let audio_bytes = decode_transcription_audio(&request.audio_bytes_base64, STUDIO_TRANSCRIPTION_MAX_BYTES)?;
    let (samples, sample_rate) = decode_wav_to_mono_f32(&audio_bytes)?;
    let resampled = resample_mono_audio(&samples, sample_rate, 16_000)?;
    let language = normalize_whisper_language(request.locale.as_deref());

    tokio::task::spawn_blocking(move || {
        let transcript = run_whisper_transcription(&model_path, &resampled, language.as_deref())?;
        Ok(SpeechToTextResponse { transcript })
    })
    .await
    .map_err(|error| AppError::Internal(format!("Local Whisper task failed: {error}")))?
}

fn decode_transcription_audio(encoded: &str, max_bytes: usize) -> Result<Vec<u8>, AppError> {
    let encoded = encoded.trim();
    let limit_message = || {
        let unit = 1024 * 1024;
        if max_bytes % unit == 0 { format!("Transcription audio exceeds the {} MiB limit.", max_bytes / unit) }
        else { format!("Transcription audio exceeds the {max_bytes}-byte limit.") }
    };
    // Check the encoded length before decoding so an oversized request does not
    // allocate a second, decoded copy of the entire payload.
    let max_encoded = max_bytes.div_ceil(3).saturating_mul(4);
    if encoded.len() > max_encoded {
        return Err(AppError::Validation(limit_message()));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)
        .map_err(|error| AppError::Validation(format!("Invalid audio payload: {error}")))?;
    if bytes.len() > max_bytes {
        return Err(AppError::Validation(limit_message()));
    }
    Ok(bytes)
}

fn decode_wav_to_mono_f32(bytes: &[u8]) -> Result<(Vec<f32>, u32), AppError> {
    let cursor = Cursor::new(bytes);
    let mut reader = WavReader::new(cursor)
        .map_err(|error| AppError::Validation(format!("Invalid WAV audio: {error}")))?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(AppError::Validation(
            "WAV audio must have at least one channel.".to_string(),
        ));
    }
    if spec.sample_rate == 0 {
        return Err(AppError::Validation("WAV audio must have a positive sample rate.".to_string()));
    }
    // Whisper consumes 16 kHz samples. Reject long or malformed files before
    // allocating decoded samples or multiplying their size during resampling.
    if u64::from(reader.duration()) > u64::from(spec.sample_rate) * LOCAL_WHISPER_MAX_DURATION_SECONDS {
        return Err(AppError::Validation("Local Whisper audio must be at most 10 minutes.".to_string()));
    }

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / i16::MAX as f32)
                    .map_err(|error| {
                        AppError::Validation(format!("Failed to read WAV samples: {error}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map_err(|error| {
                    AppError::Validation(format!("Failed to read WAV samples: {error}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported WAV format for local Whisper: {:?} {}-bit. Use 16-bit PCM WAV.",
                spec.sample_format, spec.bits_per_sample
            )))
        }
    };

    let channels = spec.channels as usize;
    let mono = if channels == 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
            .collect::<Vec<_>>()
    };

    Ok((mono, spec.sample_rate))
}

fn resample_mono_audio(samples: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>, AppError> {
    if source_rate == 0 || target_rate == 0 {
        return Err(AppError::Validation("Audio sample rates must be positive.".to_string()));
    }
    if samples.len() as u64 > u64::from(source_rate) * LOCAL_WHISPER_MAX_DURATION_SECONDS {
        return Err(AppError::Validation("Local Whisper audio must be at most 10 minutes.".to_string()));
    }
    if source_rate == target_rate || samples.is_empty() {
        return Ok(samples.to_vec());
    }

    let ratio = target_rate as f64 / source_rate as f64;
    let target_len = ((samples.len() as f64) * ratio).round().max(1.0) as usize;
    let mut output = Vec::with_capacity(target_len);

    for index in 0..target_len {
        let source_position = (index as f64) / ratio;
        let lower = source_position.floor() as usize;
        let upper = (lower + 1).min(samples.len().saturating_sub(1));
        let fraction = (source_position - lower as f64) as f32;
        let lower_value = samples[lower];
        let upper_value = samples[upper];
        output.push(lower_value + (upper_value - lower_value) * fraction);
    }

    Ok(output)
}

// Studio stores browser/native locales, whereas OpenAI-compatible speech APIs
// generally consume a language code. An omitted language permits detection.
fn transcription_language(locale: Option<&str>) -> Option<String> {
    normalize_whisper_language(locale).filter(|language| !language.is_empty() && language != "auto")
}

fn provider_transcription_language(provider: &ModelProvider, model_name: &str, mode: aruvi_model_client::openai_compatible::TranscriptionLanguageMode, locale: Option<&str>) -> Option<String> {
    use aruvi_model_client::openai_compatible::{is_official_openai_base_url, TranscriptionLanguageMode};
    if provider.provider_type.as_str() == "openai_compatible" && provider.protocol == "openai_compatible"
        && model_name == "gpt-transcribe" && mode == TranscriptionLanguageMode::Auto && is_official_openai_base_url(&provider.base_url) {
        return locale.map(str::trim).filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto")).map(str::to_owned);
    }
    transcription_language(locale)
}

fn normalize_whisper_language(locale: Option<&str>) -> Option<String> {
    locale
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.split(['-', '_']).next())
        .map(|value| value.to_lowercase())
}

fn run_whisper_transcription(
    model_path: &Path,
    samples: &[f32],
    language: Option<&str>,
) -> Result<String, AppError> {
    let ctx = WhisperContext::new_with_params(
        model_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid local model path.".to_string()))?,
        WhisperContextParameters::default(),
    )
    .map_err(|error| AppError::Internal(format!("Failed to load local Whisper model: {error}")))?;

    let mut state = ctx
        .create_state()
        .map_err(|error| AppError::Internal(format!("Failed to create Whisper state: {error}")))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_translate(false);
    params.set_n_threads(
        std::thread::available_parallelism()
            .map(|value| value.get().min(8) as i32)
            .unwrap_or(4),
    );
    if let Some(language) = language {
        params.set_language(Some(language));
    }

    state.full(params, samples).map_err(|error| {
        AppError::Internal(format!("Local Whisper transcription failed: {error}"))
    })?;

    let transcript = state
        .as_iter()
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if transcript.is_empty() {
        return Err(AppError::Provider(
            "Local Whisper returned an empty transcript".to_string(),
        ));
    }

    Ok(transcript)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn registered_transcription_models_override_provider_specific_fallbacks() {
        use crate::{commands::test_helpers::make_test_app, persistence::model_repo, state::AppState};
        use tauri::Manager;
        let app: tauri::App<tauri::test::MockRuntime> = make_test_app("speech_implicit_model").await;
        let state = app.state::<AppState>();
        model_repo::create_provider(&state.db, "official-speech", "OpenAI", "openai_compatible", "https://api.openai.com/v1", None).await.unwrap();
        model_repo::create_provider(&state.db, "proxy-speech", "Proxy", "openai_compatible", "http://127.0.0.1:1234/v1", None).await.unwrap();
        assert_eq!(super::default_transcription_model(&state.db, "official-speech").await.unwrap(), "gpt-transcribe");
        assert_eq!(super::default_transcription_model(&state.db, "proxy-speech").await.unwrap(), "whisper-1");
        model_repo::create_model_definition(&state.db, "explicit-speech", "official-speech", "custom-transcriber", None, Some("[\"speech_to_text\"]"), None).await.unwrap();
        assert_eq!(super::default_transcription_model(&state.db, "official-speech").await.unwrap(), "custom-transcriber");
        assert!(super::default_transcription_model(&state.db, "missing-provider").await.is_err());
    }

    #[test]
    fn implicit_transcription_model_changes_only_for_official_openai_api() {
        use crate::domain::model::{ModelProvider, ProviderType};
        let mut provider = ModelProvider { id: "fixture".into(), name: "Fixture".into(), provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "https://api.openai.com/v1".into(), auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        assert_eq!(super::implicit_transcription_model(&provider), "gpt-transcribe");
        for url in ["http://api.openai.com/v1", "https://api.openai.com:444/v1", "https://api.openai.com.example.invalid/v1", "https://api.openai.com/proxy", "http://localhost:1234/v1"] {
            provider.base_url = url.into(); assert_eq!(super::implicit_transcription_model(&provider), "whisper-1", "{url}");
        }
        provider.base_url = "https://api.openai.com/v1".into(); provider.protocol = "anthropic".into();
        assert_eq!(super::implicit_transcription_model(&provider), "whisper-1");
    }

    #[test]
    fn official_gpt_transcription_preserves_locale_for_shared_client() {
        use crate::domain::model::{ModelProvider, ProviderType};
        use aruvi_model_client::openai_compatible::TranscriptionLanguageMode;
        let mut provider = ModelProvider { id: "fixture".into(), name: "Fixture".into(), provider_type: ProviderType::OpenaiCompatible, protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "https://api.openai.com/v1".into(), auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let language = |p: &ModelProvider, mode, locale| super::provider_transcription_language(p, "gpt-transcribe", mode, locale);
        assert_eq!(language(&provider, TranscriptionLanguageMode::Auto, Some("zh-TW")).as_deref(), Some("zh-TW"));
        assert_eq!(language(&provider, TranscriptionLanguageMode::Auto, Some("en-US")).as_deref(), Some("en-US"));
        assert_eq!(language(&provider, TranscriptionLanguageMode::Auto, Some("auto")), None);
        assert_eq!(language(&provider, TranscriptionLanguageMode::Array, Some("zh-TW")).as_deref(), Some("zh"));
        provider.base_url = "http://localhost:1234/v1".into();
        assert_eq!(language(&provider, TranscriptionLanguageMode::Auto, Some("zh-TW")).as_deref(), Some("zh"));
    }

    #[test]
    fn automatic_transcription_prefers_explicit_capabilities_and_excludes_synthesis() {
        let model = |name: &str, tags: &[&str], enabled| crate::domain::model::ModelDefinition {
            id: "fixture".into(), provider_id: "provider".into(), name: name.into(), context_window: None,
            capability_tags: tags.iter().map(|tag| (*tag).into()).collect(), notes: "".into(), enabled,
            created_at: "".into(), updated_at: "".into(),
        };
        assert_eq!(super::transcription_model_rank(&model("audio-tts", &["audio"], true)), None);
        assert_eq!(super::transcription_model_rank(&model("speech-model", &["text_to_speech"], true)), None);
        assert_eq!(super::transcription_model_rank(&model("audio-tts", &["audio", "speech_to_text"], true)), Some(0));
        assert_eq!(super::transcription_model_rank(&model("custom", &["ASR"], true)), Some(0));
        assert_eq!(super::transcription_model_rank(&model(" DeepSeek-Voice-Synthesis ", &["speech_to_text"], true)), None);
        assert_eq!(super::transcription_model_rank(&model("speech-model", &[], true)), Some(1));
        assert_eq!(super::transcription_model_rank(&model("whisper", &[], false)), None);
    }

    #[test]
    fn speech_default_model_does_not_cross_provider_boundaries() {
        let choose = |requested, configured| super::compatible_speech_model_default(requested, configured, Some("model-a".into()));
        assert_eq!(choose(None, Some("provider-a")).as_deref(), Some("model-a"));
        assert_eq!(choose(Some("provider-a"), Some("provider-a")).as_deref(), Some("model-a"));
        assert_eq!(choose(Some("provider-b"), Some("provider-a")), None);
        assert_eq!(choose(Some("provider-b"), None).as_deref(), Some("model-a"));
        assert_eq!(super::compatible_speech_model_default(None, None, Some(" ".into())), None);
    }

    #[test]
    fn speech_environment_provider_does_not_inherit_another_providers_saved_model() {
        let choose = |env_provider, saved_provider, env_model: Option<&str>, saved_model: Option<&str>|
            super::compatible_environment_speech_model(env_provider, saved_provider, env_model.map(str::to_owned), saved_model.map(str::to_owned));
        assert_eq!(choose(Some("b"), Some("a"), None, Some("saved")), None);
        assert_eq!(choose(Some("a"), Some("a"), None, Some("saved")).as_deref(), Some("saved"));
        assert_eq!(choose(None, Some("a"), None, Some("saved")).as_deref(), Some("saved"));
        assert_eq!(choose(Some("b"), None, None, Some("saved")).as_deref(), Some("saved"));
        assert_eq!(choose(Some("b"), Some("a"), Some("explicit"), Some("saved")).as_deref(), Some("explicit"));
        assert_eq!(choose(Some("b"), Some("a"), Some(""), Some("saved")).as_deref(), Some(""));
    }
    use super::*;

    #[test]
    fn transcription_audio_size_is_checked_before_and_after_decoding() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"12345678");
        assert_eq!(decode_transcription_audio(&encoded, 8).unwrap(), b"12345678");
        assert!(decode_transcription_audio(&encoded, 7).is_err()); // same base64 length, decoded limit
        assert!(decode_transcription_audio(&encoded, 3).is_err()); // encoded limit
        assert!(decode_transcription_audio("!bad", 8).is_err());
    }

    #[test]
    fn local_resampling_rejects_zero_rate_and_excessive_duration() {
        assert!(resample_mono_audio(&[0.0], 0, 16_000).is_err());
        assert!(resample_mono_audio(&[0.0; 601], 1, 16_000).is_err());
        assert_eq!(resample_mono_audio(&[0.5, -0.5], 16_000, 16_000).unwrap(), vec![0.5, -0.5]);
        assert_eq!(resample_mono_audio(&[0.5, -0.5], 8_000, 16_000).unwrap().len(), 4);
    }

    #[test]
    fn local_wav_validation_rejects_invalid_rate_and_long_audio() {
        fn wav(rate: u32, samples: usize) -> Vec<u8> {
            let mut bytes = Cursor::new(Vec::new());
            {
                let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: SampleFormat::Int };
                let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
                for _ in 0..samples { writer.write_sample(0_i16).unwrap(); }
                writer.finalize().unwrap();
            }
            bytes.into_inner()
        }
        let mut invalid = wav(8_000, 1);
        invalid[24..28].copy_from_slice(&0_u32.to_le_bytes());
        assert!(decode_wav_to_mono_f32(&invalid).is_err());
        assert!(decode_wav_to_mono_f32(&wav(1, 601)).is_err());
        assert_eq!(decode_wav_to_mono_f32(&wav(8_000, 2)).unwrap().0.len(), 2);
    }

    #[tokio::test]
    #[ignore = "requires an installed local Whisper model and a synthetic WAV fixture"]
    async fn local_whisper_transcribes_synthetic_fixture() {
        let model = std::env::var("ARUVI_LOCAL_WHISPER_MODEL").expect("set ARUVI_LOCAL_WHISPER_MODEL");
        let audio_path = std::env::var("ARUVI_LOCAL_WHISPER_AUDIO").expect("set ARUVI_LOCAL_WHISPER_AUDIO");
        let audio = std::fs::read(audio_path).expect("read WAV fixture");
        let provider = ModelProvider {
            id: "local-fixture".into(), name: "Local fixture".into(),
            provider_type: ProviderType::LocalRuntime, protocol: "openai_compatible".into(), completion_profile: "standard".into(),
            base_url: model, auth_secret_ref: None, enabled: true,
            created_at: "".into(), updated_at: "".into(),
        };
        let request = SpeechToTextRequest {
            audio_bytes_base64: base64::engine::general_purpose::STANDARD.encode(audio),
            mime_type: "audio/wav".into(), locale: Some("en".into()),
        };
        let transcript = transcribe_audio_with_provider(&provider, "whisper", request)
            .await.expect("local transcription").transcript.to_lowercase();
        assert!(transcript.contains("fox") && transcript.contains("dog"), "synthetic fixture was not recognized: {transcript}");
    }

    #[test]
    fn remote_transcription_language_uses_primary_locale_subtag() {
        for (input, expected) in [(Some("en-US"), Some("en")), (Some(" PT_br "), Some("pt")), (Some("en"), Some("en")), (Some("auto"), None), (Some(""), None), (None, None)] {
            assert_eq!(transcription_language(input).as_deref(), expected);
        }
    }

    #[tokio::test]
    async fn remote_transcription_sends_normalized_language_to_provider() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]);
                if let Some(index) = bytes.windows(4).position(|value| value == b"\r\n\r\n") { break index + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < end + length { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]); }
            let body = String::from_utf8_lossy(&bytes[end..end + length]);
            assert!(body.contains("name=\"language\"\r\n\r\nen\r\n"), "unexpected multipart language field");
            let reply = r#"{"text":"fixture transcript"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let provider = ModelProvider { id: "fixture".into(), name: "fixture".into(), provider_type: ProviderType::OpenaiCompatible,
            protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: format!("http://{address}"),
            auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let request = SpeechToTextRequest { audio_bytes_base64: base64::engine::general_purpose::STANDARD.encode(b"audio fixture"), mime_type: "audio/wav".into(), locale: Some("en-US".into()) };
        assert_eq!(transcribe_audio_with_provider(&provider, "fixture", request).await.unwrap().transcript, "fixture transcript");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn remote_audio_limit_is_checked_before_network() {
        let provider = ModelProvider { id: "fixture".into(), name: "fixture".into(), provider_type: ProviderType::OpenaiCompatible,
            protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "http://127.0.0.1:1".into(),
            auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let request = SpeechToTextRequest { audio_bytes_base64: "A".repeat(STUDIO_TRANSCRIPTION_MAX_BYTES.div_ceil(3) * 4 + 1), mime_type: "audio/wav".into(), locale: None };
        let error = transcribe_audio_with_provider(&provider, "fixture", request).await.unwrap_err();
        assert!(matches!(error, AppError::Validation(message) if message.contains("32 MiB")));
    }

    #[tokio::test]
    async fn deepseek_audio_limit_applies_before_secret_or_network_access() {
        let provider = ModelProvider { id: "fixture".into(), name: "fixture".into(), provider_type: ProviderType::OpenaiCompatible,
            protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "http://127.0.0.1:1".into(),
            auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let encoded = base64::engine::general_purpose::STANDARD.encode(vec![0; DEEPSEEK_TRANSCRIPTION_MAX_BYTES + 1]);
        let request = SpeechToTextRequest { audio_bytes_base64: encoded, mime_type: "audio/wav".into(), locale: None };
        let error = transcribe_audio_with_provider(&provider, " DeepSeek-Voice-Transcription ", request).await.unwrap_err();
        assert!(matches!(error, AppError::Validation(message) if message.contains("8 MiB")));
    }

    #[tokio::test]
    async fn disabled_transcription_providers_are_rejected_before_io() {
        for provider_type in [ProviderType::OpenaiCompatible, ProviderType::LocalRuntime] {
            let provider = ModelProvider {
                id: "disabled".into(), name: "Disabled".into(), provider_type,
                protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "invalid".into(),
                auth_secret_ref: None, enabled: false, created_at: "".into(), updated_at: "".into(),
            };
            let request = SpeechToTextRequest {
                audio_bytes_base64: "invalid base64".into(), mime_type: "audio/wav".into(), locale: None,
            };
            let error = transcribe_audio_with_provider(&provider, "whisper", request).await.unwrap_err();
            assert!(matches!(error, AppError::Validation(ref message) if message == "Selected provider is disabled"));
        }
    }

    #[tokio::test]
    async fn opposite_deepseek_voice_selectors_are_rejected_before_io() {
        for provider_type in [ProviderType::OpenaiCompatible, ProviderType::LocalRuntime] {
            let provider = ModelProvider {
                id: "fixture".into(), name: "Fixture".into(), provider_type,
                protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: "invalid".into(),
                auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into(),
            };
            let error = synthesize_remote_speech(&provider, " DeepSeek-Voice-Transcription ".into(), "hello".into(), None, None).await.unwrap_err();
            assert!(matches!(error, AppError::Validation(message) if message.contains("cannot generate speech")));
            let request = SpeechToTextRequest { audio_bytes_base64: "invalid base64".into(), mime_type: "audio/wav".into(), locale: None };
            let error = transcribe_audio_with_provider(&provider, " DeepSeek-Voice-Synthesis ", request).await.unwrap_err();
            assert!(matches!(error, AppError::Validation(message) if message.contains("cannot transcribe audio")));
        }
    }
}

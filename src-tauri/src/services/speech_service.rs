use crate::domain::model::{ModelProvider, ProviderType};
use crate::error::AppError;
use crate::secrets;
use async_trait::async_trait;
use base64::Engine;
use hound::{SampleFormat, WavReader};
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechResponse {
    pub audio_bytes_base64: String,
    pub mime_type: String,
}

#[async_trait]
pub trait SpeechService: Send + Sync {
    async fn speech_to_text(
        &self,
        request: SpeechToTextRequest,
    ) -> Result<SpeechToTextResponse, AppError>;
    async fn text_to_speech(
        &self,
        request: TextToSpeechRequest,
    ) -> Result<TextToSpeechResponse, AppError>;
}

pub struct StubSpeechService;

#[async_trait]
impl SpeechService for StubSpeechService {
    async fn speech_to_text(
        &self,
        _request: SpeechToTextRequest,
    ) -> Result<SpeechToTextResponse, AppError> {
        Err(AppError::Internal(
            "Speech-to-text service is scaffolded but not implemented yet".to_string(),
        ))
    }

    async fn text_to_speech(
        &self,
        _request: TextToSpeechRequest,
    ) -> Result<TextToSpeechResponse, AppError> {
        Err(AppError::Internal(
            "Text-to-speech service is scaffolded but not implemented yet".to_string(),
        ))
    }
}

pub async fn transcribe_audio_with_provider(
    provider: &ModelProvider,
    model_name: &str,
    request: SpeechToTextRequest,
) -> Result<SpeechToTextResponse, AppError> {
    if matches!(provider.provider_type, ProviderType::LocalRuntime) {
        return transcribe_audio_locally(provider, request).await;
    }

    let api_key = secrets::resolve_provider_secret(provider)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| Client::new());

    let trimmed = provider.base_url.trim_end_matches('/');
    let endpoint = if trimmed.ends_with("/v1") {
        format!("{trimmed}/audio/transcriptions")
    } else {
        format!("{trimmed}/v1/audio/transcriptions")
    };

    let file_bytes = base64::engine::general_purpose::STANDARD
        .decode(request.audio_bytes_base64.trim())
        .map_err(|error| AppError::Validation(format!("Invalid audio payload: {error}")))?;
    let file_part = multipart::Part::bytes(file_bytes)
        .file_name(format!(
            "planner-recording.{}",
            mime_extension_for_mime_type(&request.mime_type)
        ))
        .mime_str(&request.mime_type)
        .map_err(|error| AppError::Validation(format!("Unsupported audio mime type: {error}")))?;

    let mut form = multipart::Form::new()
        .text("model", model_name.to_string())
        .part("file", file_part);
    if let Some(locale) = request.locale.filter(|value| !value.trim().is_empty()) {
        form = form.text("language", locale);
    }

    let mut req = client.post(endpoint).multipart(form);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let response = req
        .send()
        .await
        .map_err(|error| AppError::Provider(format!("Speech transcription request failed: {error}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE
            && text.contains("application/json")
        {
            return Err(AppError::Validation(format!(
                "Provider '{}' with model '{}' does not appear to support OpenAI-compatible multipart audio transcription uploads. Configure a speech-capable provider/model, such as a Whisper or STT endpoint, in Settings.",
                provider.name, model_name
            )));
        }
        return Err(AppError::Provider(format!(
            "Speech transcription API error {status}: {text}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| AppError::Provider(format!("Failed to parse transcription response: {error}")))?;
    let transcript = json
        .get("text")
        .and_then(serde_json::Value::as_str)
        .or_else(|| json.get("transcript").and_then(serde_json::Value::as_str))
        .map(str::trim)
        .unwrap_or_default()
        .to_string();

    if transcript.is_empty() {
        return Err(AppError::Provider(
            "Speech transcription returned an empty transcript".to_string(),
        ));
    }

    Ok(SpeechToTextResponse { transcript })
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
    normalized.contains("whisper")
        || normalized.contains("transcrib")
        || normalized.contains("speech")
        || normalized.contains("stt")
        || normalized.contains("audio")
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

    if !resolved.exists() {
        return Err(AppError::Validation(format!(
            "Local runtime model file not found: {}",
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
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(request.audio_bytes_base64.trim())
        .map_err(|error| AppError::Validation(format!("Invalid audio payload: {error}")))?;
    let (samples, sample_rate) = decode_wav_to_mono_f32(&audio_bytes)?;
    let resampled = resample_mono_audio(&samples, sample_rate, 16_000);
    let language = normalize_whisper_language(request.locale.as_deref());

    tokio::task::spawn_blocking(move || {
        let transcript = run_whisper_transcription(&model_path, &resampled, language.as_deref())?;
        Ok(SpeechToTextResponse { transcript })
    })
    .await
    .map_err(|error| AppError::Internal(format!("Local Whisper task failed: {error}")))?
}

fn decode_wav_to_mono_f32(bytes: &[u8]) -> Result<(Vec<f32>, u32), AppError> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut reader = WavReader::new(cursor)
        .map_err(|error| AppError::Validation(format!("Invalid WAV audio: {error}")))?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(AppError::Validation(
            "WAV audio must have at least one channel.".to_string(),
        ));
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

fn resample_mono_audio(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if source_rate == target_rate || samples.is_empty() {
        return samples.to_vec();
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

    output
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

    state
        .full(params, samples)
        .map_err(|error| AppError::Internal(format!("Local Whisper transcription failed: {error}")))?;

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

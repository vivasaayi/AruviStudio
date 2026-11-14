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

const LOCAL_WHISPER_MAX_BYTES: usize = 32 * 1024 * 1024;
const LOCAL_WHISPER_MAX_DURATION_SECONDS: u64 = 10 * 60;

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
    if provider.protocol != "openai_compatible" {
        return Err(AppError::Validation("Selected provider protocol does not support remote speech synthesis".into()));
    }
    let client = crate::services::model_service::create_shared_gateway(provider)?;
    client.generate_speech(aruvi_model_client::SpeechRequest { model, input, voice, speed, response_format: "wav".into() }).await
        .map_err(|error| match error {
            aruvi_model_client::ClientError::InvalidInput(message) => AppError::Validation(message),
            other => AppError::Provider(other.to_string()),
        })
}

pub async fn transcribe_audio_with_provider(
    provider: &ModelProvider,
    model_name: &str,
    request: SpeechToTextRequest,
) -> Result<SpeechToTextResponse, AppError> {
    if !provider.enabled {
        return Err(AppError::Validation("Selected provider is disabled".into()));
    }
    if matches!(provider.provider_type, ProviderType::LocalRuntime) {
        return transcribe_audio_locally(provider, request).await;
    }

    if provider.protocol != "openai_compatible" {
        return Err(AppError::Validation("Selected provider protocol does not support remote transcription".into()));
    }
    let file_bytes = base64::engine::general_purpose::STANDARD
        .decode(request.audio_bytes_base64.trim())
        .map_err(|error| AppError::Validation(format!("Invalid audio payload: {error}")))?;
    let client = crate::services::model_service::create_shared_gateway(provider)?;
    let result = client.transcribe(aruvi_model_client::TranscriptionRequest {
        model: model_name.to_string(),
        audio: file_bytes,
        filename: format!("planner-recording.{}", mime_extension_for_mime_type(&request.mime_type)),
        mime_type: request.mime_type,
        language: request.locale,
    }).await;
    match result {
        Ok(transcript) => Ok(SpeechToTextResponse { transcript }),
        Err(aruvi_model_client::ClientError::UnsupportedTranscription) => Err(AppError::Validation(format!(
            "Provider '{}' with model '{}' does not appear to support OpenAI-compatible multipart audio transcription uploads. Configure a speech-capable provider/model, such as a Whisper or STT endpoint, in Settings.",
            provider.name, model_name
        ))),
        Err(aruvi_model_client::ClientError::InvalidInput(message)) => Err(AppError::Validation(message)),
        Err(error) => Err(AppError::Provider(error.to_string())),
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
    let audio_bytes = decode_local_audio(&request.audio_bytes_base64, LOCAL_WHISPER_MAX_BYTES)?;
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

fn decode_local_audio(encoded: &str, max_bytes: usize) -> Result<Vec<u8>, AppError> {
    let encoded = encoded.trim();
    // Check the encoded length before decoding so an oversized request does not
    // allocate a second, decoded copy of the entire payload.
    let max_encoded = max_bytes.div_ceil(3).saturating_mul(4);
    if encoded.len() > max_encoded {
        return Err(AppError::Validation("Local Whisper audio exceeds the 32 MiB limit.".into()));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)
        .map_err(|error| AppError::Validation(format!("Invalid audio payload: {error}")))?;
    if bytes.len() > max_bytes {
        return Err(AppError::Validation("Local Whisper audio exceeds the 32 MiB limit.".into()));
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
    use super::*;

    #[test]
    fn local_audio_size_is_checked_before_and_after_decoding() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"12345678");
        assert_eq!(decode_local_audio(&encoded, 8).unwrap(), b"12345678");
        assert!(decode_local_audio(&encoded, 7).is_err()); // same base64 length, decoded limit
        assert!(decode_local_audio(&encoded, 3).is_err()); // encoded limit
        assert!(decode_local_audio("!bad", 8).is_err());
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
}

use crate::domain::model::ModelProvider;
use crate::error::AppError;
use crate::secrets;
use async_trait::async_trait;
use base64::Engine;
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

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

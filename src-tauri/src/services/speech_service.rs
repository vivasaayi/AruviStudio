use crate::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

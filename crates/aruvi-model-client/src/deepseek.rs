//! DeepSeek's standalone, non-streaming-request voice APIs.
//!
//! These endpoints differ from OpenAI-compatible `/audio` APIs and do not
//! accept a model or language parameter. Applications select this client explicitly.
use crate::{ClientError, ClientOptions, CompletionRequest, CompletionResponse, ModelGateway, ModelPage, SpeechRequest, TranscriptionRequest, TransportCapabilities};
use async_trait::async_trait;
use reqwest::{header::HeaderValue, Client, Url};
use serde::Serialize;

const MAX_UPLOAD: usize = 8 * 1024 * 1024;
const MAX_AUDIO_RESPONSE: usize = 16 * 1024 * 1024;

pub struct DeepSeekVoiceClient {
    base_url: String,
    api_key: HeaderValue,
    client: Client,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, thread};

    fn fixture(path: &'static str, reply: Vec<u8>, status: &'static str, inspect: fn(&[u8])) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]);
                if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
            assert!(headers.starts_with(&format!("post {path} ")));
            assert!(headers.contains("authorization: bearer synthetic-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < end + length { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]); }
            inspect(&bytes[end..end + length]);
            write!(socket, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", reply.len()).unwrap();
            socket.write_all(&reply).unwrap();
        });
        (format!("http://{address}"), handle)
    }

    fn wav() -> Vec<u8> {
        let mut result = b"RIFF".to_vec(); result.extend_from_slice(&38u32.to_le_bytes()); result.extend_from_slice(b"WAVEfmt ");
        result.extend_from_slice(&16u32.to_le_bytes()); result.extend_from_slice(&[1,0,1,0,0x80,0x3e,0,0,0,0x7d,0,0,2,0,16,0]);
        result.extend_from_slice(b"data"); result.extend_from_slice(&2u32.to_le_bytes()); result.extend_from_slice(&[0,0]); result
    }

    #[tokio::test]
    async fn uses_deepseek_voice_contract_and_preserves_status() {
        let (url, server) = fixture("/v1/tools/asr", br#"{"text":"hello","duration_ms":100}"#.to_vec(), "200 OK", |body| {
            let text = String::from_utf8_lossy(body);
            assert!(text.contains("name=\"file\"")); assert!(text.contains("audio fixture"));
            assert!(!text.contains("name=\"model\"")); assert!(!text.contains("name=\"language\""));
        });
        let client = DeepSeekVoiceClient::new(url, "synthetic-key".into()).unwrap();
        assert_eq!(client.transcribe(b"audio fixture".to_vec(), "sample.wav".into(), "audio/wav".into()).await.unwrap(), "hello");
        server.join().unwrap();

        let expected = wav();
        let (url, server) = fixture("/v1/tools/tts", expected.clone(), "200 OK", |body| {
            let json: serde_json::Value = serde_json::from_slice(body).unwrap();
            assert_eq!(json, serde_json::json!({"text":"hello", "voice_id":"tide", "format":"wav"}));
        });
        let client = DeepSeekVoiceClient::new(format!("{url}/v1"), "synthetic-key".into()).unwrap();
        assert_eq!(client.generate_speech("hello", Some(Voice::Tide)).await.unwrap(), expected);
        server.join().unwrap();

        let (url, server) = fixture("/v1/tools/tts", b"private provider body".to_vec(), "429 Too Many Requests", |_| {});
        let client = DeepSeekVoiceClient::new(url, "synthetic-key".into()).unwrap();
        let error = client.generate_speech("hello", None).await.unwrap_err();
        assert_eq!(error.http_status(), Some(429)); assert!(!error.to_string().contains("private provider body"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn validates_voice_inputs_before_connecting() {
        let client = DeepSeekVoiceClient::new("http://127.0.0.1:1".into(), "synthetic-key".into()).unwrap();
        assert!(matches!(client.transcribe(vec![], "a.wav".into(), "audio/wav".into()).await, Err(ClientError::InvalidInput(_))));
        assert!(matches!(client.transcribe(vec![0; MAX_UPLOAD + 1], "a.wav".into(), "audio/wav".into()).await, Err(ClientError::InvalidInput(_))));
        assert!(matches!(client.generate_speech(" ", None).await, Err(ClientError::InvalidInput(_))));
        assert!(matches!(client.generate_speech(&"x".repeat(10_001), None).await, Err(ClientError::InvalidInput(_))));
        assert!(DeepSeekVoiceClient::new("http://user:secret@localhost".into(), "key".into()).is_err());
        assert!(DeepSeekVoiceClient::new("http://localhost".into(), "key\ninjected".into()).is_err());
    }

    #[tokio::test]
    async fn voice_gateway_supports_common_interface_without_inventing_capabilities() {
        let (url, server) = fixture("/v1/tools/asr", br#"{"text":"hello"}"#.to_vec(), "200 OK", |body| {
            let text = String::from_utf8_lossy(body);
            assert!(text.contains("name=\"file\""));
            assert!(!text.contains("name=\"model\""));
            assert!(!text.contains("name=\"language\""));
        });
        let client: Box<dyn ModelGateway> = Box::new(DeepSeekVoiceClient::new(url, "synthetic-key".into()).unwrap());
        assert_eq!(client.transport_capabilities(), TransportCapabilities { transcription: true, speech: true, ..Default::default() });
        assert!(matches!(client.health_check().await, Err(ClientError::UnsupportedOperation(_))));
        assert!(matches!(client.list_models_page(None).await, Err(ClientError::UnsupportedOperation(_))));
        assert!(matches!(client.run_completion(CompletionRequest { model: "selector".into(), messages: vec![], temperature: None, max_tokens: None }).await, Err(ClientError::UnsupportedOperation(_))));
        assert_eq!(client.transcribe(TranscriptionRequest { model: "selector".into(), audio: b"audio".to_vec(), filename: "clip.wav".into(), mime_type: "audio/wav".into(), language: None }).await.unwrap(), "hello");
        server.join().unwrap();

        let expected = wav();
        let (url, server) = fixture("/v1/tools/tts", expected.clone(), "200 OK", |body| {
            assert_eq!(serde_json::from_slice::<serde_json::Value>(body).unwrap(), serde_json::json!({"text":"hello","voice_id":"tide","format":"wav"}));
        });
        let client: Box<dyn ModelGateway> = Box::new(DeepSeekVoiceClient::new(url, "synthetic-key".into()).unwrap());
        let request = SpeechRequest { model: "selector".into(), input: "hello".into(), voice: Some("tide".into()), speed: None, response_format: "wav".into() };
        assert_eq!(client.generate_speech(request.clone()).await.unwrap(), expected);
        server.join().unwrap();
        assert!(matches!(client.generate_speech(SpeechRequest { speed: Some(1.0), ..request.clone() }).await, Err(ClientError::InvalidInput(_))));
        assert!(matches!(client.generate_speech(SpeechRequest { response_format: "mp3".into(), ..request }).await, Err(ClientError::InvalidInput(_))));
        assert!(matches!(client.transcribe(TranscriptionRequest { model: "selector".into(), audio: vec![1], filename: "clip.wav".into(), mime_type: "audio/wav".into(), language: Some("en".into()) }).await, Err(ClientError::InvalidInput(_))));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice { Stella, Tide }

impl Voice {
    fn id(self) -> &'static str { match self { Self::Stella => "stella", Self::Tide => "tide" } }
}

impl DeepSeekVoiceClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self, ClientError> {
        Self::with_options(base_url, api_key, ClientOptions::default())
    }

    pub fn with_options(base_url: String, api_key: String, options: ClientOptions) -> Result<Self, ClientError> {
        let url = Url::parse(&base_url).map_err(|_| ClientError::InvalidInput("Invalid DeepSeek base URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err(ClientError::InvalidInput("Invalid DeepSeek base URL".into()));
        }
        if api_key.trim().is_empty() { return Err(ClientError::InvalidInput("DeepSeek API key is required".into())); }
        let mut api_key = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|_| ClientError::InvalidInput("Invalid DeepSeek API key".into()))?;
        api_key.set_sensitive(true);
        Ok(Self { base_url, api_key, client: crate::http::build_client(options)? })
    }

    fn endpoint(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") { format!("{base}/tools/{path}") } else { format!("{base}/v1/tools/{path}") }
    }

    /// Transcribe one complete WAV or Ogg Opus upload. The provider enforces
    /// codec, sample rate and duration; no locale hint is sent.
    pub async fn transcribe(&self, audio: Vec<u8>, filename: String, mime_type: String) -> Result<String, ClientError> {
        if audio.is_empty() || audio.len() > MAX_UPLOAD {
            return Err(ClientError::InvalidInput("DeepSeek audio must be between 1 byte and 8 MiB".into()));
        }
        let file = reqwest::multipart::Part::bytes(audio).file_name(filename).mime_str(&mime_type)
            .map_err(|_| ClientError::InvalidInput("Invalid audio MIME type".into()))?;
        let response = self.client.post(self.endpoint("asr")).header(reqwest::header::AUTHORIZATION, &self.api_key)
            .multipart(reqwest::multipart::Form::new().part("file", file)).send().await
            .map_err(|error| crate::http::transport_error("DeepSeek transcription request failed", error))?;
        if !response.status().is_success() { return Err(ClientError::http("DeepSeek transcription API error", response.status())); }
        let body = crate::http::read_bounded(response, 2 * 1024 * 1024, "DeepSeek transcription response").await?;
        let json: serde_json::Value = serde_json::from_slice(&body).map_err(|_| ClientError::Provider("Invalid DeepSeek transcription response".into()))?;
        if json.get("error").is_some_and(|value| !value.is_null()) { return Err(ClientError::Provider("DeepSeek transcription returned an error".into())); }
        json.get("text").and_then(serde_json::Value::as_str).map(str::trim).filter(|text| !text.is_empty())
            .map(str::to_owned).ok_or_else(|| ClientError::Provider("DeepSeek transcription returned an empty transcript".into()))
    }

    /// Request buffered WAV output. Unknown-length WAV headers cannot prove
    /// that a provider did not terminate synthesis early.
    pub async fn generate_speech(&self, text: &str, voice: Option<Voice>) -> Result<Vec<u8>, ClientError> {
        if text.trim().is_empty() || text.chars().count() > 10_000 {
            return Err(ClientError::InvalidInput("DeepSeek speech requires 1–10000 characters".into()));
        }
        #[derive(Serialize)]
        struct Request<'a> { text: &'a str, format: &'static str, #[serde(skip_serializing_if = "Option::is_none")] voice_id: Option<&'static str> }
        let body = Request { text, format: "wav", voice_id: voice.map(Voice::id) };
        let body = serde_json::to_vec(&body).map_err(|_| ClientError::InvalidInput("Invalid DeepSeek speech request".into()))?;
        if body.len() > 64 * 1024 { return Err(ClientError::InvalidInput("DeepSeek speech request exceeds 64 KiB".into())); }
        let response = self.client.post(self.endpoint("tts")).header(reqwest::header::AUTHORIZATION, &self.api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json").body(body).send().await
            .map_err(|error| crate::http::transport_error("DeepSeek speech request failed", error))?;
        if !response.status().is_success() { return Err(ClientError::http("DeepSeek speech API error", response.status())); }
        let audio = crate::http::read_bounded(response, MAX_AUDIO_RESPONSE, "DeepSeek speech response").await?;
        if !crate::openai_compatible::has_wav_audio(&audio) { return Err(ClientError::Provider("DeepSeek did not return valid WAV audio".into())); }
        Ok(audio)
    }
}

#[async_trait]
impl ModelGateway for DeepSeekVoiceClient {
    fn transport_capabilities(&self) -> TransportCapabilities {
        TransportCapabilities { transcription: true, speech: true, ..TransportCapabilities::default() }
    }

    async fn health_check(&self) -> Result<bool, ClientError> {
        Err(ClientError::UnsupportedOperation("DeepSeek voice health check"))
    }

    async fn list_models_page(&self, _cursor: Option<&str>) -> Result<ModelPage, ClientError> {
        Err(ClientError::UnsupportedOperation("DeepSeek voice model discovery"))
    }

    async fn run_completion(&self, _request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
        Err(ClientError::UnsupportedOperation("DeepSeek voice text completion"))
    }

    async fn transcribe(&self, request: TranscriptionRequest) -> Result<String, ClientError> {
        if request.language.as_deref().is_some_and(|language| !language.trim().is_empty()) {
            return Err(ClientError::InvalidInput("DeepSeek voice detects language and does not accept a language hint".into()));
        }
        // The DeepSeek voice API does not accept a model identifier.
        DeepSeekVoiceClient::transcribe(self, request.audio, request.filename, request.mime_type).await
    }

    async fn generate_speech(&self, request: SpeechRequest) -> Result<Vec<u8>, ClientError> {
        if request.speed.is_some() { return Err(ClientError::InvalidInput("DeepSeek voice does not support speed control".into())); }
        if !request.response_format.eq_ignore_ascii_case("wav") { return Err(ClientError::InvalidInput("DeepSeek voice only supports WAV output".into())); }
        let voice = match request.voice.as_deref().map(str::trim).filter(|voice| !voice.is_empty()) {
            None => None,
            Some("stella") => Some(Voice::Stella),
            Some("tide") => Some(Voice::Tide),
            Some(_) => return Err(ClientError::InvalidInput("DeepSeek voice must be stella or tide".into())),
        };
        // The DeepSeek voice API does not accept a model identifier.
        DeepSeekVoiceClient::generate_speech(self, &request.input, voice).await
    }
}

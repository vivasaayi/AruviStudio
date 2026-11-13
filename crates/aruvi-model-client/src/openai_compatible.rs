use crate::ClientError;
use crate::ModelGateway;
use crate::{CompletionRequest, CompletionResponse, SpeechRequest, TranscriptionRequest};
use async_trait::async_trait;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;

pub struct OpenAiCompatibleClient {
    base_url: String,
    api_key: Option<String>,
    client: Option<Client>,
}

impl OpenAiCompatibleClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .redirect(Policy::none())
            .build()
            .ok();
        Self {
            base_url,
            api_key,
            client,
        }
    }

    fn client(&self) -> Result<&Client, ClientError> {
        self.client.as_ref().ok_or_else(|| ClientError::Provider("Unable to initialize model HTTP client".into()))
    }

    fn endpoint_url(&self, path: &str) -> Result<String, ClientError> {
        let url = reqwest::Url::parse(&self.base_url).map_err(|_| ClientError::InvalidInput("Invalid model provider base URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err(ClientError::InvalidInput("Invalid model provider base URL".into()));
        }
        let trimmed = self.base_url.trim_end_matches('/');
        Ok(if trimmed.ends_with("/v1") { format!("{trimmed}{path}") } else { format!("{trimmed}/v1{path}") })
    }

    fn extract_choice_content(choice: &serde_json::Value) -> Option<String> {
        if let Some(content) = choice
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
        {
            return Some(content.to_string());
        }

        if let Some(content_parts) = choice
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_array())
        {
            let mut combined = String::new();
            for part in content_parts {
                if let Some(text) = part.get("text").and_then(|value| value.as_str()) {
                    combined.push_str(text);
                } else if let Some(text) = part.get("content").and_then(|value| value.as_str()) {
                    combined.push_str(text);
                }
            }
            if !combined.is_empty() {
                return Some(combined);
            }
        }

        choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(|content| content.as_str())
            .map(|content| content.to_string())
    }

    fn parse_sse_fallback(raw_body: &str) -> Result<CompletionResponse, ClientError> {
        let mut result = CompletionResponse { content: String::new(), token_count_input: None, token_count_output: None };
        let mut data = Vec::new();
        let mut done = false;
        // A final newline is accepted for compatibility with providers that omit
        // the last blank SSE separator, but a terminal [DONE] is still required.
        for line in raw_body.lines().chain(std::iter::once("")) {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                if data.is_empty() { continue; }
                let payload = data.join("\n");
                data.clear();
                if payload.trim() == "[DONE]" { done = true; break; }
                let event: serde_json::Value = serde_json::from_str(&payload)
                    .map_err(|_| ClientError::Provider("Invalid model stream event".into()))?;
                if event.get("error").is_some_and(|value| !value.is_null()) {
                    return Err(ClientError::Provider("Model stream returned an error".into()));
                }
                if let Some(choice) = event.get("choices").and_then(|v| v.as_array()).and_then(|v| v.first()) {
                    Self::reject_non_text_choice(choice, "delta")?;
                    if let Some(content) = Self::extract_choice_content(choice) { result.content.push_str(&content); }
                }
                if let Some(usage) = event.get("usage") {
                    result.token_count_input = usage.get("prompt_tokens").and_then(|v| v.as_i64()).or(result.token_count_input);
                    result.token_count_output = usage.get("completion_tokens").and_then(|v| v.as_i64()).or(result.token_count_output);
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                data.push(value.strip_prefix(' ').unwrap_or(value));
            }
        }
        if !done { return Err(ClientError::Provider("Model stream ended before its terminal event".into())); }
        if result.content.is_empty() { return Err(ClientError::Provider("Model stream returned no text".into())); }
        Ok(result)
    }

    fn reject_non_text_choice(choice: &serde_json::Value, field: &str) -> Result<(), ClientError> {
        let message = choice.get(field);
        if message.and_then(|value| value.get("tool_calls")).is_some_and(|value| value.as_array().map_or(!value.is_null(), |calls| !calls.is_empty()))
            || message.and_then(|value| value.get("function_call")).is_some_and(|value| !value.is_null())
            || choice.get("finish_reason").and_then(|value| value.as_str()).is_some_and(|value| value == "tool_calls" || value == "function_call") {
            return Err(ClientError::Provider("Model tool-use responses are not supported by the text client".into()));
        }
        if message.and_then(|value| value.get("refusal")).is_some_and(|value| !value.is_null()) {
            return Err(ClientError::Provider("Model refused the request".into()));
        }
        Ok(())
    }

    /// Stream one OpenAI-compatible chat response. Dropping the future cancels
    /// the HTTP request. Usage is returned when the server includes it.
    pub async fn stream_completion<F>(&self, request: CompletionRequest, mut on_delta: F) -> Result<CompletionResponse, ClientError>
    where F: FnMut(&str) + Send {
        if request.model.trim().is_empty() || request.messages.is_empty() || request.max_tokens.is_some_and(|value| value <= 0) {
            return Err(ClientError::InvalidInput("A model, messages and positive max_tokens are required".into()));
        }
        let body = serde_json::json!({
            "model": request.model, "messages": request.messages,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096), "stream": true,
        });
        let mut http = self.client()?.post(self.endpoint_url("/chat/completions")?).json(&body);
        if let Some(key) = &self.api_key { http = http.bearer_auth(key); }
        let response = http.send().await.map_err(|error| ClientError::Provider(format!("Model stream request failed: {error}")))?;
        if !response.status().is_success() {
            return Err(ClientError::Provider(format!("Model stream API error {}", response.status())));
        }
        let mut result = CompletionResponse { content: String::new(), token_count_input: None, token_count_output: None };
        crate::sse::read_sse(response, "Model", |data| {
            if data == b"[DONE]" { return Ok(true); }
            let event: serde_json::Value = serde_json::from_slice(data)
                .map_err(|_| ClientError::Provider("Invalid model stream event".into()))?;
            if event.get("error").is_some_and(|value| !value.is_null()) {
                return Err(ClientError::Provider("Model stream returned an error".into()));
            }
            if let Some(usage) = event.get("usage") {
                result.token_count_input = usage.get("prompt_tokens").and_then(|value| value.as_i64()).or(result.token_count_input);
                result.token_count_output = usage.get("completion_tokens").and_then(|value| value.as_i64()).or(result.token_count_output);
            }
            if let Some(choice) = event.get("choices").and_then(|value| value.as_array()).and_then(|values| values.first()) {
                Self::reject_non_text_choice(choice, "delta")?;
                if let Some(content) = choice.pointer("/delta/content").and_then(|value| value.as_str()) {
                    if !content.is_empty() { result.content.push_str(content); on_delta(content); }
                }
            }
            Ok(false)
        }).await?;
        if result.content.is_empty() { return Err(ClientError::Provider("Model stream returned no text".into())); }
        Ok(result)
    }

    /// Transcribe audio using an OpenAI-compatible multipart endpoint.
    pub async fn transcribe(&self, input: TranscriptionRequest) -> Result<String, ClientError> {
        let file = reqwest::multipart::Part::bytes(input.audio)
            .file_name(input.filename)
            .mime_str(&input.mime_type)
            .map_err(|error| ClientError::InvalidInput(format!("Unsupported audio mime type: {error}")))?;
        let mut form = reqwest::multipart::Form::new().text("model", input.model).part("file", file);
        if let Some(language) = input.language.filter(|value| !value.trim().is_empty()) {
            form = form.text("language", language);
        }
        let mut request = self.client()?.post(self.endpoint_url("/audio/transcriptions")?).multipart(form);
        if let Some(key) = &self.api_key { request = request.bearer_auth(key); }
        let response = request.send().await.map_err(|error| ClientError::Provider(format!("Speech transcription request failed: {error}")))?;
        if !response.status().is_success() {
            let status = response.status();
            if status == reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE {
                if let Ok(body) = crate::http::read_bounded(response, 8 * 1024, "Transcription error response").await {
                    if String::from_utf8_lossy(&body).contains("application/json") { return Err(ClientError::UnsupportedTranscription); }
                }
            }
            return Err(ClientError::Provider(format!("Speech transcription API error {status}")));
        }
        let bytes = crate::http::read_bounded(response, 2 * 1024 * 1024, "Transcription response").await?;
        let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ClientError::Provider("Invalid transcription JSON response".into()))?;
        let transcript = json.get("text").and_then(serde_json::Value::as_str).or_else(|| json.get("transcript").and_then(serde_json::Value::as_str)).map(str::trim).unwrap_or_default().to_string();
        if transcript.is_empty() { return Err(ClientError::Provider("Speech transcription returned an empty transcript".to_string())); }
        Ok(transcript)
    }

    /// Generate WAV audio with a bounded response size.
    pub async fn generate_speech(&self, input: SpeechRequest) -> Result<Vec<u8>, ClientError> {
        const MAX_AUDIO_BYTES: usize = 16 * 1024 * 1024;
        if input.model.trim().is_empty() || input.input.trim().is_empty() || input.input.chars().count() > 4096 {
            return Err(ClientError::InvalidInput("A model and 1–4096 characters of speech text are required".into()));
        }
        if input.response_format != "wav" || input.speed.is_some_and(|speed| !speed.is_finite() || !(0.25..=4.0).contains(&speed)) {
            return Err(ClientError::InvalidInput("Speech requires WAV output and speed between 0.25 and 4.0".into()));
        }
        let mut request = self.client()?.post(self.endpoint_url("/audio/speech")?).json(&input);
        if let Some(key) = &self.api_key { request = request.bearer_auth(key); }
        let mut response = request.send().await.map_err(|error| ClientError::Provider(format!("Speech request failed: {error}")))?;
        if !response.status().is_success() {
            return Err(ClientError::Provider(format!("Speech API error {}", response.status())));
        }
        if response.content_length().is_some_and(|length| length > MAX_AUDIO_BYTES as u64) {
            return Err(ClientError::Provider("Speech audio exceeds the size limit".into()));
        }
        let mut audio = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| ClientError::Provider(format!("Speech response failed: {error}")))? {
            if chunk.len() > MAX_AUDIO_BYTES - audio.len() { return Err(ClientError::Provider("Speech audio exceeds the size limit".into())); }
            audio.extend_from_slice(&chunk);
        }
        if audio.len() < 12 || &audio[..4] != b"RIFF" || &audio[8..12] != b"WAVE" {
            return Err(ClientError::Provider("Speech provider did not return WAV audio".into()));
        }
        Ok(audio)
    }
}

#[async_trait]
impl ModelGateway for OpenAiCompatibleClient {
    async fn health_check(&self) -> Result<bool, ClientError> {
        let url = self.endpoint_url("/models")?;
        let mut req = self.client()?.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }
        match req.send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(ClientError::Provider(format!("Health check failed: {e}"))),
        }
    }

    async fn list_models_page(&self, cursor: Option<&str>) -> Result<crate::ModelPage, ClientError> {
        if cursor.is_some() { return Err(ClientError::InvalidInput("OpenAI-compatible discovery does not support cursors".into())); }
        let mut request = self.client()?.get(self.endpoint_url("/models")?);
        if let Some(key) = &self.api_key { request = request.bearer_auth(key); }
        let response = request.send().await.map_err(|error| ClientError::Provider(format!("Model discovery request failed: {error}")))?;
        let body = crate::discovery::read_response(response).await?;
        Ok(crate::ModelPage { models: crate::discovery::parse_models(&body)?, next_cursor: None })
    }

    async fn run_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ClientError> {
        let url = self.endpoint_url("/chat/completions")?;
        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        let mut req = self.client()?.post(&url).json(&body);
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ClientError::Provider(format!("Request failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(ClientError::Provider(format!("API error {}", resp.status())));
        }

        let bytes = crate::http::read_bounded(resp, 8 * 1024 * 1024, "Model completion response").await?;
        let raw_body = std::str::from_utf8(&bytes).map_err(|_| ClientError::Provider("Invalid UTF-8 in model completion response".into()))?;
        let json: serde_json::Value = match serde_json::from_str(raw_body) {
            Ok(value) => value,
            Err(_) => {
                if raw_body.lines().any(|line| line.starts_with("data:")) { return Self::parse_sse_fallback(raw_body); }
                return Err(ClientError::Provider("Invalid model completion response".into()));
            }
        };
        if json.get("error").is_some_and(|value| !value.is_null()) {
            return Err(ClientError::Provider("Model completion returned an error".into()));
        }
        let choice = json.get("choices").and_then(|value| value.as_array()).and_then(|values| values.first())
            .ok_or_else(|| ClientError::Provider("Model completion returned no choices".into()))?;
        Self::reject_non_text_choice(choice, "message")?;
        let content = Self::extract_choice_content(choice)
            .ok_or_else(|| ClientError::Provider("Model completion returned no text".into()))?;

        Ok(CompletionResponse {
            content,
            token_count_input: json
                .get("usage")
                .and_then(|usage| usage.get("prompt_tokens"))
                .and_then(|value| value.as_i64()),
            token_count_output: json
                .get("usage")
                .and_then(|usage| usage.get("completion_tokens"))
                .and_then(|value| value.as_i64()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[tokio::test]
    async fn posts_authenticated_completion_and_extracts_usage() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            let header_end = loop {
                let n = socket.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&buffer[..n]);
                if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&data[..header_end]).to_lowercase();
            assert!(headers.starts_with("post /v1/chat/completions "));
            assert!(headers.contains("authorization: bearer test-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < header_end + length {
                let n = socket.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&buffer[..n]);
            }
            let body: serde_json::Value = serde_json::from_slice(&data[header_end..header_end + length]).unwrap();
            assert_eq!(body["model"], "fixture");
            assert_eq!(body["messages"][0]["content"], "hello");
            assert_eq!(body["max_tokens"], 12);
            let reply = r#"{"choices":[{"message":{"content":"response"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}/v1"), Some("test-key".into()));
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(12) }).await.unwrap();
        assert_eq!(result.content, "response");
        assert_eq!((result.token_count_input, result.token_count_output), (Some(3), Some(2)));
        server.join().unwrap();
    }

    #[test]
    fn parses_buffered_sse_and_multipart_content() {
        let content = serde_json::json!({"message":{"content":[{"text":"first"},{"text":" second"}]}});
        assert_eq!(OpenAiCompatibleClient::extract_choice_content(&content).as_deref(), Some("first second"));
        assert!(OpenAiCompatibleClient::reject_non_text_choice(&serde_json::json!({"message":{"content":"ok","tool_calls":[]}}), "message").is_ok());
        let stream = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n";
        assert_eq!(OpenAiCompatibleClient::parse_sse_fallback(stream).unwrap().content, "hello");
        assert!(OpenAiCompatibleClient::parse_sse_fallback("data: [DONE]\n").is_err());
    }

    #[tokio::test]
    async fn sends_authenticated_multipart_audio_and_returns_transcript() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&buffer[..n]);
                if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
            assert!(headers.starts_with("post /v1/audio/transcriptions "));
            assert!(headers.contains("authorization: bearer audio-key"));
            assert!(headers.contains("multipart/form-data; boundary="));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < end + length {
                let n = socket.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&buffer[..n]);
            }
            let body = String::from_utf8_lossy(&data[end..end + length]);
            for expected in ["name=\"model\"", "whisper-fixture", "name=\"language\"", "en-US", "filename=\"sample.wav\"", "RIFFfixture"] { assert!(body.contains(expected), "missing {expected}"); }
            let reply = r#"{"text":"  hello world  "}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}"), Some("audio-key".into()));
        let text = client.transcribe(TranscriptionRequest { model: "whisper-fixture".into(), audio: b"RIFFfixture".to_vec(), mime_type: "audio/wav".into(), filename: "sample.wav".into(), language: Some("en-US".into()) }).await.unwrap();
        assert_eq!(text, "hello world");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn identifies_json_only_transcription_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).unwrap();
            let reply = "expected application/json";
            write!(socket, "HTTP/1.1 415 Unsupported Media Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}"), None);
        let result = client.transcribe(TranscriptionRequest { model: "fixture".into(), audio: vec![1, 2], mime_type: "audio/wav".into(), filename: "a.wav".into(), language: None }).await;
        assert!(matches!(result, Err(ClientError::UnsupportedTranscription)));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn requests_authenticated_wav_speech_and_rejects_invalid_audio() {
        for (reply, valid) in [(b"RIFF\x04\0\0\0WAVE".as_slice(), true), (b"not audio".as_slice(), false)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let reply = reply.to_vec();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let n = socket.read(&mut request).unwrap();
                let headers = String::from_utf8_lossy(&request[..n]).to_lowercase();
                assert!(headers.starts_with("post /v1/audio/speech "));
                assert!(headers.contains("authorization: bearer speech-key"));
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", reply.len()).unwrap();
                socket.write_all(&reply).unwrap();
            });
            let client = OpenAiCompatibleClient::new(format!("http://{addr}"), Some("speech-key".into()));
            let result = client.generate_speech(SpeechRequest { model: "voice".into(), input: "Hello".into(), voice: None, speed: None, response_format: "wav".into() }).await;
            assert_eq!(result.is_ok(), valid);
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn rejects_advertised_oversized_speech() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16777217\r\nConnection: close\r\n\r\n").unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}"), None);
        let result = client.generate_speech(SpeechRequest { model: "voice".into(), input: "Hello".into(), voice: None, speed: None, response_format: "wav".into() }).await;
        assert!(result.unwrap_err().to_string().contains("size limit"));
        server.join().unwrap();
    }

    async fn openai_stream_fixture(body: &str) -> (Result<CompletionResponse, ClientError>, Vec<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let bytes = body.as_bytes().to_vec();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new(); let mut buf = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); request.extend_from_slice(&buf[..n]);
                if let Some(i) = request.windows(4).position(|part| part == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
            assert!(headers.starts_with("post /v1/chat/completions "));
            assert!(headers.contains("authorization: bearer test-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while request.len() < end + length { let n = socket.read(&mut buf).unwrap(); assert_ne!(n, 0); request.extend_from_slice(&buf[..n]); }
            let json: serde_json::Value = serde_json::from_slice(&request[end..end + length]).unwrap();
            assert_eq!(json["stream"], true);
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
            for part in bytes.chunks(3) { if socket.write_all(part).is_err() { break; } }
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}/v1"), Some("test-key".into()));
        let mut chunks = Vec::new();
        let result = client.stream_completion(CompletionRequest { model:"fixture".into(), messages:vec![ChatMessage { role:"user".into(), content:"Hello".into() }], temperature:None, max_tokens:Some(20) }, |delta| chunks.push(delta.to_owned())).await;
        server.join().unwrap();
        (result, chunks)
    }

    #[tokio::test]
    async fn streams_fragmented_openai_events_and_optional_usage() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hé\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\n",
            "data: [DONE]\n\n");
        let (result, chunks) = openai_stream_fixture(body).await;
        let response = result.unwrap();
        assert_eq!(chunks, ["Hé", "llo"]);
        assert_eq!(response.content, "Héllo");
        assert_eq!((response.token_count_input, response.token_count_output), (Some(4), Some(2)));
    }

    #[tokio::test]
    async fn rejects_openai_stream_errors_and_incomplete_or_unsupported_events() {
        for body in [
            "data: {\"error\":{\"message\":\"overloaded\"}}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{}]}}]}\n\n",
            "data: malformed\n\n",
            "data: [DONE]\n\n",
            "data: {\"choices\":[{\"delta\":{\"refusal\":\"no\"}}]}\n\ndata: [DONE]\n\n",
        ] { assert!(openai_stream_fixture(body).await.0.is_err()); }
    }


    fn completion_request() -> CompletionRequest {
        CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(12) }
    }

    fn reply_fixture(response: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let _ = socket.write_all(&response);
        });
        (format!("http://{address}"), server)
    }

    #[tokio::test]
    async fn completion_rejects_provider_errors_tool_use_and_incomplete_streams() {
        for body in [
            r#"{"error":{"message":"synthetic-secret"}}"#,
            r#"{"choices":[]}"#,
            r#"{"choices":[{"message":{"content":null,"tool_calls":[{}]}}]}"#,
            r#"{"choices":[{"message":{"content":"partial","refusal":"no"}}]}"#,
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\ndata: {\"error\":{\"message\":\"synthetic-secret\"}}\n\ndata: [DONE]\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{}]}}]}\n\ndata: [DONE]\n\n",
            "data: malformed\n\ndata: [DONE]\n\n",
        ] {
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes();
            let (url, server) = reply_fixture(reply);
            let error = OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.unwrap_err().to_string();
            assert!(!error.contains("synthetic-secret"));
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn rejects_large_and_chunked_completion_responses() {
        let (url, server) = reply_fixture(b"HTTP/1.1 200 OK\r\nContent-Length: 8388609\r\nConnection: close\r\n\r\n".to_vec());
        let error = OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.unwrap_err().to_string();
        assert!(error.contains("size limit")); server.join().unwrap();
        let mut reply = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
        reply.extend_from_slice(b"800001\r\n"); reply.extend(std::iter::repeat_n(b'x', 8 * 1024 * 1024 + 1)); reply.extend_from_slice(b"\r\n0\r\n\r\n");
        let (url, server) = reply_fixture(reply);
        let error = OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.unwrap_err().to_string();
        assert!(error.contains("size limit")); server.join().unwrap();
    }

    #[tokio::test]
    async fn provider_error_and_malformed_responses_never_echo_body() {
        for status in ["401 Unauthorized", "200 OK"] {
            let body = "synthetic-provider-secret";
            let reply = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes();
            let (url, server) = reply_fixture(reply);
            let error = OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.unwrap_err().to_string();
            assert!(!error.contains(body));
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn rejects_redirects_and_embedded_url_credentials() {
        let target = TcpListener::bind("127.0.0.1:0").unwrap(); target.set_nonblocking(true).unwrap();
        let redirect = format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{}/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", target.local_addr().unwrap());
        let (url, server) = reply_fixture(redirect.into_bytes());
        assert!(OpenAiCompatibleClient::new(url, Some("synthetic-secret".into())).run_completion(completion_request()).await.is_err());
        assert!(target.accept().is_err()); server.join().unwrap();
        let client = OpenAiCompatibleClient::new("http://user:synthetic-secret@127.0.0.1:1".into(), None);
        let error = client.run_completion(completion_request()).await.unwrap_err().to_string();
        assert!(error.contains("Invalid model provider base URL"));
        assert!(!error.contains("synthetic-secret"));
    }

    #[tokio::test]
    async fn rejects_large_transcription_responses_and_hides_error_body() {
        for reply in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 2097153\r\nConnection: close\r\n\r\n".to_vec(),
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 25\r\nConnection: close\r\n\r\nsynthetic-provider-secret".to_vec(),
        ] {
            let (url, server) = reply_fixture(reply);
            let request = TranscriptionRequest { model: "fixture".into(), audio: vec![1, 2], mime_type: "audio/wav".into(), filename: "audio.wav".into(), language: None };
            let error = OpenAiCompatibleClient::new(url, None).transcribe(request).await.unwrap_err().to_string();
            assert!(!error.contains("synthetic-provider-secret")); server.join().unwrap();
        }
    }

}

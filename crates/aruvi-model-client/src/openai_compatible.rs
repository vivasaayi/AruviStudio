use crate::ClientError;
use crate::ModelGateway;
use crate::{CompletionRequest, CompletionResponse, SpeechRequest, TranscriptionRequest};
use async_trait::async_trait;
use reqwest::Client;

// Some streaming encoders use a sentinel for an unknown RIFF/data length.
// Validate the container structure without restricting WAV codecs.
fn has_wav_audio(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" { return false; }
    let length = |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let unknown = |size: u32| size == u32::MAX || size == i32::MAX as u32;
    let riff_size = length(4);
    if !unknown(riff_size) && riff_size as usize + 8 != bytes.len() { return false; }
    let mut offset = 12;
    let mut format = false;
    let mut data = false;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let declared = length(offset + 4);
        let start = offset + 8;
        if unknown(declared) {
            return id == b"data" && format && start < bytes.len();
        }
        let end = match start.checked_add(declared as usize) { Some(end) if end <= bytes.len() => end, _ => return false };
        if id == b"fmt " { format = declared >= 16; }
        if id == b"data" { data = declared > 0; }
        offset = match end.checked_add((declared & 1) as usize) { Some(end) if end <= bytes.len() => end, _ => return false };
    }
    offset == bytes.len() && format && data
}

pub struct OpenAiCompatibleClient {
    base_url: String,
    api_key: Option<String>,
    client: Option<Client>,
}

impl OpenAiCompatibleClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = crate::http::build_client(crate::ClientOptions::default()).ok();
        Self { base_url, api_key, client }
    }

    /// Construct a client with explicitly bounded connection and total request timeouts.
    pub fn with_options(base_url: String, api_key: Option<String>, options: crate::ClientOptions) -> Result<Self, ClientError> {
        let client = crate::http::build_client(options)?;
        Ok(Self { base_url, api_key, client: Some(client) })
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

    fn validate_completion_request(request: &CompletionRequest) -> Result<(), ClientError> {
        if request.model.trim().is_empty() || request.messages.is_empty() || request.max_tokens.is_some_and(|value| value <= 0) {
            return Err(ClientError::InvalidInput("A model, messages and positive max_tokens are required".into()));
        }
        if request.temperature.is_some_and(|value| !value.is_finite()) {
            return Err(ClientError::InvalidInput("Temperature must be finite".into()));
        }
        Ok(())
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
        let mut result = CompletionResponse::default();
        // Buffered completions permit the final event without a blank separator,
        // as before, while still requiring the provider's terminal marker.
        let mut parser = crate::sse::EventParser::new(8 * 1024 * 1024 + 2);
        let mut on_event = |payload: &[u8]| -> Result<bool, ClientError> {
            if payload == b"[DONE]" { return Ok(true); }
            let event: serde_json::Value = serde_json::from_slice(payload)
                .map_err(|_| ClientError::Provider("Invalid model stream event".into()))?;
            if event.get("error").is_some_and(|value| !value.is_null()) {
                return Err(ClientError::Provider("Model stream returned an error".into()));
            }
            let choice = event.get("choices").and_then(|v| v.as_array()).and_then(|v| v.first());
            crate::CompletionMetadata::merge(&mut result.metadata, &event, choice.and_then(|v| v.get("finish_reason")));
            if let Some(choice) = choice {
                Self::reject_non_text_choice(choice, "delta")?;
                if let Some(content) = Self::extract_choice_content(choice) { result.content.push_str(&content); }
            }
            if let Some(usage) = event.get("usage") {
                result.token_count_input = usage.get("prompt_tokens").and_then(|v| v.as_i64()).or(result.token_count_input);
                result.token_count_output = usage.get("completion_tokens").and_then(|v| v.as_i64()).or(result.token_count_output);
            }
            Ok(false)
        };
        let done = parser.push(raw_body.as_bytes(), &mut on_event)? || parser.push(b"\n\n", &mut on_event)?;
        if !done { return Err(ClientError::Provider("Model stream ended before its terminal event".into())); }
        if result.content.is_empty() { return Err(ClientError::Provider("Model stream returned no text".into())); }
        Ok(result)
    }

    fn reject_non_text_choice(choice: &serde_json::Value, field: &str) -> Result<(), ClientError> {
        let message = choice.get(field);
        if message.and_then(|value| value.get("tool_calls")).is_some_and(|value| value.as_array().map_or(!value.is_null(), |calls| !calls.is_empty()))
            || message.and_then(|value| value.get("function_call")).is_some_and(|value| !value.is_null()) {
            return Err(ClientError::Provider("Model tool-use responses are not supported by the text client".into()));
        }
        // Some compatible servers omit the field, and intermediate stream
        // chunks use null. Reject non-complete outcomes when they are supplied.
        match choice.get("finish_reason") {
            None | Some(serde_json::Value::Null) => {},
            Some(serde_json::Value::String(reason)) => match reason.as_str() {
                "stop" => {},
                "length" => return Err(ClientError::Provider("Model response was truncated by a token limit".into())),
                "content_filter" => return Err(ClientError::Provider("Model response was filtered".into())),
                "tool_calls" | "function_call" => return Err(ClientError::Provider("Model tool-use responses are not supported by the text client".into())),
                _ => return Err(ClientError::Provider("Model returned an unsupported finish reason".into())),
            },
            Some(_) => return Err(ClientError::Provider("Invalid model finish reason".into())),
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
        Self::validate_completion_request(&request)?;
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
        let mut result = CompletionResponse::default();
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
            let choice = event.get("choices").and_then(|value| value.as_array()).and_then(|values| values.first());
            crate::CompletionMetadata::merge(&mut result.metadata, &event, choice.and_then(|v| v.get("finish_reason")));
            if let Some(choice) = choice {
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
        if input.model.trim().is_empty() || input.audio.is_empty() {
            return Err(ClientError::InvalidInput("A model and nonempty audio are required for transcription".into()));
        }
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
        if !has_wav_audio(&audio) {
            return Err(ClientError::Provider("Speech provider did not return valid WAV audio".into()));
        }
        Ok(audio)
    }
}

impl OpenAiCompatibleClient {
    async fn request_completion(
        &self,
        request: CompletionRequest,
        json_object: bool,
    ) -> Result<CompletionResponse, ClientError> {
        Self::validate_completion_request(&request)?;
        let url = self.endpoint_url("/chat/completions")?;
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if json_object { body["response_format"] = serde_json::json!({"type":"json_object"}); }
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
                if raw_body.trim_start_matches('\u{feff}').split(['\r', '\n']).any(|line| line.starts_with("data:")) { return Self::parse_sse_fallback(raw_body); }
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

        let mut metadata = None;
        crate::CompletionMetadata::merge(&mut metadata, &json, choice.get("finish_reason"));
        Ok(CompletionResponse {
            content,
            metadata,
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

#[async_trait]
impl ModelGateway for OpenAiCompatibleClient {
    fn supports_json_object_completion(&self) -> bool { true }
    fn transport_capabilities(&self) -> crate::TransportCapabilities {
        crate::TransportCapabilities { text_completion: true, text_streaming: true, model_discovery: true, transcription: true, speech: true }
    }
    async fn stream_completion(&self, request: CompletionRequest, on_delta: &mut (dyn for<'a> FnMut(&'a str) + Send)) -> Result<CompletionResponse, ClientError> {
        OpenAiCompatibleClient::stream_completion(self, request, |delta: &str| on_delta(delta)).await
    }
    async fn transcribe(&self, request: TranscriptionRequest) -> Result<String, ClientError> {
        OpenAiCompatibleClient::transcribe(self, request).await
    }
    async fn generate_speech(&self, request: SpeechRequest) -> Result<Vec<u8>, ClientError> {
        OpenAiCompatibleClient::generate_speech(self, request).await
    }
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

    async fn run_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
        self.request_completion(request, false).await
    }

    async fn run_json_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, ClientError> {
        let response = self.request_completion(request, true).await?;
        let value: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|_| ClientError::Provider("Model did not return a JSON object".into()))?;
        if !value.is_object() { return Err(ClientError::Provider("Model did not return a JSON object".into())); }
        Ok(response)
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
            assert!(body.get("response_format").is_none());
            let reply = r#"{"id":"chat-1","model":"fixture-v1","choices":[{"message":{"content":"response"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"prompt_tokens_details":{"cached_tokens":1}}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{addr}/v1"), Some("test-key".into()));
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(12) }).await.unwrap();
        assert_eq!(result.content, "response");
        let meta = result.metadata.as_ref().unwrap();
        assert_eq!((meta.response_id.as_deref(), meta.model.as_deref(), meta.finish_reason.as_deref()), (Some("chat-1"), Some("fixture-v1"), Some("stop")));
        assert_eq!(meta.usage.as_ref().unwrap()["prompt_tokens_details"]["cached_tokens"], 1);
        assert_eq!((result.token_count_input, result.token_count_output), (Some(3), Some(2)));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn json_mode_is_opt_in_and_validates_provider_output() {
        for (content, valid) in [(r#"{"type":"final","summary":"ready"}"#, true), ("not JSON", false), ("[1,2]", false)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let content = content.to_owned();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut data = Vec::new(); let mut buffer = [0; 4096];
                let end = loop { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]); if let Some(i) = data.windows(4).position(|v| v == b"\r\n\r\n") { break i + 4; } };
                let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                assert!(headers.contains("authorization: bearer test-key"));
                let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
                while data.len() < end + length { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); data.extend_from_slice(&buffer[..n]); }
                let request: serde_json::Value = serde_json::from_slice(&data[end..end+length]).unwrap();
                assert_eq!(request["response_format"]["type"], "json_object");
                let body = serde_json::json!({"choices":[{"message":{"content":content},"finish_reason":"stop"}]}).to_string();
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            });
            let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{addr}"), Some("test-key".into())));
            let request = CompletionRequest { model:"fixture".into(), messages:vec![ChatMessage { role:"user".into(), content:"Return JSON".into() }], temperature:None, max_tokens:Some(20) };
            let result = client.run_json_completion(request).await;
            assert_eq!(result.is_ok(), valid);
            if !valid { assert!(!result.unwrap_err().to_string().contains("not JSON")); }
            server.join().unwrap();
        }
    }

    #[test]
    fn parses_buffered_sse_and_multipart_content() {
        let content = serde_json::json!({"message":{"content":[{"text":"first"},{"text":" second"}]}});
        assert_eq!(OpenAiCompatibleClient::extract_choice_content(&content).as_deref(), Some("first second"));
        assert!(OpenAiCompatibleClient::reject_non_text_choice(&serde_json::json!({"message":{"content":"ok","tool_calls":[]}}), "message").is_ok());
        let stream = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n";
        assert_eq!(OpenAiCompatibleClient::parse_sse_fallback(stream).unwrap().content, "hello");
        let cr_stream = "\u{feff}data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\r\rdata: [DONE]\r";
        assert_eq!(OpenAiCompatibleClient::parse_sse_fallback(cr_stream).unwrap().content, "hello");
        let stream = concat!(
            "data: {\"id\":\"buffered\",\"model\":\"fixture\",\"choices\":[{\"delta\":{\"content\":\"hello\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"usage\":{\"prompt_tokens\":3}}\n\n",
            "data: [DONE]\n\n");
        let result = OpenAiCompatibleClient::parse_sse_fallback(stream).unwrap();
        let meta = result.metadata.unwrap();
        assert_eq!((meta.response_id.as_deref(), meta.model.as_deref(), meta.finish_reason.as_deref()), (Some("buffered"), Some("fixture"), Some("stop")));
        assert_eq!(meta.usage.unwrap()["prompt_tokens"], 3);
        assert!(OpenAiCompatibleClient::parse_sse_fallback("data: [DONE]\n").is_err());
    }

    #[tokio::test]
    async fn transcription_rejects_missing_model_or_audio_before_network() {
        let client = OpenAiCompatibleClient::new("not a URL".into(), None);
        for (model, audio) in [("", vec![1]), ("  ", vec![1]), ("whisper", vec![])] {
            let input = TranscriptionRequest {
                model: model.into(), audio, mime_type: "audio/wav".into(),
                filename: "fixture.wav".into(), language: None,
            };
            assert!(matches!(client.transcribe(input).await, Err(ClientError::InvalidInput(message)) if message.contains("model and nonempty audio")));
        }
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
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{addr}"), Some("audio-key".into())));
        assert!(client.transport_capabilities().transcription);
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
        let wav = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data\x02\0\0\0\0\0";
        for (reply, valid) in [(wav.as_slice(), true), (b"RIFF\x04\0\0\0WAVE".as_slice(), false), (b"not audio".as_slice(), false)] {
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
            let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{addr}"), Some("speech-key".into())));
            assert!(client.transport_capabilities().speech);
            let result = client.generate_speech(SpeechRequest { model: "voice".into(), input: "Hello".into(), voice: None, speed: None, response_format: "wav".into() }).await;
            assert_eq!(result.is_ok(), valid);
            server.join().unwrap();
        }
    }

    #[test]
    fn validates_wav_chunks_and_streaming_lengths() {
        let valid = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data\x02\0\0\0\0\0";
        assert!(has_wav_audio(valid));
        let mut streamed = valid.to_vec();
        streamed[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        streamed[40..44].copy_from_slice(&(i32::MAX as u32).to_le_bytes());
        assert!(has_wav_audio(&streamed));
        assert!(!has_wav_audio(&valid[..valid.len() - 1]));
        let mut empty = valid.to_vec();
        empty[40..44].copy_from_slice(&0u32.to_le_bytes());
        assert!(!has_wav_audio(&empty));
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
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{addr}/v1"), Some("test-key".into())));
        assert!(client.transport_capabilities().text_streaming);
        assert!(client.supports_json_object_completion());
        let mut chunks = Vec::new();
        let result = client.stream_completion(CompletionRequest { model:"fixture".into(), messages:vec![ChatMessage { role:"user".into(), content:"Hello".into() }], temperature:None, max_tokens:Some(20) }, &mut |delta| chunks.push(delta.to_owned())).await;
        server.join().unwrap();
        (result, chunks)
    }

    #[tokio::test]
    async fn streams_fragmented_openai_events_and_optional_usage() {
        let body = concat!(
            "data: {\"id\":\"chat-stream\",\"model\":\"fixture-v1\",\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hé\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\n",
            "data: [DONE]\n\n");
        let (result, chunks) = openai_stream_fixture(body).await;
        let response = result.unwrap();
        assert_eq!(chunks, ["Hé", "llo"]);
        assert_eq!(response.content, "Héllo");
        assert_eq!((response.token_count_input, response.token_count_output), (Some(4), Some(2)));
        let meta = response.metadata.unwrap();
        assert_eq!((meta.response_id.as_deref(), meta.model.as_deref(), meta.finish_reason.as_deref()), (Some("chat-stream"), Some("fixture-v1"), Some("stop")));
        assert_eq!(meta.usage.unwrap()["completion_tokens"], 2);
    }

    #[tokio::test]
    async fn streams_cr_delimited_events_with_a_utf8_bom() {
        let body = "\u{feff}data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\r\rdata: [DONE]\r\r";
        let (result, chunks) = openai_stream_fixture(body).await;
        assert_eq!(result.unwrap().content, "hello");
        assert_eq!(chunks, ["hello"]);
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
    async fn buffered_completion_accepts_cr_delimited_sse_with_bom() {
        let body = "\u{feff}data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\r\rdata: [DONE]\r";
        let reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes();
        let (url, server) = reply_fixture(reply);
        let result = OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.unwrap();
        assert_eq!(result.content, "hello");
        server.join().unwrap();
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
    async fn rejects_incomplete_finish_reasons_in_all_completion_modes() {
        for reason in ["length", "content_filter", "tool_calls", "function_call", "future_unknown"] {
            let body = serde_json::json!({"choices":[{"message":{"content":"partial"},"finish_reason":reason}]}).to_string();
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes();
            let (url, server) = reply_fixture(reply);
            assert!(OpenAiCompatibleClient::new(url, None).run_completion(completion_request()).await.is_err());
            server.join().unwrap();
            let stream = format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"partial\"}},\"finish_reason\":null}}]}}\n\ndata: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{reason}\"}}]}}\n\ndata: [DONE]\n\n");
            assert!(OpenAiCompatibleClient::parse_sse_fallback(&stream).is_err());
            let (result, emitted) = openai_stream_fixture(&stream).await;
            assert!(result.is_err());
            assert_eq!(emitted, ["partial"]);
        }
        for reason in [serde_json::json!("stop"), serde_json::Value::Null] {
            let choice = serde_json::json!({"message":{"content":"ok"},"finish_reason":reason});
            assert!(OpenAiCompatibleClient::reject_non_text_choice(&choice, "message").is_ok());
        }
        assert!(OpenAiCompatibleClient::reject_non_text_choice(&serde_json::json!({"finish_reason":1}), "message").is_err());
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

    #[tokio::test]
    async fn completion_modes_reject_invalid_inputs_before_network() {
        let client = OpenAiCompatibleClient::new("http://127.0.0.1:1".into(), None);
        let make = || CompletionRequest { model: "fixture".into(), messages: vec![crate::ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        let mut invalid = Vec::new();
        let mut request = make(); request.model = "  ".into(); invalid.push(request);
        let mut request = make(); request.messages.clear(); invalid.push(request);
        let mut request = make(); request.max_tokens = Some(0); invalid.push(request);
        for temperature in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] { let mut request = make(); request.temperature = Some(temperature); invalid.push(request); }
        for request in invalid {
            assert!(matches!(client.run_completion(request.clone()).await, Err(ClientError::InvalidInput(_))));
            assert!(matches!(client.run_json_completion(request.clone()).await, Err(ClientError::InvalidInput(_))));
            assert!(matches!(client.stream_completion(request, |_| {}).await, Err(ClientError::InvalidInput(_))));
        }
    }

}

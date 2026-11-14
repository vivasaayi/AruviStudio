//! Independent-consumer regression fixture, pinned to a published Git commit.
#[cfg(test)]
mod tests {
    use aruvi_model_client::{anthropic::AnthropicClient, openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest, ModelGateway};
    use std::{io::{Read, Write}, net::TcpListener, thread};

    #[tokio::test]
    async fn independent_client_rejects_ambiguous_completion_choices() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).unwrap();
            let response = r#"{"choices":[{"index":0,"message":{"content":"first"}},{"index":1,"message":{"content":"second"}}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}"), None);
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        assert!(client.run_completion(request).await.is_err());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn independent_client_can_call_deepseek_voice() {
        use aruvi_model_client::deepseek::{DeepSeekVoiceClient, Voice};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]);
                if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") { break index + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
            assert!(headers.starts_with("post /v1/tools/tts "));
            assert!(headers.contains("authorization: bearer synthetic-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < end + length { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]); }
            let body: serde_json::Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
            assert_eq!(body["voice_id"], "tide");
            let body = "private fixture response";
            write!(socket, "HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = DeepSeekVoiceClient::new(format!("http://{address}"), "synthetic-key".into()).unwrap();
        let error = client.generate_speech("Hello", Some(Voice::Tide)).await.unwrap_err();
        assert_eq!(error.http_status(), Some(429));
        assert!(!error.to_string().contains("private fixture response"));
        server.join().unwrap();
    }

    #[test]
    fn rejects_invalid_anthropic_credentials_without_exposing_them() {
        let error = AnthropicClient::new("https://example.invalid".into(), "private-key\ninjected".into()).err().expect("invalid key must fail at construction");
        assert!(matches!(error, aruvi_model_client::ClientError::InvalidInput(_)));
        assert!(!format!("{error:?} {error}").contains("private-key"));
    }

    #[tokio::test]
    async fn configures_openai_completion_fields_from_an_independent_project() {
        use aruvi_model_client::openai_compatible::{CompletionOptions, TemperatureMode, TokenLimitField};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(index) = bytes.windows(4).position(|v| v == b"\r\n\r\n") { break index + 4; }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while bytes.len() < end + length { let n = socket.read(&mut buffer).unwrap(); assert_ne!(n, 0); bytes.extend_from_slice(&buffer[..n]); }
            let body: serde_json::Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
            assert_eq!(body["max_completion_tokens"], 42);
            assert!(body.get("max_tokens").is_none());
            assert!(body.get("temperature").is_none());
            let response = r#"{"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        });
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}"), None)
            .with_completion_options(CompletionOptions { token_limit_field: TokenLimitField::MaxCompletionTokens, temperature: TemperatureMode::Omit }));
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: Some(0.7), max_tokens: Some(42) };
        assert_eq!(client.run_completion(request).await.unwrap().content, "ok");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn transport_error_omits_sensitive_provider_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}/sensitive-path"), None));
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        let error = client.run_completion(request).await.unwrap_err();
        assert!(!error.to_string().contains("sensitive-path"));
    }

    #[tokio::test]
    async fn exposes_http_status_without_provider_response_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = "private provider detail";
            write!(socket, "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}"), None));
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        let error = client.run_completion(request).await.unwrap_err();
        assert_eq!(error.http_status(), Some(401));
        assert!(!format!("{error:?}").contains("private provider detail"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_malformed_anthropic_message_after_partial_text() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\ndata: {\"type\":\"message_delta\",\"delta\":42}\n\ndata: {\"type\":\"message_stop\"}\n\n";
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client: Box<dyn ModelGateway> = Box::new(AnthropicClient::new(format!("http://{address}"), "synthetic-key".into()).unwrap());
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        let mut partial = String::new();
        let error = client.stream_completion(request, &mut |delta| partial.push_str(delta)).await.unwrap_err();
        assert_eq!(partial, "partial");
        assert!(error.to_string().contains("Invalid Anthropic message delta"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_malformed_delta_after_partial_text() {
        for streaming in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
                let body = "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":42}}]}\n\ndata: [DONE]\n\n";
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            });
            let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}"), None));
            let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
            let mut partial = String::new();
            let result = if streaming { client.stream_completion(request, &mut |delta| partial.push_str(delta)).await } else { client.run_completion(request).await };
            assert!(result.unwrap_err().to_string().contains("Invalid model content"));
            if streaming { assert_eq!(partial, "partial"); }
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn rejects_empty_transcription_before_network() {
        let client = OpenAiCompatibleClient::new("not a URL".into(), None);
        let input = aruvi_model_client::TranscriptionRequest {
            model: "whisper".into(), audio: vec![], mime_type: "audio/wav".into(),
            filename: "fixture.wav".into(), language: None,
        };
        assert!(matches!(client.transcribe(input).await, Err(aruvi_model_client::ClientError::InvalidInput(message)) if message.contains("nonempty audio")));
    }

    #[tokio::test]
    async fn rejects_partial_multipart_response_from_public_gateway() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = r#"{"choices":[{"message":{"content":[{"type":"text","text":"partial"},{"type":"image","url":"fixture"}]},"finish_reason":"stop"}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}"), None));
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) };
        assert!(client.run_completion(request).await.unwrap_err().to_string().contains("unsupported or invalid"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_credentials_in_provider_url_without_exposing_them() {
        let client = OpenAiCompatibleClient::new("http://user:synthetic-secret@127.0.0.1:1".into(), None);
        let error = client.list_models_page(None).await.unwrap_err().to_string();
        assert!(error.contains("Invalid model provider base URL"));
        assert!(!error.contains("synthetic-secret"));
    }

    #[tokio::test]
    async fn consumes_cr_delimited_buffered_completion() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = "\u{feff}data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\r\rdata: [DONE]\r";
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}"), None);
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }).await.unwrap();
        assert_eq!(result.content, "hello");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_incomplete_buffered_completions() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n";
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}"), None);
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }).await;
        assert!(result.unwrap_err().to_string().contains("terminal event"));
        server.join().unwrap();
    }

    #[test]
    fn configures_timeout_from_an_independent_project() {
        let options = aruvi_model_client::ClientOptions { connect_timeout: std::time::Duration::from_secs(2), request_timeout: std::time::Duration::from_secs(15) };
        assert!(OpenAiCompatibleClient::with_options("http://localhost:1234".into(), None, options).is_ok());
        assert!(AnthropicClient::with_options("https://api.anthropic.com".into(), "synthetic-key".into(), options).is_ok());
        let invalid = aruvi_model_client::ClientOptions { connect_timeout: std::time::Duration::ZERO, ..options };
        assert!(OpenAiCompatibleClient::with_options("http://localhost".into(), None, invalid).is_err());
    }

    #[tokio::test]
    async fn rejects_anthropic_truncated_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = r#"{"content":[{"type":"text","text":"partial"}],"stop_reason":"max_tokens"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = AnthropicClient::new(format!("http://{address}"), "synthetic-key".into()).unwrap();
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }).await;
        assert!(result.unwrap_err().to_string().contains("truncated"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_completed_anthropic_server_tool_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = r#"{"content":[{"type":"text","text":"partial"},{"type":"server_tool_use","name":"web_search"}],"stop_reason":"end_turn"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = AnthropicClient::new(format!("http://{address}"), "synthetic-key".into()).unwrap();
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }).await;
        assert!(result.unwrap_err().to_string().contains("tool-use"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn uses_json_object_mode_from_an_independent_project() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = Vec::new(); let mut buffer = [0; 4096];
            let end = loop { let count = socket.read(&mut buffer).unwrap(); assert_ne!(count, 0); data.extend_from_slice(&buffer[..count]); if let Some(index) = data.windows(4).position(|part| part == b"\r\n\r\n") { break index + 4; } };
            let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len() < end + length { let count = socket.read(&mut buffer).unwrap(); assert_ne!(count, 0); data.extend_from_slice(&buffer[..count]); }
            let body: serde_json::Value = serde_json::from_slice(&data[end..end + length]).unwrap();
            assert_eq!(body["response_format"]["type"], "json_object");
            let response = serde_json::json!({"choices":[{"message":{"content":"{\"ok\":true}"},"finish_reason":"stop"}]}).to_string();
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        });
        let client: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(format!("http://{address}"), None));
        assert!(client.supports_json_object_completion());
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "Return JSON".into() }], temperature: None, max_tokens: Some(10) };
        let response = client.run_json_completion(request).await.unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&response.content).unwrap()["ok"], true);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_openai_token_limit_finish() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096]; let _ = socket.read(&mut request).unwrap();
            let body = r#"{"choices":[{"message":{"content":"partial"},"finish_reason":"length"}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}"), None);
        let result = client.run_completion(CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }).await;
        assert!(result.unwrap_err().to_string().contains("truncated"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn unified_gateway_exposes_capabilities_and_unsupported_voice() {
        let client: Box<dyn ModelGateway> = Box::new(AnthropicClient::new("http://127.0.0.1:1".into(), "synthetic-key".into()).unwrap());
        let capabilities = client.transport_capabilities();
        assert!(!client.supports_json_object_completion());
        assert!(capabilities.text_streaming);
        assert!(!capabilities.speech);
        let input = aruvi_model_client::SpeechRequest { model: "voice".into(), input: "hi".into(), voice: None, speed: None, response_format: "wav".into() };
        assert!(matches!(client.generate_speech(input).await, Err(aruvi_model_client::ClientError::UnsupportedOperation("speech generation"))));
        let openai: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new("http://127.0.0.1:1".into(), None));
        assert!(openai.transport_capabilities().transcription);
    }

    #[tokio::test]
    async fn consumes_public_discovery_api() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new(); let mut buffer = [0; 2048];
            loop {
                let size = socket.read(&mut buffer).unwrap();
                assert_ne!(size, 0);
                request.extend_from_slice(&buffer[..size]);
                if request.windows(4).any(|part| part == b"\r\n\r\n") { break; }
            }
            let headers = String::from_utf8_lossy(&request).to_lowercase();
            assert!(headers.starts_with("get /v1/models "));
            assert!(headers.contains("authorization: bearer synthetic-test-key"));
            let body = r#"{"data":[{"id":"coding-model","task":"text"}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}/v1"), Some("synthetic-test-key".into()));
        let page = client.list_models_page(None).await.unwrap();
        assert_eq!(page.models[0].id, "coding-model");
        assert_eq!(page.models[0].metadata["task"], "text");
        assert!(page.next_cursor.is_none());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn consumes_public_api_and_performs_authenticated_completion() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            let header_end = loop {
                let count = socket.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(index) = request.windows(4).position(|part| part == b"\r\n\r\n") { break index + 4; }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]).to_lowercase();
            assert!(headers.starts_with("post /v1/chat/completions "));
            assert!(headers.contains("authorization: bearer synthetic-test-key"));
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while request.len() < header_end + length {
                let count = socket.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let body = r#"{"id":"consumer-response","model":"fixture-v1","choices":[{"message":{"content":"consumer fixture"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}/v1"), Some("synthetic-test-key".into()));
        let response = client.run_completion(CompletionRequest {
            model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }],
            temperature: None, max_tokens: Some(10),
        }).await.unwrap();
        assert_eq!(response.content, "consumer fixture");
        assert_eq!(response.token_count_input, Some(3));
        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.response_id.as_deref(), Some("consumer-response"));
        assert_eq!(metadata.finish_reason.as_deref(), Some("stop"));
        assert_eq!(metadata.usage.unwrap()["completion_tokens"], 2);
        assert!(AnthropicClient::new("https://api.anthropic.com".into(), "synthetic-test-key".into()).is_ok());
        server.join().unwrap();
    }
    #[tokio::test]
    async fn invalid_completion_inputs_fail_before_network() {
        let client = OpenAiCompatibleClient::new("http://127.0.0.1:1".into(), None);
        let request = CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: Some(f64::NAN), max_tokens: Some(10) };
        assert!(matches!(client.run_completion(request.clone()).await, Err(aruvi_model_client::ClientError::InvalidInput(_))));
        assert!(matches!(client.run_json_completion(request.clone()).await, Err(aruvi_model_client::ClientError::InvalidInput(_))));
        assert!(matches!(client.stream_completion(request, |_| {}).await, Err(aruvi_model_client::ClientError::InvalidInput(_))));
    }

}

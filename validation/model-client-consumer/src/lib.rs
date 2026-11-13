//! Independent-consumer regression fixture, pinned to a published Git commit.
#[cfg(test)]
mod tests {
    use aruvi_model_client::{anthropic::AnthropicClient, openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest, ModelGateway};
    use std::{io::{Read, Write}, net::TcpListener, thread};

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
            let body = r#"{"choices":[{"message":{"content":"consumer fixture"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let client = OpenAiCompatibleClient::new(format!("http://{address}/v1"), Some("synthetic-test-key".into()));
        let response = client.run_completion(CompletionRequest {
            model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }],
            temperature: None, max_tokens: Some(10),
        }).await.unwrap();
        assert_eq!(response.content, "consumer fixture");
        assert_eq!(response.token_count_input, Some(3));
        assert!(AnthropicClient::new("https://api.anthropic.com".into(), "synthetic-test-key".into()).is_ok());
        server.join().unwrap();
    }
}

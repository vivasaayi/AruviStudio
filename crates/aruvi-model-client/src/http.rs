use crate::{ClientError, ClientOptions};
use reqwest::{Client, redirect::Policy};
use std::time::Duration;

pub(crate) fn build_client(options: ClientOptions) -> Result<Client, ClientError> {
    let max = Duration::from_secs(24 * 60 * 60);
    if options.connect_timeout.is_zero() || options.request_timeout.is_zero()
        || options.connect_timeout > options.request_timeout || options.request_timeout > max {
        return Err(ClientError::InvalidInput("Timeouts must be positive, connection timeout must not exceed request timeout, and request timeout must not exceed 24 hours".into()));
    }
    Client::builder().connect_timeout(options.connect_timeout).timeout(options.request_timeout)
        .redirect(Policy::none()).build()
        .map_err(|_| ClientError::Provider("Unable to initialize model HTTP client".into()))
}

/// Bound both advertised and chunked responses before parsing their content.
pub(crate) async fn read_bounded(mut response: reqwest::Response, limit: usize, label: &str) -> Result<Vec<u8>, ClientError> {
    if response.content_length().is_some_and(|length| length > limit as u64) {
        return Err(ClientError::Provider(format!("{label} exceeds size limit")));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| ClientError::Provider(format!("{label} read failed: {error}")))? {
        if chunk.len() > limit - bytes.len() {
            return Err(ClientError::Provider(format!("{label} exceeds size limit")));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{anthropic::AnthropicClient, openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest};
    use std::{io::{Read, Write}, net::TcpListener, thread};

    fn request() -> CompletionRequest {
        CompletionRequest { model: "fixture".into(), messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }], temperature: None, max_tokens: Some(10) }
    }

    // Send headers immediately, then stall the body. This checks the total
    // timeout still applies after connection and throughout stream consumption.
    fn stalled_stream() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data = [0; 4096]; let _ = socket.read(&mut data);
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 100\r\nConnection: close\r\n\r\n");
            thread::sleep(Duration::from_millis(400));
        });
        (format!("http://{addr}"), server)
    }

    // Keep an incomplete response open after the first delta. Once the caller
    // drops its future, the server must observe that the client closed the socket.
    fn cancellable_stream(event: &'static str) -> (String, thread::JoinHandle<bool>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buf).unwrap();
                assert_ne!(n, 0);
                request.extend_from_slice(&buf[..n]);
                if let Some(i) = request.windows(4).position(|v| v == b"\r\n\r\n") { break i + 4; }
            };
            let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while request.len() < end + length {
                let n = socket.read(&mut buf).unwrap();
                assert_ne!(n, 0);
                request.extend_from_slice(&buf[..n]);
            }
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 100000\r\nConnection: close\r\n\r\ndata: {event}\n\n").unwrap();
            matches!(socket.read(&mut buf), Ok(0))
        });
        (format!("http://{addr}"), server)
    }

    #[tokio::test]
    async fn dropping_stream_future_closes_both_provider_connections() {
        for anthropic in [false, true] {
            let event = if anthropic {
                r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"partial"}}"#
            } else {
                r#"{"choices":[{"delta":{"content":"partial"}}]}"#
            };
            let (url, server) = cancellable_stream(event);
            let (sent, received) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(async move {
                let mut sent = Some(sent);
                let mut callback = |_: &str| { if let Some(sent) = sent.take() { let _ = sent.send(()); } };
                if anthropic {
                    AnthropicClient::new(url, "synthetic-key".into()).unwrap()
                        .stream_completion(request(), &mut callback).await
                } else {
                    OpenAiCompatibleClient::new(url, None)
                        .stream_completion(request(), &mut callback).await
                }
            });
            tokio::time::timeout(Duration::from_secs(5), received).await.expect("first delta timed out").expect("stream closed before delta");
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            assert!(server.join().unwrap(), "server did not observe client disconnect");
        }
    }

    #[test]
    fn timeout_options_are_validated_for_both_providers() {
        let invalid = [
            ClientOptions { connect_timeout: Duration::ZERO, request_timeout: Duration::from_secs(1) },
            ClientOptions { connect_timeout: Duration::from_secs(1), request_timeout: Duration::ZERO },
            ClientOptions { connect_timeout: Duration::from_secs(2), request_timeout: Duration::from_secs(1) },
            ClientOptions { connect_timeout: Duration::from_secs(1), request_timeout: Duration::from_secs(86401) },
        ];
        for options in invalid {
            assert!(matches!(OpenAiCompatibleClient::with_options("http://localhost".into(), None, options), Err(ClientError::InvalidInput(_))));
            assert!(matches!(AnthropicClient::with_options("http://localhost".into(), "key".into(), options), Err(ClientError::InvalidInput(_))));
        }
    }

    #[tokio::test]
    async fn custom_request_timeout_covers_stream_body_for_both_providers() {
        let options = ClientOptions { connect_timeout: Duration::from_millis(100), request_timeout: Duration::from_millis(150) };
        let (url, server) = stalled_stream();
        let client = OpenAiCompatibleClient::with_options(url, None, options).unwrap();
        let error = client.stream_completion(request(), |_| {}).await.unwrap_err();
        assert!(error.to_string().contains("stream read failed"));
        assert!(!server.is_finished(), "request should time out before the server closes");
        server.join().unwrap();
        let (url, server) = stalled_stream();
        let client = AnthropicClient::with_options(url, "synthetic-key".into(), options).unwrap();
        let error = client.stream_completion(request(), |_| {}).await.unwrap_err();
        assert!(error.to_string().contains("stream read failed"));
        assert!(!server.is_finished(), "request should time out before the server closes");
        server.join().unwrap();
    }
}

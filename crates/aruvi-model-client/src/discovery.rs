use crate::{ClientError, DiscoveredModel};
use serde_json::Value;

const MAX_DISCOVERY_BYTES: usize = 2 * 1024 * 1024;
const MAX_MODELS: usize = 10_000;

pub(crate) async fn read_response(mut response: reqwest::Response) -> Result<Value, ClientError> {
    if !response.status().is_success() {
        return Err(ClientError::http("Model discovery API error", response.status()));
    }
    if response.content_length().is_some_and(|length| length > MAX_DISCOVERY_BYTES as u64) {
        return Err(ClientError::Provider("Model discovery response exceeds size limit".into()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| crate::http::transport_error("Model discovery response failed", error))? {
        if chunk.len() > MAX_DISCOVERY_BYTES - bytes.len() {
            return Err(ClientError::Provider("Model discovery response exceeds size limit".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ClientError::Provider("Invalid model discovery JSON".into()))
}

pub(crate) fn parse_models(body: &Value) -> Result<Vec<DiscoveredModel>, ClientError> {
    let items = body.get("data").and_then(Value::as_array)
        .ok_or_else(|| ClientError::Provider("Model discovery response is missing data".into()))?;
    if items.len() > MAX_MODELS {
        return Err(ClientError::Provider("Model discovery response has too many models".into()));
    }
    items.iter().map(|item| {
        let id = item.get("id").and_then(Value::as_str).filter(|id| !id.trim().is_empty() && id.len() <= 1024)
            .ok_or_else(|| ClientError::Provider("Model discovery returned an invalid model ID".into()))?;
        Ok(DiscoveredModel {
            id: id.to_string(),
            display_name: item.get("display_name").and_then(Value::as_str).map(str::to_owned),
            metadata: item.clone(),
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use crate::{anthropic::AnthropicClient, openai_compatible::OpenAiCompatibleClient, ModelGateway};
    use std::{io::{Read, Write}, net::TcpListener, thread};

    fn fixture(status: &str, body: String, expected_path: &'static str, expected_auth: &'static str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_string();
        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_write_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut bytes = Vec::new(); let mut buffer = [0; 2048];
            loop {
                let count = socket.read(&mut buffer).unwrap(); assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                if bytes.windows(4).any(|item| item == b"\r\n\r\n") { break; }
            }
            let headers = String::from_utf8_lossy(&bytes).to_lowercase();
            assert!(headers.starts_with(expected_path), "{headers}");
            assert!(headers.contains(expected_auth), "missing synthetic auth header");
            write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        (format!("http://{address}/v1"), handle)
    }

    #[tokio::test]
    async fn openai_discovers_models_with_metadata_and_authentication() {
        let (base, server) = fixture("200 OK", r#"{"data":[{"id":"local-code","owned_by":"host","task":"text-generation"}]}"#.into(), "get /v1/models ", "authorization: bearer test-key");
        let client = OpenAiCompatibleClient::new(base, Some("test-key".into()));
        let page = client.list_models_page(None).await.unwrap();
        assert_eq!(page.models[0].id, "local-code");
        assert_eq!(page.models[0].metadata["task"], "text-generation");
        assert!(page.next_cursor.is_none());
        assert!(client.list_models_page(Some("unexpected")).await.is_err());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn anthropic_forwards_cursor_and_returns_next_page_without_inventing_capabilities() {
        let (base, server) = fixture("200 OK", r#"{"data":[{"id":"claude-next","display_name":"Claude next","capabilities":{"batch":{"supported":true}}}],"has_more":true,"last_id":"claude-next"}"#.into(), "get /v1/models?after_id=cursor%2f1 ", "x-api-key: test-key");
        let client = AnthropicClient::new(base, "test-key".into()).unwrap();
        let page = client.list_models_page(Some("cursor/1")).await.unwrap();
        assert_eq!(page.models[0].display_name.as_deref(), Some("Claude next"));
        assert_eq!(page.models[0].metadata["capabilities"]["batch"]["supported"], true);
        assert_eq!(page.next_cursor.as_deref(), Some("claude-next"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_bad_status_malformed_data_and_stalled_pagination() {
        for (status, body) in [("401 Unauthorized", "{}"), ("200 OK", "{}"), ("200 OK", "{\"data\":[{}]}")] {
            let (base, server) = fixture(status, body.into(), "get /v1/models ", "authorization: bearer test-key");
            assert!(OpenAiCompatibleClient::new(base, Some("test-key".into())).list_models_page(None).await.is_err());
            server.join().unwrap();
        }
        let (base, server) = fixture("200 OK", r#"{"data":[],"has_more":true,"last_id":"same"}"#.into(), "get /v1/models?after_id=same ", "x-api-key: test-key");
        assert!(AnthropicClient::new(base, "test-key".into()).unwrap().list_models_page(Some("same")).await.is_err());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn rejects_oversized_discovery_payloads() {
        let body = format!("{{\"data\":[],\"padding\":\"{}\"}}", "x".repeat(2 * 1024 * 1024));
        let (base, server) = fixture("200 OK", body, "get /v1/models ", "authorization: bearer test-key");
        assert!(OpenAiCompatibleClient::new(base, Some("test-key".into())).list_models_page(None).await.is_err());
        let _ = server.join(); // Client can close the socket before the oversized body is sent.
    }
}

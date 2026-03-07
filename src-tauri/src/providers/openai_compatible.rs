use crate::error::AppError;
use crate::providers::gateway::ModelGateway;
use crate::providers::types::{CompletionRequest, CompletionResponse};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            base_url,
            api_key,
            client,
        }
    }

    fn endpoint_url(&self, path: &str) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            format!("{trimmed}{path}")
        } else {
            format!("{trimmed}/v1{path}")
        }
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

    fn parse_sse_fallback(raw_body: &str) -> Option<CompletionResponse> {
        let mut combined_content = String::new();
        let mut token_count_input: Option<i64> = None;
        let mut token_count_output: Option<i64> = None;
        let mut saw_event = false;

        for line in raw_body.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("data:") {
                continue;
            }
            saw_event = true;

            let payload = trimmed.trim_start_matches("data:").trim();
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }

            let event_json = match serde_json::from_str::<serde_json::Value>(payload) {
                Ok(json) => json,
                Err(_) => continue,
            };

            if let Some(choice) = event_json
                .get("choices")
                .and_then(|choices| choices.as_array())
                .and_then(|choices| choices.first())
            {
                if let Some(chunk) = Self::extract_choice_content(choice) {
                    combined_content.push_str(&chunk);
                }
            }

            if let Some(usage) = event_json.get("usage") {
                token_count_input = usage.get("prompt_tokens").and_then(|value| value.as_i64());
                token_count_output = usage
                    .get("completion_tokens")
                    .and_then(|value| value.as_i64());
            }
        }

        if saw_event && !combined_content.is_empty() {
            Some(CompletionResponse {
                content: combined_content,
                token_count_input,
                token_count_output,
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl ModelGateway for OpenAiCompatibleProvider {
    async fn health_check(&self) -> Result<bool, AppError> {
        let url = self.endpoint_url("/models");
        let mut req = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }
        match req.send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(AppError::Provider(format!("Health check failed: {e}"))),
        }
    }

    async fn run_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, AppError> {
        let url = self.endpoint_url("/chat/completions");
        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        let mut req = self.client.post(&url).json(&body);
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("Request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("API error {status}: {text}")));
        }

        let raw_body = resp
            .text()
            .await
            .map_err(|e| AppError::Provider(format!("Failed to read response body: {e}")))?;

        let json: serde_json::Value = match serde_json::from_str(&raw_body) {
            Ok(value) => value,
            Err(error) => {
                if let Some(fallback) = Self::parse_sse_fallback(&raw_body) {
                    return Ok(fallback);
                }
                let response_preview = raw_body.chars().take(400).collect::<String>();
                return Err(AppError::Provider(format!(
                    "Parse error: {error}; response_preview: {response_preview}"
                )));
            }
        };
        let content = json
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .and_then(Self::extract_choice_content)
            .unwrap_or_default();

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

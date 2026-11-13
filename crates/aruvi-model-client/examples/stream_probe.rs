//! Minimal streaming smoke test. Pass the provider key on stdin; never on the command line.
use aruvi_model_client::{openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest};
use std::io::Read;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("ARUVI_PROBE_BASE_URL")?;
    let model = std::env::var("ARUVI_PROBE_MODEL")?;
    let mut key = String::new();
    std::io::stdin().read_to_string(&mut key)?;
    let key = (!key.trim().is_empty()).then(|| key.trim().to_owned());
    let client = OpenAiCompatibleClient::new(base_url, key);
    let mut chunks = 0usize;
    let response = client.stream_completion(CompletionRequest {
        model,
        messages: vec![ChatMessage { role: "user".into(), content: "Say hello.".into() }],
        temperature: Some(0.2), max_tokens: Some(24),
    }, |_| chunks += 1).await?;
    if response.content.is_empty() || chunks == 0 { return Err("No streamed text received".into()); }
    println!("Received {} text chunks ({} characters)", chunks, response.content.chars().count());
    Ok(())
}

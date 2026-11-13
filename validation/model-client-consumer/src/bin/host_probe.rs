//! Real-host check from a separate crate using the pinned public Git dependency.
//! Read the token from stdin; do not print requests, responses, or credentials.
use aruvi_model_client::{openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest, ModelGateway};
use std::io::Read;

fn request(model: &str, prompt: &str, max_tokens: i64) -> CompletionRequest {
    CompletionRequest { model: model.into(), messages: vec![ChatMessage { role: "user".into(), content: prompt.into() }], temperature: Some(0.1), max_tokens: Some(max_tokens) }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("ARUVI_PROBE_BASE_URL")?;
    let model = std::env::var("ARUVI_PROBE_MODEL")?;
    let mut token = String::new();
    std::io::stdin().read_to_string(&mut token)?;
    let token = token.trim();
    if token.is_empty() { return Err("Model Host token is empty".into()); }
    let gateway: Box<dyn ModelGateway> = Box::new(OpenAiCompatibleClient::new(base, Some(token.into())));
    let caps = gateway.transport_capabilities();
    if !caps.text_completion || !caps.text_streaming || !caps.model_discovery { return Err("Required client transport capability is unavailable".into()); }
    let page = gateway.list_models_page(None).await?;
    if !page.models.iter().any(|entry| entry.id == model) { return Err("Requested model not advertised".into()); }
    let response = gateway.run_completion(request(&model, "Reply with the word ready.", 100)).await?;
    if response.content.trim().is_empty() { return Err("Empty completion".into()); }
    let mut chunks = 0;
    let stream = gateway.stream_completion(request(&model, "Reply with the word ready.", 100), &mut |_| { chunks += 1; }).await?;
    if stream.content.trim().is_empty() || chunks == 0 { return Err("Empty streamed completion".into()); }
    let short = gateway.run_completion(request(&model, "Write a detailed explanation of recursion with three code examples.", 1)).await;
    match short {
        Err(error) if error.to_string().contains("truncated by a token limit") => {},
        _ => return Err("Expected token-limit error was not received".into()),
    }
    println!("Independent pinned client: discovery, completion, streaming, and token-limit handling passed ({chunks} stream chunks).");
    Ok(())
}

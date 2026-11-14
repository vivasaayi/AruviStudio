//! Real-host check from a separate crate using the pinned public Git dependency.
//! Read the token from stdin; do not print requests, responses, or credentials.
use aruvi_model_client::{openai_compatible::OpenAiCompatibleClient, ChatMessage, CompletionRequest, ModelGateway, SpeechRequest, TranscriptionRequest};
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
    if !response.content.to_lowercase().contains("ready") { return Err("Completion did not answer the fixture prompt".into()); }
    let mut chunks = 0;
    let mut emitted = String::new();
    let stream = gateway.stream_completion(request(&model, "Reply with the word ready.", 100), &mut |delta| { chunks += 1; emitted.push_str(delta); }).await?;
    if chunks == 0 || emitted != stream.content || !stream.content.to_lowercase().contains("ready") {
        return Err("Stream chunks or completed response did not match the fixture prompt".into());
    }
    if std::env::var_os("ARUVI_PROBE_JSON_MODE").is_some() {
        let json = gateway.run_json_completion(request(&model, "Return a JSON object with type set to final and summary set to ready.", 100)).await?;
        if json.content.trim().is_empty() { return Err("Empty JSON completion".into()); }
        println!("Independent pinned client: JSON-object completion passed.");
    }
    match (std::env::var("ARUVI_PROBE_VOICE_MODEL").ok(), std::env::var("ARUVI_PROBE_TRANSCRIPTION_MODEL").ok()) {
        (Some(voice_model), Some(transcription_model)) => {
            if !caps.speech || !caps.transcription { return Err("Voice transport capability is unavailable".into()); }
            if !page.models.iter().any(|entry| entry.id == voice_model) || !page.models.iter().any(|entry| entry.id == transcription_model) {
                return Err("Requested voice models not advertised".into());
            }
            let audio = gateway.generate_speech(SpeechRequest { model: voice_model, input: "The quick brown fox jumps over the lazy dog.".into(), voice: None, speed: None, response_format: "wav".into() }).await?;
            let transcript = gateway.transcribe(TranscriptionRequest { model: transcription_model, audio, mime_type: "audio/wav".into(), filename: "fixture.wav".into(), language: Some("en".into()) }).await?.to_lowercase();
            if !transcript.contains("fox") || !transcript.contains("dog") { return Err("Speech fixture transcript did not contain expected words".into()); }
            println!("Independent pinned client: synthetic speech generation and transcription passed.");
        }
        (None, None) => {},
        _ => return Err("Both voice model IDs are required".into()),
    }
    let short = gateway.run_completion(request(&model, "Write a detailed explanation of recursion with three code examples.", 1)).await;
    match short {
        Err(error) if error.to_string().contains("truncated by a token limit") => {},
        _ => return Err("Expected token-limit error was not received".into()),
    }
    println!("Independent pinned client: discovery, completion, streaming, and token-limit handling passed ({chunks} stream chunks).");
    Ok(())
}

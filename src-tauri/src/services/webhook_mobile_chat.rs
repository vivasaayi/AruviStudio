use crate::persistence::model_repo;
use crate::services::model_service::create_gateway;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::services::webhook_bridge::ensure_mobile_api_authorized;
use crate::services::webhook_mobile_model::{
    resolve_mobile_chat_model_name, resolve_mobile_chat_provider_id,
};
use crate::services::webhook_model_telemetry::{
    char_count_i64, elapsed_ms, record_webhook_model_call, WebhookModelCallContext,
    WebhookModelCallRecord,
};
use crate::services::webhook_service::WebhookState;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;
use std::time::Instant;
use tracing::error;

/// Newline-delimited JSON events: delta, done, or error. A final done event is
/// required before streamed text can be treated as a completed response.
pub(crate) async fn mobile_chat_stream(
    State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileChatCompletionRequest>,
) -> axum::response::Response {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let messages: Vec<_> = body.messages.into_iter().filter(|message| !message.content.trim().is_empty()).collect();
    if messages.is_empty() { return (StatusCode::BAD_REQUEST, "At least one chat message is required.").into_response(); }
    let provider_id = match resolve_mobile_chat_provider_id(&state.app_state, body.provider_id).await {
        Ok(id) => id, Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let model_name = match resolve_mobile_chat_model_name(&state.app_state, &provider_id, body.model_name).await {
        Ok(name) => name, Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let provider = match model_repo::get_provider(&state.app_state.db, &provider_id).await {
        Ok(provider) if provider.enabled => provider,
        Ok(_) => return (StatusCode::BAD_REQUEST, "Selected provider is disabled.").into_response(),
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let gateway = match crate::services::model_service::create_shared_gateway(&provider) {
        Ok(gateway) => gateway, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let (sender, receiver) = tokio::sync::mpsc::channel::<String>(1024);
    tokio::spawn(async move {
        let started = Instant::now();
        let mut partial = String::new();
        let mut overflow = false;
        let max_tokens = body.max_tokens.or(Some(4096));
        let temperature = body.temperature;
        let slow = std::sync::Arc::new(tokio::sync::Notify::new());
        let output = sender.clone();
        let signal = slow.clone();
        let result = {
            let mut on_delta = |delta: &str| {
                partial.push_str(delta);
                let event = serde_json::json!({"type":"delta","content":delta}).to_string() + "\n";
                if output.try_send(event).is_err() { overflow = true; signal.notify_one(); }
            };
            let future = gateway.stream_completion(CompletionRequest { model: model_name.clone(), messages: messages.clone(), temperature, max_tokens }, &mut on_delta);
            tokio::pin!(future);
            tokio::select! {
                result = &mut future => Some(result.map_err(|error| error.to_string())),
                _ = sender.closed() => None,
                _ = slow.notified() => Some(Err("Stream consumer could not keep up".to_string())),
            }
        };
        let result = if overflow && result.is_some() { Some(Err("Stream consumer could not keep up".to_string())) } else { result };
        let (status, message, input, output_tokens) = match &result {
            Some(Ok(response)) => ("completed", None, response.token_count_input, response.token_count_output),
            Some(Err(error)) => ("failed", Some(error.as_str()), None, None),
            None => ("failed", Some("Stream cancelled by client"), None, None),
        };
        if let Err(record_error) = record_webhook_model_call(&state.app_state, WebhookModelCallRecord {
            context: WebhookModelCallContext { source_kind: "mobile_chat", source_id: None, source_label: "Mobile Chat", session_id: None, product_id: None },
            provider: &provider, model_name: &model_name, messages: &messages, max_tokens, temperature,
            response_chars: char_count_i64(&partial), token_count_input: input, token_count_output: output_tokens,
            duration_ms: elapsed_ms(started), status, error_message: message, response_text: Some(&partial),
        }).await { error!(error = %record_error, "Failed to record mobile stream telemetry"); }
        let terminal = match result {
            Some(Ok(response)) => Some(serde_json::json!({"type":"done","metadata":response.metadata})),
            Some(Err(error)) => Some(serde_json::json!({"type":"error","error":error})),
            None => None,
        };
        if let Some(event) = terminal { let _ = sender.send(event.to_string() + "\n").await; }
    });
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|event| (Ok::<_, std::convert::Infallible>(event), receiver))
    });
    ([(axum::http::header::CONTENT_TYPE, "application/x-ndjson"), (axum::http::header::CACHE_CONTROL, "no-store")], axum::body::Body::from_stream(stream)).into_response()
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileChatCompletionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
}

pub(crate) async fn mobile_chat_completion(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileChatCompletionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let messages = body
        .messages
        .into_iter()
        .filter(|message| !message.content.trim().is_empty())
        .collect::<Vec<_>>();
    if messages.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "At least one chat message is required.",
        )
            .into_response();
    }

    let provider_id =
        match resolve_mobile_chat_provider_id(&state.app_state, body.provider_id).await {
            Ok(provider_id) => provider_id,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let model_name =
        match resolve_mobile_chat_model_name(&state.app_state, &provider_id, body.model_name).await
        {
            Ok(model_name) => model_name,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let provider = match model_repo::get_provider(&state.app_state.db, &provider_id).await {
        Ok(provider) if provider.enabled => provider,
        Ok(_) => {
            return (StatusCode::BAD_REQUEST, "Selected provider is disabled.").into_response()
        }
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let gateway = match create_gateway(&provider) {
        Ok(gateway) => gateway,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let max_tokens = body.max_tokens.or(Some(4096));
    let temperature = body.temperature;
    let started = Instant::now();
    match gateway
        .run_completion(CompletionRequest {
            model: model_name.clone(),
            messages: messages.clone(),
            temperature,
            max_tokens,
        })
        .await
    {
        Ok(response) => {
            if let Err(record_error) = record_webhook_model_call(
                &state.app_state,
                WebhookModelCallRecord {
                    context: WebhookModelCallContext {
                        source_kind: "mobile_chat",
                        source_id: None,
                        source_label: "Mobile Chat",
                        session_id: None,
                        product_id: None,
                    },
                    provider: &provider,
                    model_name: &model_name,
                    messages: &messages,
                    max_tokens,
                    temperature,
                    response_chars: char_count_i64(&response.content),
                    token_count_input: response.token_count_input,
                    token_count_output: response.token_count_output,
                    duration_ms: elapsed_ms(started),
                    status: "completed",
                    error_message: None,
                    response_text: Some(&response.content),
                },
            )
            .await
            {
                error!(error = %record_error, "Failed to record mobile chat telemetry");
            }
            Json::<CompletionResponse>(response).into_response()
        }
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_webhook_model_call(
                &state.app_state,
                WebhookModelCallRecord {
                    context: WebhookModelCallContext {
                        source_kind: "mobile_chat",
                        source_id: None,
                        source_label: "Mobile Chat",
                        session_id: None,
                        product_id: None,
                    },
                    provider: &provider,
                    model_name: &model_name,
                    messages: &messages,
                    max_tokens,
                    temperature,
                    response_chars: 0,
                    token_count_input: None,
                    token_count_output: None,
                    duration_ms: elapsed_ms(started),
                    status: "failed",
                    error_message: Some(&error_message),
                    response_text: None,
                },
            )
            .await
            {
                error!(error = %record_error, "Failed to record failed mobile chat telemetry");
            }
            (StatusCode::BAD_REQUEST, error_message).into_response()
        }
    }
}

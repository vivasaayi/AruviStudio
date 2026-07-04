use crate::persistence::model_repo;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
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
    let api_key = match secrets::resolve_provider_secret(&provider) {
        Ok(api_key) => api_key,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
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

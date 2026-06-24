use crate::persistence::{model_repo, planner_repo};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::secrets;
use crate::services::webhook_bridge::ensure_mobile_api_authorized;
use crate::services::webhook_mobile_model::{
    resolve_mobile_chat_model_name, resolve_mobile_chat_provider_id,
};
use crate::services::webhook_mobile_planner_chat_support::{
    build_mobile_planner_system_prompt, extract_json_payload, non_zero_token_count,
    persist_mobile_planner_assistant_message, persist_mobile_planner_tool_trace,
    resolve_mobile_planner_product_context, truncate_for_prompt, MobilePlannerToolTraceEntry,
};
use crate::services::webhook_mobile_planner_tools::execute_mobile_planner_mcp_tool;
use crate::services::webhook_model_telemetry::{
    char_count_i64, elapsed_ms, record_webhook_model_call, WebhookModelCallContext,
    WebhookModelCallRecord,
};
use crate::services::webhook_service::WebhookState;
use crate::state::AppState;
use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Instant;
use tracing::error;

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerChatSessionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    product_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MobilePlannerChatSessionResponse {
    session_id: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerChatTurnRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    product_id: Option<String>,
    messages: Vec<ChatMessage>,
    max_tool_steps: Option<u8>,
}

#[derive(Debug, Serialize)]
struct MobilePlannerChatTurnResponse {
    session_id: String,
    status: String,
    assistant_message: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
    tool_trace: Vec<MobilePlannerToolTraceEntry>,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
}

pub(crate) async fn mobile_create_planner_chat_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobilePlannerChatSessionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
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
    let active_product = match resolve_mobile_planner_product_context(
        &state.app_state,
        body.product_id.as_deref(),
        None,
        None,
    )
    .await
    {
        Ok(context) => context,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    match planner_repo::create_mobile_planner_chat_session(
        &state.app_state.db,
        &session_id,
        Some(&provider_id),
        Some(&model_name),
        active_product.as_ref().map(|product| product.id.as_str()),
        active_product.as_ref().map(|product| product.name.as_str()),
    )
    .await
    {
        Ok(session) => Json(MobilePlannerChatSessionResponse {
            session_id: session.id,
            provider_id,
            model_name,
            product_id: session.active_product_id,
            product_name: session.active_product_name,
        })
        .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_submit_planner_chat_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerChatTurnRequest>,
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
            "At least one planner chat message is required.",
        )
            .into_response();
    }
    let latest_user_input = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.trim().to_string())
        .filter(|content| !content.is_empty());
    let Some(latest_user_input) = latest_user_input else {
        return (
            StatusCode::BAD_REQUEST,
            "Planner chat turns require a user message.",
        )
            .into_response();
    };

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
    let existing_session =
        match planner_repo::get_mobile_planner_chat_session(&state.app_state.db, &session_id).await
        {
            Ok(session) => session,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    let active_product = match resolve_mobile_planner_product_context(
        &state.app_state,
        body.product_id.as_deref(),
        existing_session.active_product_id.as_deref(),
        Some(&latest_user_input),
    )
    .await
    {
        Ok(context) => context,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let updated_session = match planner_repo::update_mobile_planner_chat_session(
        &state.app_state.db,
        &session_id,
        Some(&provider_id),
        Some(&model_name),
        active_product.as_ref().map(|product| product.id.as_str()),
        active_product.as_ref().map(|product| product.name.as_str()),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if let Err(error) = planner_repo::append_mobile_planner_chat_message(
        &state.app_state.db,
        &uuid::Uuid::new_v4().to_string(),
        &session_id,
        "user",
        &latest_user_input,
    )
    .await
    {
        return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
    }
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
    match run_mobile_planner_chat_turn(
        &state.app_state,
        MobilePlannerChatTurnInput {
            gateway: &gateway,
            provider: &provider,
            session_id,
            provider_id,
            model_name,
            product_id: updated_session.active_product_id,
            product_name: updated_session.active_product_name,
            max_tool_steps: body.max_tool_steps.unwrap_or(4).clamp(1, 8),
        },
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

struct MobilePlannerChatTurnInput<'a> {
    gateway: &'a OpenAiCompatibleProvider,
    provider: &'a crate::domain::model::ModelProvider,
    session_id: String,
    provider_id: String,
    model_name: String,
    product_id: Option<String>,
    product_name: Option<String>,
    max_tool_steps: u8,
}

async fn run_mobile_planner_chat_turn(
    state: &AppState,
    input: MobilePlannerChatTurnInput<'_>,
) -> Result<MobilePlannerChatTurnResponse, String> {
    let MobilePlannerChatTurnInput {
        gateway,
        provider,
        session_id,
        provider_id,
        model_name,
        product_id,
        product_name,
        max_tool_steps,
    } = input;
    let persisted_messages =
        planner_repo::list_mobile_planner_chat_messages(&state.db, &session_id, 24)
            .await
            .map_err(|error| error.to_string())?;
    let prior_tool_traces =
        planner_repo::list_mobile_planner_chat_tool_traces(&state.db, &session_id, 12)
            .await
            .map_err(|error| error.to_string())?;

    let mut conversation =
        Vec::with_capacity(persisted_messages.len() + prior_tool_traces.len() + 4);
    conversation.push(ChatMessage {
        role: "system".to_string(),
        content: build_mobile_planner_system_prompt(product_id.as_deref(), product_name.as_deref()),
    });
    for message in persisted_messages {
        conversation.push(ChatMessage {
            role: message.role,
            content: message.content,
        });
    }
    if !prior_tool_traces.is_empty() {
        let mut trace_context =
            String::from("Recent persisted MCP tool observations for this planner session:\n");
        for trace in prior_tool_traces {
            trace_context.push_str("- ");
            trace_context.push_str(&trace.tool_name);
            trace_context.push_str(" args=");
            trace_context.push_str(&truncate_for_prompt(&trace.arguments_json, 600));
            if let Some(error) = trace.error {
                trace_context.push_str(" error=");
                trace_context.push_str(&truncate_for_prompt(&error, 400));
            } else if let Some(result_json) = trace.result_json {
                trace_context.push_str(" result=");
                trace_context.push_str(&truncate_for_prompt(&result_json, 1200));
            }
            trace_context.push('\n');
        }
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: trace_context,
        });
    }

    let mut tool_trace = Vec::new();
    let mut token_count_input = 0_i64;
    let mut token_count_output = 0_i64;

    for step in 1..=max_tool_steps {
        let max_tokens = Some(4096);
        let temperature = Some(0.2);
        let started = Instant::now();
        let completion = match gateway
            .run_completion(CompletionRequest {
                model: model_name.clone(),
                messages: conversation.clone(),
                temperature,
                max_tokens,
            })
            .await
        {
            Ok(completion) => completion,
            Err(error) => {
                let error_message = error.to_string();
                if let Err(record_error) = record_webhook_model_call(
                    state,
                    WebhookModelCallRecord {
                        context: WebhookModelCallContext {
                            source_kind: "mobile_planner_chat",
                            source_id: Some(&session_id),
                            source_label: "Mobile Planner Chat",
                            session_id: Some(&session_id),
                            product_id: product_id.as_deref(),
                        },
                        provider,
                        model_name: &model_name,
                        messages: &conversation,
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
                    error!(error = %record_error, "Failed to record failed mobile planner telemetry");
                }
                return Err(error_message);
            }
        };
        if let Err(record_error) = record_webhook_model_call(
            state,
            WebhookModelCallRecord {
                context: WebhookModelCallContext {
                    source_kind: "mobile_planner_chat",
                    source_id: Some(&session_id),
                    source_label: "Mobile Planner Chat",
                    session_id: Some(&session_id),
                    product_id: product_id.as_deref(),
                },
                provider,
                model_name: &model_name,
                messages: &conversation,
                max_tokens,
                temperature,
                response_chars: char_count_i64(&completion.content),
                token_count_input: completion.token_count_input,
                token_count_output: completion.token_count_output,
                duration_ms: elapsed_ms(started),
                status: "completed",
                error_message: None,
                response_text: Some(&completion.content),
            },
        )
        .await
        {
            error!(error = %record_error, "Failed to record mobile planner telemetry");
        }
        if let Some(tokens) = completion.token_count_input {
            token_count_input += tokens;
        }
        if let Some(tokens) = completion.token_count_output {
            token_count_output += tokens;
        }

        let model_output = completion.content.trim().to_string();
        let Some(decision) = extract_json_payload(&model_output)
            .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        else {
            persist_mobile_planner_assistant_message(state, &session_id, &model_output).await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message: model_output,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        };

        let decision_type = decision
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        if decision_type == "final" {
            let assistant_message = decision
                .get("message")
                .or_else(|| decision.get("assistant_message"))
                .or_else(|| decision.get("summary"))
                .and_then(Value::as_str)
                .unwrap_or(&model_output)
                .trim()
                .to_string();
            persist_mobile_planner_assistant_message(state, &session_id, &assistant_message)
                .await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        }

        if decision_type != "tool_call" {
            persist_mobile_planner_assistant_message(state, &session_id, &model_output).await?;
            return Ok(MobilePlannerChatTurnResponse {
                session_id,
                status: "final".to_string(),
                assistant_message: model_output,
                provider_id,
                model_name,
                product_id,
                product_name,
                tool_trace,
                token_count_input: non_zero_token_count(token_count_input),
                token_count_output: non_zero_token_count(token_count_output),
            });
        }

        let tool_name = decision
            .get("tool")
            .or_else(|| decision.get("tool_name"))
            .or_else(|| decision.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Planner tool call is missing tool name.".to_string())?
            .to_string();
        let arguments = decision
            .get("arguments")
            .or_else(|| decision.get("args"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let step_trace =
            match execute_mobile_planner_mcp_tool(state, step, &tool_name, arguments.clone()).await
            {
                Ok(result) => MobilePlannerToolTraceEntry {
                    step,
                    tool_name: tool_name.clone(),
                    arguments: arguments.clone(),
                    result: Some(result.clone()),
                    error: None,
                },
                Err(error) => MobilePlannerToolTraceEntry {
                    step,
                    tool_name: tool_name.clone(),
                    arguments: arguments.clone(),
                    result: None,
                    error: Some(error),
                },
            };
        persist_mobile_planner_tool_trace(state, &session_id, &step_trace).await?;
        let tool_observation = serde_json::to_string(&step_trace)
            .unwrap_or_else(|_| "{\"error\":\"failed to serialize tool trace\"}".to_string());
        tool_trace.push(step_trace);
        conversation.push(ChatMessage {
            role: "assistant".to_string(),
            content: model_output,
        });
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Tool observation for step {step}: {tool_observation}\nContinue. Return another tool_call only if essential; otherwise return type=final with a natural mobile-friendly summary of what you did, including created/updated item names and where they were added. End with a short follow-up invitation if another refinement would be useful."
            ),
        });
    }

    conversation.push(ChatMessage {
        role: "user".to_string(),
        content: "You reached the mobile planner tool-step limit. Return exactly one JSON object with type=final. In message, give a natural mobile-friendly summary of what you learned or changed, list created/updated item names when available, and invite one concise follow-up question. Do not call another tool.".to_string(),
    });
    let max_tokens = Some(2048);
    let temperature = Some(0.2);
    let started = Instant::now();
    let completion = match gateway
        .run_completion(CompletionRequest {
            model: model_name.clone(),
            messages: conversation.clone(),
            temperature,
            max_tokens,
        })
        .await
    {
        Ok(completion) => completion,
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_webhook_model_call(
                state,
                WebhookModelCallRecord {
                    context: WebhookModelCallContext {
                        source_kind: "mobile_planner_chat",
                        source_id: Some(&session_id),
                        source_label: "Mobile Planner Chat",
                        session_id: Some(&session_id),
                        product_id: product_id.as_deref(),
                    },
                    provider,
                    model_name: &model_name,
                    messages: &conversation,
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
                error!(error = %record_error, "Failed to record failed mobile planner telemetry");
            }
            return Err(error_message);
        }
    };
    if let Err(record_error) = record_webhook_model_call(
        state,
        WebhookModelCallRecord {
            context: WebhookModelCallContext {
                source_kind: "mobile_planner_chat",
                source_id: Some(&session_id),
                source_label: "Mobile Planner Chat",
                session_id: Some(&session_id),
                product_id: product_id.as_deref(),
            },
            provider,
            model_name: &model_name,
            messages: &conversation,
            max_tokens,
            temperature,
            response_chars: char_count_i64(&completion.content),
            token_count_input: completion.token_count_input,
            token_count_output: completion.token_count_output,
            duration_ms: elapsed_ms(started),
            status: "completed",
            error_message: None,
            response_text: Some(&completion.content),
        },
    )
    .await
    {
        error!(error = %record_error, "Failed to record mobile planner telemetry");
    }
    if let Some(tokens) = completion.token_count_input {
        token_count_input += tokens;
    }
    if let Some(tokens) = completion.token_count_output {
        token_count_output += tokens;
    }
    let assistant_message = extract_json_payload(&completion.content)
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| {
            value
                .get("message")
                .or_else(|| value.get("assistant_message"))
                .or_else(|| value.get("summary"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| completion.content.trim().to_string());
    persist_mobile_planner_assistant_message(state, &session_id, &assistant_message).await?;

    Ok(MobilePlannerChatTurnResponse {
        session_id,
        status: "tool_limit_final".to_string(),
        assistant_message,
        provider_id,
        model_name,
        product_id,
        product_name,
        tool_trace,
        token_count_input: non_zero_token_count(token_count_input),
        token_count_output: non_zero_token_count(token_count_output),
    })
}

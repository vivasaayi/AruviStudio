use crate::persistence::planner_repo;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::services::webhook_mobile_planner_chat_support::{
    build_mobile_planner_system_prompt, extract_json_payload, non_zero_token_count,
    persist_mobile_planner_assistant_message, persist_mobile_planner_tool_trace,
    truncate_for_prompt, MobilePlannerToolTraceEntry,
};
use crate::services::webhook_mobile_planner_tools::execute_mobile_planner_mcp_tool;
use crate::services::webhook_model_telemetry::{
    char_count_i64, elapsed_ms, record_webhook_model_call, WebhookModelCallContext,
    WebhookModelCallRecord,
};
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Instant;
use tracing::error;

#[derive(Debug, Serialize)]
pub(crate) struct MobilePlannerChatTurnResponse {
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

pub(crate) struct MobilePlannerChatTurnInput<'a> {
    pub(crate) gateway: &'a OpenAiCompatibleProvider,
    pub(crate) provider: &'a crate::domain::model::ModelProvider,
    pub(crate) session_id: String,
    pub(crate) provider_id: String,
    pub(crate) model_name: String,
    pub(crate) product_id: Option<String>,
    pub(crate) product_name: Option<String>,
    pub(crate) max_tool_steps: u8,
}

pub(crate) async fn run_mobile_planner_chat_turn(
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

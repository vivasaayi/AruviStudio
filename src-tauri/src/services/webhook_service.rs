use crate::mcp;
use crate::services::channel_service::{
    handle_inbound_message, resolve_twilio_config, ChannelInboundMessage,
};
use crate::services::webhook_bridge::{
    ensure_mcp_api_authorized, ensure_mcp_origin_allowed, resolve_webhook_bind_config,
};
pub use crate::services::webhook_bridge::{
    resolve_mcp_bridge_status, resolve_mobile_bridge_status, McpBridgeStatus, MobileBridgeStatus,
};
use crate::services::webhook_mobile_api::{
    mobile_validate_planner_draft, mobile_analyze_planner_repository, mobile_clear_planner_turn, mobile_confirm_planner_turn, mobile_create_planner_session, mobile_create_product, mobile_edit_planner_draft,
    mobile_get_model_call, mobile_get_product_tree, mobile_get_product_tree_summary, mobile_list_product_areas, mobile_create_hierarchy_node, mobile_update_hierarchy_node, mobile_delete_hierarchy_node, mobile_reset_product_plan,
    mobile_healthcheck, mobile_list_model_calls, mobile_model_call_snapshot, mobile_list_products, mobile_submit_planner_turn,
    mobile_list_providers, mobile_discover_provider_models, mobile_create_provider, mobile_update_provider, mobile_delete_provider, mobile_rotate_provider_credential, mobile_cleanup_provider_credentials, mobile_list_models, mobile_create_model, mobile_update_model, mobile_delete_model, mobile_test_provider,
    mobile_list_repositories, mobile_register_repository, mobile_inspect_repository, mobile_save_repository_file, mobile_list_repository_attachments, mobile_attach_repository, mobile_set_default_repository, mobile_detach_repository,
    mobile_list_agents, mobile_create_agent, mobile_update_agent, mobile_delete_agent, mobile_list_agent_bindings, mobile_set_agent_binding,
    mobile_list_teams, mobile_create_team, mobile_update_team, mobile_delete_team,
    mobile_list_team_members, mobile_add_team_member, mobile_remove_team_member,
    mobile_list_team_assignments, mobile_add_team_assignment, mobile_remove_team_assignment,
    mobile_list_skills, mobile_create_skill, mobile_update_skill, mobile_delete_skill,
    mobile_list_skill_links, mobile_add_skill_link, mobile_remove_skill_link,
    mobile_list_routing_policies, mobile_save_routing_policy, mobile_delete_routing_policy,
    mobile_get_workflow_test_config, mobile_set_workflow_test_config, mobile_ci_read, mobile_ci_run, mobile_ci_log, mobile_ci_run_action, mobile_ci_bindings, mobile_ci_binding_action, mobile_ci_revisions, mobile_ci_artifact_readiness, mobile_ci_feedback_list, mobile_ci_feedback_create,
    mobile_submit_planner_voice_turn, mobile_transcribe_audio, mobile_generate_speech, mobile_update_planner_session,
};
use crate::services::webhook_mobile_chat::mobile_chat_completion;
use crate::services::webhook_mobile_planner_chat::{
    mobile_create_planner_chat_session, mobile_submit_planner_chat_turn,
};
use crate::services::webhook_mobile_work::{
    mobile_approve_work_item, mobile_create_work_item, mobile_get_work_item, mobile_update_work_item,
    mobile_get_work_item_delivery, mobile_handle_workflow_action, mobile_retry_git_delivery, mobile_list_work_items,
    mobile_start_workflow, mobile_get_workflow_test_result,
};
use crate::services::webhook_remote_app::REMOTE_APP_HTML;
use crate::services::webhook_twilio::{
    messaging_twiml, planner_reply_text, validate_twilio_signature, voice_gather_twiml,
};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{error, info};

#[derive(Clone)]
pub struct WebhookState {
    pub app_state: AppState,
}

#[derive(Debug, Deserialize)]
struct TwilioMessagingForm {
    #[serde(rename = "Body")]
    body: Option<String>,
    #[serde(rename = "From")]
    from: Option<String>,
    #[serde(rename = "WaId")]
    wa_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwilioVoiceForm {
    #[serde(rename = "CallSid")]
    call_sid: Option<String>,
    #[serde(rename = "From")]
    from: Option<String>,
    #[serde(rename = "SpeechResult")]
    speech_result: Option<String>,
}

async fn healthcheck(State(state): State<WebhookState>) -> impl IntoResponse {
    match sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind("__healthcheck__")
        .fetch_optional(&state.app_state.db)
        .await
    {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "database unavailable"),
    }
}

async fn remote_app() -> impl IntoResponse {
    Html(REMOTE_APP_HTML)
}

fn json_response(status: StatusCode, payload: Value) -> Response {
    (status, Json(payload)).into_response()
}

async fn mcp_http_get(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mcp_origin_allowed(&state.app_state, &headers).await {
        return response;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This MCP endpoint accepts POST JSON-RPC requests. SSE is not enabled.",
    )
        .into_response()
}

async fn mcp_http_delete(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mcp_origin_allowed(&state.app_state, &headers).await {
        return response;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This MCP endpoint does not manage explicit HTTP sessions.",
    )
        .into_response()
}

async fn mcp_http_post(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = ensure_mcp_api_authorized(&state.app_state, &headers).await {
        return response;
    }

    let payload = match serde_json::from_slice::<Value>(&body) {
        Ok(payload) => payload,
        Err(error) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error",
                        "data": {
                            "details": error.to_string(),
                        }
                    }
                }),
            )
        }
    };

    match mcp::handle_json_rpc_value(&state.app_state, payload).await {
        Some(response) => json_response(StatusCode::OK, response),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn twilio_whatsapp(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let config = match resolve_twilio_config(&state.app_state).await {
        Ok(config) => config,
        Err(error) => {
            return Html(messaging_twiml(&format!("Planner error: {}", error))).into_response()
        }
    };
    if let Err(error) = validate_twilio_signature(
        config.auth_token.as_deref(),
        config.webhook_base_url.as_deref(),
        "/webhooks/twilio/whatsapp",
        &params,
        &headers,
    ) {
        return (axum::http::StatusCode::UNAUTHORIZED, error).into_response();
    }
    let form: TwilioMessagingForm = serde_json::from_value(
        serde_json::to_value(&params).unwrap_or_default(),
    )
    .unwrap_or(TwilioMessagingForm {
        body: None,
        from: None,
        wa_id: None,
    });
    let content = form.body.unwrap_or_default();
    let remote_user_id = form
        .wa_id
        .or(form.from.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let remote_conversation_id = form.from.unwrap_or_else(|| remote_user_id.clone());
    match handle_inbound_message(
        &state.app_state,
        ChannelInboundMessage {
            channel: "twilio_whatsapp".to_string(),
            remote_user_id,
            remote_conversation_id,
            content,
        },
    )
    .await
    {
        Ok(response) => Html(messaging_twiml(&planner_reply_text(&response))).into_response(),
        Err(error) => {
            error!(error = %error, "twilio whatsapp webhook failed");
            Html(messaging_twiml(&format!("Planner error: {}", error))).into_response()
        }
    }
}

async fn twilio_voice_entry() -> impl IntoResponse {
    Html(voice_gather_twiml(
        "Welcome to Aruvi planner. Tell me what you want to plan after the tone.",
    ))
}

async fn twilio_voice_gather(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let config = match resolve_twilio_config(&state.app_state).await {
        Ok(config) => config,
        Err(error) => {
            return Html(voice_gather_twiml(&format!("Planner error: {}", error))).into_response()
        }
    };
    if let Err(error) = validate_twilio_signature(
        config.auth_token.as_deref(),
        config.webhook_base_url.as_deref(),
        "/webhooks/twilio/voice/gather",
        &params,
        &headers,
    ) {
        return (axum::http::StatusCode::UNAUTHORIZED, error).into_response();
    }
    let form: TwilioVoiceForm = serde_json::from_value(
        serde_json::to_value(&params).unwrap_or_default(),
    )
    .unwrap_or(TwilioVoiceForm {
        call_sid: None,
        from: None,
        speech_result: None,
    });
    let content = form
        .speech_result
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "I need more detail".to_string());
    let remote_user_id = form.from.unwrap_or_else(|| "unknown".to_string());
    let remote_conversation_id = form.call_sid.unwrap_or_else(|| remote_user_id.clone());
    match handle_inbound_message(
        &state.app_state,
        ChannelInboundMessage {
            channel: "twilio_voice".to_string(),
            remote_user_id,
            remote_conversation_id,
            content,
        },
    )
    .await
    {
        Ok(response) => Html(voice_gather_twiml(&planner_reply_text(&response))).into_response(),
        Err(error) => {
            error!(error = %error, "twilio voice webhook failed");
            Html(voice_gather_twiml(&format!("Planner error: {}", error))).into_response()
        }
    }
}

async fn twilio_voice_entry_with_prompt(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let prompt = params.get("prompt").cloned().unwrap_or_else(|| {
        "Welcome to Aruvi planner. Tell me what you want to plan after the tone.".to_string()
    });
    Html(voice_gather_twiml(&prompt))
}

pub async fn start_webhook_server(app_state: AppState) -> Result<(), String> {
    let bind_config = resolve_webhook_bind_config(&app_state)
        .await
        .map_err(|error| format!("failed to resolve webhook bind config: {error}"))?;
    let host = bind_config.host;
    let port = bind_config.port;
    let bind_target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let address: SocketAddr = bind_target
        .parse()
        .map_err(|error| format!("invalid webhook bind address: {error}"))?;

    let router = build_webhook_router(app_state);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("failed to bind webhook server: {error}"))?;

    info!(address = %address, "webhook server listening");
    axum::serve(listener, router)
        .await
        .map_err(|error| format!("webhook server failed: {error}"))
}

pub(crate) fn build_webhook_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(healthcheck))
        .route("/remote", get(remote_app))
        .route("/remote/*path", get(remote_app))
        .route(
            "/api/mcp",
            get(mcp_http_get)
                .post(mcp_http_post)
                .delete(mcp_http_delete),
        )
        .route("/api/mobile/health", get(mobile_healthcheck))
        .route("/api/mobile/ci/:kind", get(mobile_ci_read))
        .route("/api/mobile/ci/bindings", get(mobile_ci_bindings))
        .route("/api/mobile/ci/bindings/:action", post(mobile_ci_binding_action))
        .route("/api/mobile/ci/repositories/:id/:kind", get(mobile_ci_revisions))
        .route("/api/mobile/ci/artifacts/:id/readiness", get(mobile_ci_artifact_readiness))
        .route("/api/mobile/ci/runs/:id", get(mobile_ci_run))
        .route("/api/mobile/ci/runs/:id/logs", get(mobile_ci_log))
        .route("/api/mobile/ci/runs/:id/feedback", get(mobile_ci_feedback_list).post(mobile_ci_feedback_create))
        .route("/api/mobile/ci/runs/:id/:action", post(mobile_ci_run_action))
        .route("/api/mobile/teams", get(mobile_list_teams).post(mobile_create_team))
        .route("/api/mobile/teams/update", post(mobile_update_team))
        .route("/api/mobile/teams/delete", post(mobile_delete_team))
        .route("/api/mobile/team-members", get(mobile_list_team_members).post(mobile_add_team_member))
        .route("/api/mobile/team-members/remove", post(mobile_remove_team_member))
        .route("/api/mobile/team-assignments", get(mobile_list_team_assignments).post(mobile_add_team_assignment))
        .route("/api/mobile/team-assignments/remove", post(mobile_remove_team_assignment))
        .route("/api/mobile/skills", get(mobile_list_skills).post(mobile_create_skill))
        .route("/api/mobile/skills/update", post(mobile_update_skill))
        .route("/api/mobile/skills/delete", post(mobile_delete_skill))
        .route("/api/mobile/skill-links", get(mobile_list_skill_links).post(mobile_add_skill_link))
        .route("/api/mobile/skill-links/remove", post(mobile_remove_skill_link))
        .route("/api/mobile/routing-policies", get(mobile_list_routing_policies).post(mobile_save_routing_policy))
        .route("/api/mobile/routing-policies/delete", post(mobile_delete_routing_policy))
        .route("/api/mobile/model-calls", get(mobile_list_model_calls))
        .route("/api/mobile/model-calls/:id/snapshots/:kind", get(mobile_model_call_snapshot))
        .route("/api/mobile/providers", get(mobile_list_providers).post(mobile_create_provider))
        .route("/api/mobile/providers/update", post(mobile_update_provider))
        .route("/api/mobile/providers/discover", post(mobile_discover_provider_models))
        .route("/api/mobile/providers/delete", post(mobile_delete_provider))
        .route("/api/mobile/providers/credential", post(mobile_rotate_provider_credential))
        .route("/api/mobile/providers/credential/cleanup", post(mobile_cleanup_provider_credentials))
        .route("/api/mobile/providers/:id/test", post(mobile_test_provider))
        .route("/api/mobile/models", get(mobile_list_models).post(mobile_create_model))
        .route("/api/mobile/models/update", post(mobile_update_model))
        .route("/api/mobile/models/delete", post(mobile_delete_model))
        .route("/api/mobile/products/:product_id/areas", get(mobile_list_product_areas))
        .route("/api/mobile/products/:product_id/hierarchy", post(mobile_create_hierarchy_node))
        .route("/api/mobile/products/:product_id/hierarchy/update", post(mobile_update_hierarchy_node))
        .route("/api/mobile/products/:product_id/hierarchy/delete", post(mobile_delete_hierarchy_node))
        .route("/api/mobile/products/:product_id/reset-plan", post(mobile_reset_product_plan))
        .route("/api/mobile/repositories", get(mobile_list_repositories).post(mobile_register_repository))
        .route("/api/mobile/repository-inspect", post(mobile_inspect_repository))
        .route("/api/mobile/repository-file/save", post(mobile_save_repository_file).layer(axum::extract::DefaultBodyLimit::max(16 * 1024 * 1024)))
        .route("/api/mobile/repository-attachments", get(mobile_list_repository_attachments).post(mobile_attach_repository))
        .route("/api/mobile/repository-attachments/default", post(mobile_set_default_repository))
        .route("/api/mobile/repository-attachments/detach", post(mobile_detach_repository))
        .route("/api/mobile/agents", get(mobile_list_agents).post(mobile_create_agent))
        .route("/api/mobile/agents/update", post(mobile_update_agent))
        .route("/api/mobile/agents/delete", post(mobile_delete_agent))
        .route("/api/mobile/agent-model-bindings", get(mobile_list_agent_bindings).post(mobile_set_agent_binding))
        .route("/api/mobile/workflow-test-config", get(mobile_get_workflow_test_config).post(mobile_set_workflow_test_config))
        .route(
            "/api/mobile/model-calls/:call_id",
            get(mobile_get_model_call),
        )
        .route("/api/mobile/products", get(mobile_list_products).post(mobile_create_product))
        .route(
            "/api/mobile/work-items",
            get(mobile_list_work_items).post(mobile_create_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id",
            get(mobile_get_work_item).post(mobile_update_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/approve",
            post(mobile_approve_work_item),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/workflow/start",
            post(mobile_start_workflow),
        )
        .route(
            "/api/mobile/work-items/:work_item_id/delivery",
            get(mobile_get_work_item_delivery),
        )
        .route("/api/mobile/work-items/:work_item_id/test-results/:artifact_id", get(mobile_get_workflow_test_result))
        .route(
            "/api/mobile/workflows/:workflow_run_id/action",
            post(mobile_handle_workflow_action),
        )
        .route("/api/mobile/workflows/:workflow_run_id/retry-git-delivery", post(mobile_retry_git_delivery))
        .route(
            "/api/mobile/products/:product_id/summary",
            get(mobile_get_product_tree_summary),
        )
        .route(
            "/api/mobile/products/:product_id/tree",
            get(mobile_get_product_tree),
        )
        .route(
            "/api/mobile/planner/sessions",
            post(mobile_create_planner_session),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id",
            post(mobile_update_planner_session),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/validation",
            get(mobile_validate_planner_draft),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/analyze-repository",
            post(mobile_analyze_planner_repository),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/turn",
            post(mobile_submit_planner_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/voice-turn",
            post(mobile_submit_planner_voice_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/confirm",
            post(mobile_confirm_planner_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/clear",
            post(mobile_clear_planner_turn),
        )
        .route(
            "/api/mobile/planner/sessions/:session_id/draft/:action",
            post(mobile_edit_planner_draft),
        )
        .route("/api/mobile/chat/completions", post(mobile_chat_completion))
        .route(
            "/api/mobile/planner-chat/sessions",
            post(mobile_create_planner_chat_session),
        )
        .route(
            "/api/mobile/planner-chat/sessions/:session_id/turn",
            post(mobile_submit_planner_chat_turn),
        )
        .route(
            "/api/mobile/speech/transcribe",
            post(mobile_transcribe_audio),
        )
        .route("/api/mobile/speech/generate", post(mobile_generate_speech))
        .route("/webhooks/twilio/whatsapp", post(twilio_whatsapp))
        .route(
            "/webhooks/twilio/voice",
            get(twilio_voice_entry_with_prompt).post(twilio_voice_entry),
        )
        .route("/webhooks/twilio/voice/gather", post(twilio_voice_gather))
        .with_state(WebhookState { app_state })
}

#[cfg(test)]
mod tests;

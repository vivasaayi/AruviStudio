use crate::persistence::{model_call_repo, model_repo, product_repo, repository_repo, settings_repo};
use crate::services::planner_service::{
    clear_planner_pending, confirm_planner_plan, create_planner_session, submit_planner_turn,
    submit_planner_voice_turn, update_planner_session,
};
use crate::services::product_service::HIDE_EXAMPLE_PRODUCTS_KEY;
use crate::services::speech_service::{
    transcribe_audio_with_provider, SpeechToTextRequest, SpeechToTextResponse,
};
use crate::services::webhook_bridge::ensure_mobile_api_authorized;
use crate::services::webhook_service::WebhookState;
use crate::state::AppState;
use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

const SPEECH_PROVIDER_KEY: &str = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY: &str = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY: &str = "speech.locale";

pub(crate) async fn mobile_ci_read(State(state): State<WebhookState>, headers: HeaderMap, Path(kind): Path<String>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let method = match kind.as_str() {
        "status" => "health",
        "targets" => "targets",
        "runs" => "runs",
        _ => return (StatusCode::NOT_FOUND, "Unknown CI view").into_response(),
    };
    match tokio::task::spawn_blocking(move || crate::commands::ci_commands::request(serde_json::json!({"method": method}))).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => (StatusCode::SERVICE_UNAVAILABLE, error).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CI request failed").into_response(),
    }
}

#[derive(Deserialize)]
pub(crate) struct CiLogQuery { stage: String, offset: Option<u64> }

async fn ci_request(payload: serde_json::Value) -> axum::response::Response {
    match tokio::task::spawn_blocking(move || crate::commands::ci_commands::request(payload)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => (StatusCode::SERVICE_UNAVAILABLE, error).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CI request failed").into_response(),
    }
}

pub(crate) async fn mobile_ci_run(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<i64>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if id <= 0 { return (StatusCode::BAD_REQUEST, "Invalid run ID").into_response(); }
    ci_request(serde_json::json!({"method":"run","id":id})).await
}

pub(crate) async fn mobile_ci_log(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<i64>, Query(query): Query<CiLogQuery>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if id <= 0 || query.stage.trim().is_empty() || query.stage.len() > 256 { return (StatusCode::BAD_REQUEST, "Invalid run ID or stage").into_response(); }
    ci_request(serde_json::json!({"method":"logs","id":id,"stage":query.stage,"offset":query.offset.unwrap_or(0)})).await
}

pub(crate) async fn mobile_ci_run_action(State(state): State<WebhookState>, headers: HeaderMap, Path((id, action)): Path<(i64, String)>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if id <= 0 { return (StatusCode::BAD_REQUEST, "Invalid run ID").into_response(); }
    let payload = match action.as_str() {
        "cancel" => serde_json::json!({"method":"cancel","id":id}),
        "retry" => serde_json::json!({"method":"retry","id":id,"key":format!("studio-retry-{id}-{}", uuid::Uuid::new_v4())}),
        _ => return (StatusCode::NOT_FOUND, "Unknown CI action").into_response(),
    };
    ci_request(payload).await
}

#[derive(Deserialize)]
pub(crate) struct CiFeedbackRequest { verdict: String, #[serde(default)] notes: String }

pub(crate) async fn mobile_ci_feedback_list(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<i64>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if id <= 0 { return (StatusCode::BAD_REQUEST, "Invalid run ID").into_response(); }
    match crate::commands::ci_commands::list_feedback(&state.app_state, id).await {
        Ok(value) => Json(value).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub(crate) async fn mobile_ci_feedback_create(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<i64>, Json(body): Json<CiFeedbackRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::commands::ci_commands::record_verified_feedback(&state.app_state, id, body.verdict, body.notes).await {
        Ok(value) => Json(value).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub(crate) async fn mobile_ci_bindings(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::commands::ci_commands::list_target_bindings(&state.app_state).await {
        Ok(value) => Json(value).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub(crate) async fn mobile_ci_revisions(State(state): State<WebhookState>, headers: HeaderMap, Path((id, kind)): Path<(String, String)>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let action = match kind.as_str() { "branches" => "list_branches", "commits" => "list_commits", _ => return (StatusCode::NOT_FOUND, "Unknown revision type").into_response() };
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_ci", serde_json::json!({"action":action,"arguments":{"repository_id":id}})).await {
        Ok(value) => Json(value["result"].clone()).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_ci_artifact_readiness(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<i64>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if id <= 0 { return (StatusCode::BAD_REQUEST, "Invalid artifact ID").into_response(); }
    ci_request(serde_json::json!({"method":"promotion_readiness","id":id})).await
}

pub(crate) async fn mobile_ci_binding_action(State(state): State<WebhookState>, headers: HeaderMap, Path(action): Path<String>, Json(arguments): Json<serde_json::Value>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let action = match action.as_str() { "save" => "save_binding", "delete" => "delete_binding", "queue" => "queue_build", _ => return (StatusCode::NOT_FOUND, "Unknown CI action").into_response() };
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_ci", serde_json::json!({"action":action,"arguments":arguments})).await {
        Ok(value) => Json(value["result"].clone()).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerSessionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerDraftRequest {
    node_id: String,
    name: Option<String>,
    child_type: Option<String>,
    summary: Option<String>,
}

pub(crate) async fn mobile_edit_planner_draft(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path((session_id, action)): Path<(String, String)>,
    Json(body): Json<MobilePlannerDraftRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let (action, arguments) = match action.as_str() {
        "rename" => ("rename_planner_draft_node", serde_json::json!({"session_id":session_id,"node_id":body.node_id,"name":body.name})),
        "add" => ("add_planner_draft_child", serde_json::json!({"session_id":session_id,"parent_node_id":body.node_id,"name":body.name,"child_type":body.child_type,"summary":body.summary})),
        "delete" => ("delete_planner_draft_node", serde_json::json!({"session_id":session_id,"node_id":body.node_id})),
        _ => return (StatusCode::NOT_FOUND, "Unknown draft action").into_response(),
    };
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_planner", serde_json::json!({"action":action,"arguments":arguments})).await {
        Ok(value) => Json(value["result"].clone()).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelCallListQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileCreateProductRequest {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileCreateProviderRequest {
    name: String,
    base_url: String,
    api_key: Option<String>,
    protocol: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileCreateModelRequest {
    provider_id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileRegisterRepositoryRequest {
    name: String,
    local_path: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileAttachRepositoryRequest {
    product_id: String,
    product_area_id: Option<String>,
    repository_id: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileAgentBindingRequest {
    agent_id: String,
    model_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkflowTestConfig {
    #[serde(default)]
    argv: Vec<String>,
    product_id: Option<String>,
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkflowTestQuery { product_id: Option<String> }

pub(crate) async fn mobile_get_workflow_test_config(State(state): State<WebhookState>, headers: HeaderMap, Query(query): Query<MobileWorkflowTestQuery>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    if let Some(id) = query.product_id {
        if let Err(error) = product_repo::get_product(db, &id).await { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
        return match crate::services::workflow_test_runner::resolve_command(db, &id).await {
            Ok((mode, argv)) => Json(serde_json::json!({"mode":mode,"argv":argv})).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    }
    match settings_repo::get_setting(db, crate::services::workflow_test_runner::TEST_COMMAND_KEY).await {
        Ok(Some(value)) if !value.trim().is_empty() => match crate::services::workflow_test_runner::parse_command(&value) {
            Ok(argv) => Json(serde_json::json!({"argv":argv})).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        },
        Ok(_) => Json(serde_json::json!({"argv":[]})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_set_workflow_test_config(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileWorkflowTestConfig>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    if let Some(id) = body.product_id.as_deref() {
        if let Err(error) = product_repo::get_product(db, id).await { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
        let key = crate::services::workflow_test_runner::product_command_key(id);
        let result = match body.mode.as_deref() {
            Some("inherit") => settings_repo::delete_setting(db, &key).await,
            Some("disabled") => settings_repo::set_setting(db, &key, "[]").await,
            Some("custom") => {
                let value = serde_json::to_string(&body.argv).unwrap_or_default();
                if let Err(error) = crate::services::workflow_test_runner::parse_command(&value) { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
                settings_repo::set_setting(db, &key, &value).await
            },
            _ => return (StatusCode::BAD_REQUEST, "Invalid test command mode").into_response(),
        };
        if let Err(error) = result { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
        return match crate::services::workflow_test_runner::resolve_command(db, id).await {
            Ok((mode, argv)) => Json(serde_json::json!({"mode":mode,"argv":argv})).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    }
    let value = if body.argv.is_empty() { String::new() } else {
        let value = serde_json::to_string(&body.argv).unwrap_or_default();
        if let Err(error) = crate::services::workflow_test_runner::parse_command(&value) { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
        value
    };
    match settings_repo::set_setting(db, crate::services::workflow_test_runner::TEST_COMMAND_KEY, &value).await {
        Ok(()) => Json(serde_json::json!({"argv": body.argv})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn agent_action(state: &WebhookState, action: &str, arguments: serde_json::Value) -> axum::response::Response {
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_agents", serde_json::json!({"action": action, "arguments": arguments})).await {
        Ok(result) => Json(result["result"].clone()).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_agents(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_agent_definitions", serde_json::json!({})).await
}

pub(crate) async fn mobile_list_agent_bindings(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_agent_model_bindings", serde_json::json!({})).await
}

pub(crate) async fn mobile_set_agent_binding(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileAgentBindingRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.agent_id.trim().is_empty() || body.model_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Agent and model are required").into_response();
    }
    agent_action(&state, "set_primary_agent_model_binding", serde_json::json!({"agent_id": body.agent_id.trim(), "model_id": body.model_id.trim()})).await
}

#[derive(Debug, Deserialize, serde::Serialize, PartialEq)]
pub(crate) struct AgentFields {
    name: String, role: String, description: String, prompt_template_ref: String,
    allowed_tools: Vec<String>, skill_tags: Vec<String>, boundaries: serde_json::Value,
    enabled: bool, employment_status: String,
}

impl AgentFields {
    fn validate(&self) -> Result<(), &'static str> {
        if self.name.trim().is_empty() || self.role.trim().is_empty() { return Err("Agent name and role are required"); }
        if !matches!(self.employment_status.as_str(), "active" | "inactive" | "terminated") { return Err("Invalid employment status"); }
        if !self.boundaries.is_object() || serde_json::from_value::<crate::services::agent_execution_limits::AgentExecutionBoundaries>(self.boundaries.clone()).is_err() {
            return Err("Boundaries must be a JSON object with valid execution settings");
        }
        Ok(())
    }
    fn matches(&self, agent: &crate::domain::agent::AgentDefinition) -> bool {
        self.name == agent.name && self.role == agent.role && self.description == agent.description
            && self.prompt_template_ref == agent.prompt_template_ref && self.allowed_tools == agent.allowed_tools
            && self.skill_tags == agent.skill_tags && self.boundaries == agent.boundaries
            && self.enabled == agent.enabled && self.employment_status == agent.employment_status
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentEditRequest { id: String, expected: AgentFields, fields: AgentFields }
#[derive(Debug, Deserialize)]
pub(crate) struct AgentDeleteRequest { id: String, expected: AgentFields }

pub(crate) async fn mobile_create_agent(State(state): State<WebhookState>, headers: HeaderMap, Json(mut body): Json<AgentFields>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if let Err(error) = body.validate() { return (StatusCode::BAD_REQUEST, error).into_response(); }
    body.name = body.name.trim().to_string(); body.role = body.role.trim().to_string();
    agent_action(&state, "create_agent_definition", serde_json::to_value(body).expect("agent fields serialize")).await
}

async fn lock_and_read_agent(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: &str) -> Result<Option<crate::domain::agent::AgentDefinition>, sqlx::Error> {
    let result = sqlx::query("UPDATE agent_definitions SET id=id WHERE id=?").bind(id).execute(&mut **tx).await?;
    if result.rows_affected() == 0 { return Ok(None); }
    sqlx::query("SELECT id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status,created_at,updated_at FROM agent_definitions WHERE id=?")
        .bind(id).map(crate::persistence::agent_definition_repo::row_to_agent_definition).fetch_optional(&mut **tx).await
}

pub(crate) async fn mobile_update_agent(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<AgentEditRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if let Err(error) = body.fields.validate() { return (StatusCode::BAD_REQUEST, error).into_response(); }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    let current = match lock_and_read_agent(&mut tx, &body.id).await { Ok(current) => current, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    if !current.as_ref().is_some_and(|agent| body.expected.matches(agent)) { return (StatusCode::CONFLICT, "Agent changed or was removed. Refresh before saving.").into_response(); }
    let tools = serde_json::to_string(&body.fields.allowed_tools).expect("tools serialize");
    let tags = serde_json::to_string(&body.fields.skill_tags).expect("tags serialize");
    let boundaries = serde_json::to_string(&body.fields.boundaries).expect("boundaries serialize");
    let result = sqlx::query("UPDATE agent_definitions SET name=?,role=?,description=?,prompt_template_ref=?,allowed_tools=?,skill_tags=?,boundaries=?,enabled=?,employment_status=?,updated_at=datetime('now') WHERE id=?")
        .bind(body.fields.name.trim()).bind(body.fields.role.trim()).bind(&body.fields.description).bind(&body.fields.prompt_template_ref)
        .bind(tools).bind(tags).bind(boundaries).bind(body.fields.enabled).bind(&body.fields.employment_status).bind(&body.id).execute(&mut *tx).await;
    if let Err(error) = result { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
    if let Err(error) = tx.commit().await { return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(); }
    match crate::persistence::agent_repo::get_agent_definition(&state.app_state.db, &body.id).await {
        Ok(agent) => Json(agent).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_delete_agent(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<AgentDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    let current = match lock_and_read_agent(&mut tx, &body.id).await { Ok(current) => current, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    if !current.as_ref().is_some_and(|agent| body.expected.matches(agent)) { return (StatusCode::CONFLICT, "Agent changed or was removed. Refresh before deleting.").into_response(); }
    if let Err(error) = sqlx::query("DELETE FROM agent_definitions WHERE id=?").bind(&body.id).execute(&mut *tx).await {
        if error.as_database_error().is_some_and(|error| error.is_foreign_key_violation()) {
            return (StatusCode::CONFLICT, "Agent has execution history and cannot be deleted. Disable it or mark it inactive instead.").into_response();
        }
        return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
    }
    match tx.commit().await { Ok(()) => Json(serde_json::json!({"deleted":true})).into_response(), Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamFields {
    name: String,
    department: String,
    description: String,
    enabled: bool,
    max_concurrent_workflows: i32,
}

impl TeamFields {
    fn valid(&self) -> bool {
        !self.name.trim().is_empty() && self.name.len() <= 200 && self.department.len() <= 200
            && self.description.len() <= 10000 && self.max_concurrent_workflows >= 1
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamEditRequest { id: String, expected: TeamFields, fields: TeamFields }
#[derive(Debug, Deserialize)]
pub(crate) struct TeamDeleteRequest { id: String, expected: TeamFields }
#[derive(Debug, Deserialize)]
pub(crate) struct TeamMemberRequest { team_id: String, agent_id: String, title: String, is_lead: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct TeamMemberDeleteRequest { id: String, team_id: String, agent_id: String }

pub(crate) async fn mobile_list_teams(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_agent_teams", serde_json::json!({})).await
}

pub(crate) async fn mobile_create_team(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamFields>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if !body.valid() { return (StatusCode::BAD_REQUEST, "Team name and a positive capacity are required").into_response(); }
    agent_action(&state, "create_agent_team", serde_json::json!({"name": body.name.trim(), "department":body.department, "description":body.description, "enabled":body.enabled, "max_concurrent_workflows":body.max_concurrent_workflows})).await
}

pub(crate) async fn mobile_update_team(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamEditRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if !body.fields.valid() { return (StatusCode::BAD_REQUEST, "Team name and a positive capacity are required").into_response(); }
    let result = sqlx::query("UPDATE agent_teams SET name=?,department=?,description=?,enabled=?,max_concurrent_workflows=?,updated_at=datetime('now') WHERE id=? AND name=? AND department=? AND description=? AND enabled=? AND max_concurrent_workflows=?")
        .bind(body.fields.name.trim()).bind(&body.fields.department).bind(&body.fields.description).bind(body.fields.enabled).bind(body.fields.max_concurrent_workflows)
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.department).bind(&body.expected.description).bind(body.expected.enabled).bind(body.expected.max_concurrent_workflows)
        .execute(&state.app_state.db).await;
    match result {
        Ok(result) if result.rows_affected() == 1 => match crate::persistence::agent_repo::get_agent_team(&state.app_state.db, &body.id).await {
            Ok(team) => Json(team).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        },
        Ok(_) => (StatusCode::CONFLICT, "Team changed or was removed. Refresh before saving.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_delete_team(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let result = sqlx::query("DELETE FROM agent_teams WHERE id=? AND name=? AND department=? AND description=? AND enabled=? AND max_concurrent_workflows=?")
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.department).bind(&body.expected.description).bind(body.expected.enabled).bind(body.expected.max_concurrent_workflows)
        .execute(&state.app_state.db).await;
    match result {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Team changed or was removed. Refresh before deleting.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_team_members(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_team_memberships", serde_json::json!({})).await
}

pub(crate) async fn mobile_add_team_member(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamMemberRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.team_id.trim().is_empty() || body.agent_id.trim().is_empty() || body.title.len() > 200 { return (StatusCode::BAD_REQUEST, "Team, agent and a valid title are required").into_response(); }
    agent_action(&state, "add_team_member", serde_json::json!({"team_id":body.team_id,"agent_id":body.agent_id,"title":body.title,"is_lead":body.is_lead})).await
}

pub(crate) async fn mobile_remove_team_member(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamMemberDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match sqlx::query("DELETE FROM agent_team_memberships WHERE id=? AND team_id=? AND agent_id=?").bind(&body.id).bind(&body.team_id).bind(&body.agent_id).execute(&state.app_state.db).await {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Membership changed or was removed. Refresh the roster.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
struct TeamAssignmentView {
    id: String,
    team_id: String,
    scope_type: String,
    scope_id: String,
    created_at: String,
    scope_name: Option<String>,
    product_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamAssignmentRequest { team_id: String, product_id: String, scope_type: String, scope_id: String }
#[derive(Debug, Deserialize)]
pub(crate) struct TeamAssignmentDeleteRequest { id: String, team_id: String, scope_type: String, scope_id: String }

pub(crate) async fn mobile_list_team_assignments(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let result = sqlx::query_as::<_, TeamAssignmentView>(
        "SELECT ta.id,ta.team_id,ta.scope_type,ta.scope_id,ta.created_at, COALESCE(p.name,pa.name,c.name) AS scope_name, COALESCE(p.name,ap.name,cp.name) AS product_name FROM team_assignments ta LEFT JOIN products p ON ta.scope_type='product' AND p.id=ta.scope_id LEFT JOIN product_areas pa ON ta.scope_type='product_area' AND pa.id=ta.scope_id LEFT JOIN products ap ON ap.id=pa.product_id LEFT JOIN capabilities c ON ta.scope_type='capability' AND c.id=ta.scope_id LEFT JOIN product_areas ca ON ca.id=c.product_area_id LEFT JOIN products cp ON cp.id=ca.product_id ORDER BY ta.created_at,ta.rowid"
    ).fetch_all(&state.app_state.db).await;
    match result { Ok(assignments) => Json(assignments).into_response(), Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() }
}

pub(crate) async fn mobile_add_team_assignment(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamAssignmentRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let scope_check = match body.scope_type.as_str() {
        "product" => "SELECT 1 FROM products WHERE id=? AND id=?",
        "product_area" => "SELECT 1 FROM product_areas WHERE id=? AND product_id=?",
        "capability" => "SELECT 1 FROM capabilities c JOIN product_areas a ON a.id=c.product_area_id WHERE c.id=? AND a.product_id=?",
        _ => return (StatusCode::BAD_REQUEST, "Unsupported assignment scope").into_response(),
    };
    // The existence test and insert are a single statement, so a concurrent hierarchy deletion
    // cannot leave an assignment pointing at an already removed scope.
    let sql = format!("INSERT INTO team_assignments (id,team_id,scope_type,scope_id) SELECT ?,?,?,? WHERE EXISTS ({scope_check}) AND EXISTS (SELECT 1 FROM agent_teams WHERE id=?)");
    let id = uuid::Uuid::new_v4().to_string();
    match sqlx::query(&sql).bind(&id).bind(&body.team_id).bind(&body.scope_type).bind(&body.scope_id).bind(&body.scope_id).bind(&body.product_id).bind(&body.team_id).execute(&state.app_state.db).await {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"id":id})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Team or scope changed or was removed. Refresh and select it again.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_remove_team_assignment(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<TeamAssignmentDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match sqlx::query("DELETE FROM team_assignments WHERE id=? AND team_id=? AND scope_type=? AND scope_id=?")
        .bind(&body.id).bind(&body.team_id).bind(&body.scope_type).bind(&body.scope_id).execute(&state.app_state.db).await {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Assignment changed or was removed. Refresh before removing it.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillFields { name: String, category: String, description: String, instructions: String, enabled: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct SkillEditRequest { id: String, expected: SkillFields, fields: SkillFields }
#[derive(Debug, Deserialize)]
pub(crate) struct SkillDeleteRequest { id: String, expected: SkillFields }
#[derive(Debug, Deserialize)]
pub(crate) struct SkillLinkRequest { skill_id: String, target_type: String, target_id: String, proficiency: Option<String> }
#[derive(Debug, Deserialize)]
pub(crate) struct SkillUnlinkRequest { id: String, skill_id: String, target_type: String, target_id: String }

pub(crate) async fn mobile_list_skills(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_skills", serde_json::json!({})).await
}

pub(crate) async fn mobile_create_skill(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<SkillFields>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.name.trim().is_empty() { return (StatusCode::BAD_REQUEST, "Skill name is required").into_response(); }
    agent_action(&state, "create_skill", serde_json::json!({"name":body.name.trim(),"category":body.category,"description":body.description,"instructions":body.instructions,"enabled":body.enabled})).await
}

pub(crate) async fn mobile_update_skill(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<SkillEditRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.fields.name.trim().is_empty() { return (StatusCode::BAD_REQUEST, "Skill name is required").into_response(); }
    let result = sqlx::query("UPDATE skills SET name=?,category=?,description=?,instructions=?,enabled=?,updated_at=datetime('now') WHERE id=? AND name=? AND category=? AND description=? AND instructions=? AND enabled=?")
        .bind(body.fields.name.trim()).bind(&body.fields.category).bind(&body.fields.description).bind(&body.fields.instructions).bind(body.fields.enabled)
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.category).bind(&body.expected.description).bind(&body.expected.instructions).bind(body.expected.enabled)
        .execute(&state.app_state.db).await;
    match result {
        Ok(result) if result.rows_affected() == 1 => match crate::persistence::agent_skill_repo::get_skill(&state.app_state.db, &body.id).await {
            Ok(skill) => Json(skill).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        },
        Ok(_) => (StatusCode::CONFLICT, "Skill changed or was removed. Refresh before saving.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_delete_skill(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<SkillDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let result = sqlx::query("DELETE FROM skills WHERE id=? AND name=? AND category=? AND description=? AND instructions=? AND enabled=?")
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.category).bind(&body.expected.description).bind(&body.expected.instructions).bind(body.expected.enabled)
        .execute(&state.app_state.db).await;
    match result {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Skill changed or was removed. Refresh before deleting.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_skill_links(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let (agents, teams) = tokio::join!(crate::persistence::agent_repo::list_agent_skill_links(&state.app_state.db), crate::persistence::agent_repo::list_team_skill_links(&state.app_state.db));
    match (agents, teams) {
        (Ok(agents), Ok(teams)) => Json(serde_json::json!({"agents":agents,"teams":teams})).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to load skill links").into_response(),
    }
}

pub(crate) async fn mobile_add_skill_link(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<SkillLinkRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let id = uuid::Uuid::new_v4().to_string();
    let result = match body.target_type.as_str() {
        "agent" => {
            let proficiency = body.proficiency.as_deref().unwrap_or("working");
            if !matches!(proficiency, "learning" | "working" | "expert") { return (StatusCode::BAD_REQUEST, "Invalid proficiency").into_response(); }
            sqlx::query("INSERT INTO agent_skill_links (id,agent_id,skill_id,proficiency) VALUES (?,?,?,?)")
                .bind(&id).bind(&body.target_id).bind(&body.skill_id).bind(proficiency).execute(&state.app_state.db).await
        },
        "team" => sqlx::query("INSERT INTO team_skill_links (id,team_id,skill_id) VALUES (?,?,?)")
            .bind(&id).bind(&body.target_id).bind(&body.skill_id).execute(&state.app_state.db).await,
        _ => return (StatusCode::BAD_REQUEST, "Invalid link target type").into_response(),
    };
    match result { Ok(_) => Json(serde_json::json!({"id":id})).into_response(), Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
}

pub(crate) async fn mobile_remove_skill_link(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<SkillUnlinkRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let sql = match body.target_type.as_str() {
        "agent" => "DELETE FROM agent_skill_links WHERE id=? AND skill_id=? AND agent_id=?",
        "team" => "DELETE FROM team_skill_links WHERE id=? AND skill_id=? AND team_id=?",
        _ => return (StatusCode::BAD_REQUEST, "Invalid link target type").into_response(),
    };
    match sqlx::query(sql).bind(&body.id).bind(&body.skill_id).bind(&body.target_id).execute(&state.app_state.db).await {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Skill link changed or was removed. Refresh before removing it.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoutingFields { primary_roles: Vec<String>, fallback_roles: Vec<String>, coordinator_required: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct RoutingSnapshot { id: String, primary_roles: Vec<String>, fallback_roles: Vec<String>, coordinator_required: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct RoutingSaveRequest { stage_name: String, expected: Option<RoutingSnapshot>, fields: RoutingFields }
#[derive(Debug, Deserialize)]
pub(crate) struct RoutingDeleteRequest { stage_name: String, expected: RoutingSnapshot }

fn supported_routing_stage(stage: &str) -> bool {
    matches!(stage, "requirement_analysis" | "planning" | "coding" | "unit_test_generation" | "integration_test_generation" | "ui_test_planning" | "qa_validation" | "security_review" | "performance_review" | "push_preparation" | "git_push")
}

impl RoutingSnapshot {
    fn matches(&self, policy: &crate::domain::agent::WorkflowStagePolicy) -> bool {
        self.id == policy.id && self.primary_roles == policy.primary_roles && self.fallback_roles == policy.fallback_roles && self.coordinator_required == policy.coordinator_required
    }
}

async fn lock_and_read_policy(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, stage: &str) -> Result<Option<crate::domain::agent::WorkflowStagePolicy>, sqlx::Error> {
    sqlx::query("UPDATE workflow_stage_policies SET stage_name=stage_name WHERE stage_name=?").bind(stage).execute(&mut **tx).await?;
    sqlx::query("SELECT id,stage_name,primary_roles,fallback_roles,coordinator_required,created_at,updated_at FROM workflow_stage_policies WHERE stage_name=?")
        .bind(stage).try_map(crate::persistence::agent_repo::row_to_workflow_stage_policy).fetch_optional(&mut **tx).await
}

pub(crate) async fn mobile_list_routing_policies(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    agent_action(&state, "list_workflow_stage_policies", serde_json::json!({})).await
}

pub(crate) async fn mobile_save_routing_policy(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<RoutingSaveRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if !supported_routing_stage(&body.stage_name) { return (StatusCode::BAD_REQUEST, "Unsupported workflow stage").into_response(); }
    if body.fields.primary_roles.iter().chain(body.fields.fallback_roles.iter()).any(|role| role.trim().is_empty()) {
        return (StatusCode::BAD_REQUEST, "Role names cannot be blank").into_response();
    }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    let current = match lock_and_read_policy(&mut tx, &body.stage_name).await { Ok(current) => current, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    let matches = match (&body.expected, &current) { (None, None) => true, (Some(expected), Some(current)) => expected.matches(current), _ => false };
    if !matches { return (StatusCode::CONFLICT, "Routing policy changed. Refresh before saving.").into_response(); }
    let primary: Vec<&str> = body.fields.primary_roles.iter().map(|role| role.trim()).collect();
    let fallback: Vec<&str> = body.fields.fallback_roles.iter().map(|role| role.trim()).collect();
    let primary = serde_json::to_string(&primary).expect("roles serialize");
    let fallback = serde_json::to_string(&fallback).expect("roles serialize");
    let result = match current {
        Some(policy) => sqlx::query("UPDATE workflow_stage_policies SET primary_roles=?,fallback_roles=?,coordinator_required=?,updated_at=datetime('now') WHERE id=?")
            .bind(primary).bind(fallback).bind(body.fields.coordinator_required).bind(policy.id).execute(&mut *tx).await,
        None => sqlx::query("INSERT INTO workflow_stage_policies (id,stage_name,primary_roles,fallback_roles,coordinator_required) VALUES (?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&body.stage_name).bind(primary).bind(fallback).bind(body.fields.coordinator_required).execute(&mut *tx).await,
    };
    if let Err(error) = result { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
    if let Err(error) = tx.commit().await { return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(); }
    match crate::persistence::agent_repo::get_workflow_stage_policy(&state.app_state.db, &body.stage_name).await {
        Ok(Some(policy)) => Json(policy).into_response(), _ => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to load saved policy").into_response(),
    }
}

pub(crate) async fn mobile_delete_routing_policy(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<RoutingDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if !supported_routing_stage(&body.stage_name) { return (StatusCode::BAD_REQUEST, "Unsupported workflow stage").into_response(); }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    let current = match lock_and_read_policy(&mut tx, &body.stage_name).await { Ok(current) => current, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() };
    if !current.as_ref().is_some_and(|policy| body.expected.matches(policy)) { return (StatusCode::CONFLICT, "Routing policy changed. Refresh before resetting.").into_response(); }
    if let Err(error) = sqlx::query("DELETE FROM workflow_stage_policies WHERE stage_name=?").bind(&body.stage_name).execute(&mut *tx).await { return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(); }
    match tx.commit().await { Ok(()) => Json(serde_json::json!({"deleted":true})).into_response(), Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response() }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileRepositoryInspectRequest {
    action: String,
    repository_id: String,
    relative_path: Option<String>,
}

pub(crate) async fn mobile_inspect_repository(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileRepositoryInspectRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if !matches!(body.action.as_str(), "list_repository_tree" | "read_repository_file" | "get_git_status" | "get_git_diff") {
        return (StatusCode::BAD_REQUEST, "Unsupported repository inspection action").into_response();
    }
    let arguments = serde_json::json!({"repository_id":body.repository_id, "relative_path":body.relative_path, "max_depth":8});
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_repositories", serde_json::json!({"action":body.action,"arguments":arguments})).await {
        Ok(value) => Json(value["result"].clone()).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileRepositorySaveRequest {
    repository_id: String,
    relative_path: String,
    expected_content: String,
    content: String,
}

pub(crate) async fn mobile_save_repository_file(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileRepositorySaveRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.expected_content.len() > 2 * 1024 * 1024 || body.content.len() > 2 * 1024 * 1024 {
        return (StatusCode::BAD_REQUEST, "File content exceeds the 2 MiB limit").into_response();
    }
    static SAVE_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _guard = SAVE_LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    let repository = match repository_repo::get_repository(&state.app_state.db, &body.repository_id).await {
        Ok(repository) => repository, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let current = match crate::services::repo_service::read_repository_file(&repository.local_path, &body.relative_path) {
        Ok(content) => content, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if current != body.expected_content {
        return (StatusCode::CONFLICT, "File changed since it was opened. Copy or discard your edits, then reopen the file.").into_response();
    }
    match crate::services::repo_service::write_repository_file(&repository.local_path, &body.relative_path, &body.content) {
        Ok(()) => Json(serde_json::json!({"ok":true})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_repositories(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match repository_repo::list_repositories(&state.app_state.db).await {
        Ok(repositories) => Json(repositories).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_register_repository(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileRegisterRepositoryRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let path = std::path::Path::new(body.local_path.trim());
    if body.name.trim().is_empty() || !path.is_absolute() {
        return (StatusCode::BAD_REQUEST, "A name and absolute path to an accessible Git repository are required").into_response();
    }
    let branch = match git2::Repository::open(path) {
        Ok(repository) if !repository.is_bare() => repository.head().ok().filter(|head| head.is_branch()).and_then(|head| head.shorthand().map(str::to_owned)).unwrap_or_else(|| "main".into()),
        _ => return (StatusCode::BAD_REQUEST, "A name and absolute path to an accessible Git working repository are required").into_response(),
    };
    match crate::commands::repository_commands::register_repository_for_state(
        &state.app_state, body.name.trim().to_string(), body.local_path.trim().to_string(), String::new(), branch
    ).await {
        Ok(repository) => (StatusCode::CREATED, Json(repository)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_repository_attachments(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match repository_repo::list_all_repository_attachments(&state.app_state.db).await {
        Ok(attachments) => Json(attachments).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_product_areas(State(state): State<WebhookState>, headers: HeaderMap, Path(product_id): Path<String>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if let Err(error) = product_repo::get_product(&state.app_state.db, &product_id).await { return (StatusCode::BAD_REQUEST, error.to_string()).into_response(); }
    match product_repo::list_product_areas(&state.app_state.db, &product_id).await {
        Ok(areas) => Json(areas).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn mobile_repository_scope<'a>(db: &sqlx::SqlitePool, body: &'a MobileAttachRepositoryRequest) -> Result<(&'static str, &'a str), crate::error::AppError> {
    product_repo::get_product(db, &body.product_id).await?;
    if let Some(area_id) = body.product_area_id.as_deref() {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM product_areas WHERE id=? AND product_id=?)")
            .bind(area_id).bind(&body.product_id).fetch_one(db).await?;
        if !exists { return Err(crate::error::AppError::Validation("Product area does not belong to this product".into())); }
        Ok(("product_area", area_id))
    } else {
        Ok(("product", &body.product_id))
    }
}

pub(crate) async fn mobile_attach_repository(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileAttachRepositoryRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    let (scope_type, scope_id) = match mobile_repository_scope(db, &body).await { Ok(scope) => scope, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() };
    if repository_repo::get_repository(db, &body.repository_id).await.is_err() { return (StatusCode::BAD_REQUEST, "Repository not found").into_response(); }
    match repository_repo::attach_repository(db, &uuid::Uuid::new_v4().to_string(), scope_type, scope_id, &body.repository_id, body.is_default).await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_set_default_repository(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileAttachRepositoryRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    let (scope_type, scope_id) = match mobile_repository_scope(db, &body).await { Ok(scope) => scope, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() };
    match repository_repo::set_default_repository(db, scope_type, scope_id, &body.repository_id).await {
        Ok(()) => Json(serde_json::json!({"ok":true})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_detach_repository(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileAttachRepositoryRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    let (scope_type, scope_id) = match mobile_repository_scope(db, &body).await { Ok(scope) => scope, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() };
    match repository_repo::detach_repository(db, scope_type, scope_id, &body.repository_id).await {
        Ok(()) => Json(serde_json::json!({"ok":true})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn model_action(state: &WebhookState, action: &str, arguments: serde_json::Value) -> axum::response::Response {
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_models", serde_json::json!({"action": action, "arguments": arguments})).await {
        Ok(result) => {
            let mut value = result["result"].clone();
            if action == "list_providers" {
                if let Some(providers) = value.as_array_mut() {
                    for provider in providers { if let Some(fields) = provider.as_object_mut() { fields.remove("auth_secret_ref"); } }
                }
            } else if action == "create_provider" {
                if let Some(fields) = value.as_object_mut() { fields.remove("auth_secret_ref"); }
            }
            Json(value).into_response()
        },
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderEditFields { name: String, base_url: String, enabled: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct ProviderEditRequest { id: String, expected: ProviderEditFields, fields: ProviderEditFields }
#[derive(Debug, Deserialize)]
pub(crate) struct ModelEditFields { name: String, provider_id: String, context_window: Option<i64>, notes: String, enabled: bool }
#[derive(Debug, Deserialize)]
pub(crate) struct ModelEditRequest { id: String, expected: ModelEditFields, fields: ModelEditFields }
#[derive(Debug, Deserialize)]
pub(crate) struct ModelDeleteRequest { id: String, expected_name: String }

pub(crate) async fn mobile_update_provider(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ProviderEditRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let f = &body.fields; let e = &body.expected;
    let url = reqwest::Url::parse(f.base_url.trim());
    if f.name.trim().is_empty() || !url.as_ref().is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some() && u.username().is_empty() && u.password().is_none()) {
        return (StatusCode::BAD_REQUEST, "A name and valid HTTP(S) base URL are required").into_response();
    }
    let result = sqlx::query("UPDATE model_providers SET name=?,base_url=?,enabled=?,updated_at=datetime('now') WHERE id=? AND name=? AND base_url=? AND enabled=?")
        .bind(f.name.trim()).bind(f.base_url.trim()).bind(f.enabled).bind(&body.id).bind(&e.name).bind(&e.base_url).bind(e.enabled).execute(&state.app_state.db).await;
    match result { Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Provider changed or was removed. Refresh and review your edits.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderCredentialRequest { id: String, expected: ProviderEditFields, api_key: String }

pub(crate) async fn mobile_rotate_provider_credential(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ProviderCredentialRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.api_key.trim().is_empty() || body.api_key.trim().len() > 16 * 1024 { return (StatusCode::BAD_REQUEST, "API key must contain 1 to 16384 bytes").into_response(); }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() };
    let locked = sqlx::query("UPDATE model_providers SET name=name WHERE id=? AND name=? AND base_url=? AND enabled=?")
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.base_url).bind(body.expected.enabled).execute(&mut *tx).await;
    match locked {
        Ok(result) if result.rows_affected() == 1 => {},
        Ok(_) => return (StatusCode::CONFLICT, "Provider changed or was removed. Refresh before rotating its key.").into_response(),
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
    let reference = match crate::secrets::store_rotated_provider_secret(&body.id, &body.api_key) {
        Ok(reference) => reference,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Unable to store the provider key").into_response(),
    };
    if let Err(error) = sqlx::query("UPDATE model_providers SET auth_secret_ref=?,updated_at=datetime('now') WHERE id=?")
        .bind(reference).bind(&body.id).execute(&mut *tx).await { return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(); }
    match tx.commit().await {
        Ok(()) => Json(serde_json::json!({"ok":true})).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderCleanupRequest { id: String, expected: ProviderEditFields }

pub(crate) async fn mobile_cleanup_provider_credentials(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ProviderCleanupRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let mut tx = match state.app_state.db.begin().await { Ok(tx) => tx, Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() };
    let locked = sqlx::query("UPDATE model_providers SET name=name WHERE id=? AND name=? AND base_url=? AND enabled=?")
        .bind(&body.id).bind(&body.expected.name).bind(&body.expected.base_url).bind(body.expected.enabled).execute(&mut *tx).await;
    match locked { Ok(result) if result.rows_affected() == 1 => {},
        Ok(_) => return (StatusCode::CONFLICT, "Provider changed or was removed. Refresh before cleanup.").into_response(),
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
    let refs: Vec<String> = match sqlx::query_scalar("SELECT auth_secret_ref FROM model_providers WHERE auth_secret_ref IS NOT NULL").fetch_all(&mut *tx).await {
        Ok(refs) => refs, Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    };
    let protected = refs.into_iter().map(|value| value.strip_prefix("ref:").unwrap_or(&value).to_owned()).collect();
    let removed = match crate::secrets::cleanup_unreferenced_rotated_fallback_keys(&body.id, &protected) {
        Ok(count) => count, Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Unable to clean stored provider keys").into_response(),
    };
    match tx.commit().await {
        Ok(()) => Json(serde_json::json!({"removed":removed})).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_update_model(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ModelEditRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let f = &body.fields; let e = &body.expected;
    if f.name.trim().is_empty() || f.provider_id.trim().is_empty() || f.context_window.is_some_and(|v| v <= 0) {
        return (StatusCode::BAD_REQUEST, "A name, provider and positive context window are required").into_response();
    }
    let result = sqlx::query("UPDATE model_definitions SET name=?,provider_id=?,context_window=?,notes=?,enabled=?,updated_at=datetime('now') WHERE id=? AND name=? AND provider_id=? AND context_window IS ? AND notes=? AND enabled=?")
        .bind(f.name.trim()).bind(&f.provider_id).bind(f.context_window).bind(&f.notes).bind(f.enabled)
        .bind(&body.id).bind(&e.name).bind(&e.provider_id).bind(e.context_window).bind(&e.notes).bind(e.enabled).execute(&state.app_state.db).await;
    match result { Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Model changed or was removed. Refresh and review your edits.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
}

pub(crate) async fn mobile_delete_provider(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ModelDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let result = sqlx::query("DELETE FROM model_providers WHERE id=? AND name=? AND NOT EXISTS(SELECT 1 FROM model_definitions WHERE provider_id=?)")
        .bind(&body.id).bind(&body.expected_name).bind(&body.id).execute(&state.app_state.db).await;
    match result { Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Provider changed, was removed, or still has models. Remove its models first.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
}

pub(crate) async fn mobile_delete_model(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<ModelDeleteRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let result = sqlx::query("DELETE FROM model_definitions WHERE id=? AND name=?").bind(&body.id).bind(&body.expected_name).execute(&state.app_state.db).await;
    match result { Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Model changed or was removed. Refresh before deleting.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response() }
}

pub(crate) async fn mobile_list_providers(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    model_action(&state, "list_providers", serde_json::json!({})).await
}

pub(crate) async fn mobile_create_provider(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileCreateProviderRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let name = body.name.trim();
    let url = reqwest::Url::parse(body.base_url.trim());
    if name.is_empty() || !url.as_ref().is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none()) {
        return (StatusCode::BAD_REQUEST, "A name and valid HTTP(S) base URL are required").into_response();
    }
    model_action(&state, "create_provider", serde_json::json!({"name": name, "provider_type": "openai_compatible", "base_url": body.base_url.trim(), "auth_secret_ref": body.api_key, "protocol": body.protocol.unwrap_or_else(|| "openai_compatible".into())})).await
}

pub(crate) async fn mobile_list_models(State(state): State<WebhookState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    model_action(&state, "list_model_definitions", serde_json::json!({})).await
}

pub(crate) async fn mobile_create_model(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileCreateModelRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.name.trim().is_empty() || body.provider_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Provider and model name are required").into_response();
    }
    model_action(&state, "create_model_definition", serde_json::json!({"name": body.name.trim(), "provider_id": body.provider_id.trim()})).await
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileDiscoverModelsRequest { id: String, cursor: Option<String> }

pub(crate) async fn mobile_discover_provider_models(State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileDiscoverModelsRequest>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    model_action(&state, "discover_provider_models", serde_json::json!({"id": body.id, "cursor": body.cursor})).await
}

pub(crate) async fn mobile_test_provider(State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    model_action(&state, "test_provider_connectivity", serde_json::json!({"id": id})).await
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerUpdateRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobilePlannerTurnRequest {
    user_input: String,
    selected_draft_node_id: Option<String>,
    #[serde(alias = "productId")]
    product_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileSpeechTranscriptionRequest {
    provider_id: Option<String>,
    model_name: Option<String>,
    audio_bytes_base64: String,
    mime_type: String,
    locale: Option<String>,
}

pub(crate) async fn mobile_healthcheck(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    Json(serde_json::json!({
        "status": "ok",
        "service": "aruvi-mobile-api",
    }))
    .into_response()
}

pub(crate) async fn mobile_list_model_calls(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Query(query): Query<ModelCallListQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match model_call_repo::list_model_calls(&state.app_state.db, query.limit.unwrap_or(100)).await {
        Ok(calls) => Json(calls).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_model_call_snapshot(State(state): State<WebhookState>, headers: HeaderMap, Path((id, kind)): Path<(String, String)>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::commands::model_commands::read_snapshot_for_state(&state.app_state, &id, &kind, Some(1024 * 1024)).await {
        Ok(content) => Json(serde_json::json!({"content":content})).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_get_model_call(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(call_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match model_call_repo::get_model_call(&state.app_state.db, &call_id).await {
        Ok(call) => Json(call).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_list_products(
    State(state): State<WebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let hide_examples =
        match settings_repo::get_bool_setting(&state.app_state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true)
            .await
        {
            Ok(value) => value,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
    let mut products = match product_repo::list_products(&state.app_state.db).await {
        Ok(products) => products,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if hide_examples {
        products.retain(|product| !product.is_example_product());
    }
    Json(products).into_response()
}

pub(crate) async fn mobile_create_product(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileCreateProductRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let name = body.name.trim();
    if name.is_empty() {
        return (StatusCode::BAD_REQUEST, "Product name is required").into_response();
    }
    let payload = serde_json::json!({"action": "create_product", "arguments": {
        "name": name, "description": body.description.trim()
    }});
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_catalog", payload).await {
        Ok(result) => (StatusCode::CREATED, Json(result["result"].clone())).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileCreateHierarchyNode {
    name: String,
    description: Option<String>,
    product_area_id: Option<String>,
    parent_capability_id: Option<String>,
}

pub(crate) async fn mobile_create_hierarchy_node(State(state): State<WebhookState>, headers: HeaderMap, Path(product_id): Path<String>, Json(body): Json<MobileCreateHierarchyNode>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let db = &state.app_state.db;
    if body.name.trim().is_empty() || body.name.len() > 200 { return (StatusCode::BAD_REQUEST, "Name must contain 1 to 200 characters").into_response(); }
    if product_repo::get_product(db, &product_id).await.is_err() { return (StatusCode::BAD_REQUEST, "Product not found").into_response(); }
    let action = if let Some(area_id) = &body.product_area_id {
        let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM product_areas WHERE id=? AND product_id=?)").bind(area_id).bind(&product_id).fetch_one(db).await;
        if !matches!(exists, Ok(true)) { return (StatusCode::BAD_REQUEST, "Product area does not belong to this product").into_response(); }
        if let Some(parent_id) = &body.parent_capability_id {
            let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM capabilities WHERE id=? AND product_area_id=?)").bind(parent_id).bind(area_id).fetch_one(db).await;
            if !matches!(exists, Ok(true)) { return (StatusCode::BAD_REQUEST, "Parent does not belong to this product area").into_response(); }
        }
        "create_capability"
    } else {
        if body.parent_capability_id.is_some() { return (StatusCode::BAD_REQUEST, "Product area is required for a child").into_response(); }
        "create_product_area"
    };
    let arguments = serde_json::json!({"product_id": product_id, "product_area_id":body.product_area_id, "parent_capability_id":body.parent_capability_id, "name":body.name.trim(), "description":body.description.unwrap_or_default()});
    match crate::mcp::dispatch_tool(&state.app_state, "aruvi_catalog", serde_json::json!({"action":action,"arguments":arguments})).await {
        Ok(result) => (StatusCode::CREATED, Json(result["result"].clone())).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileUpdateHierarchyNode {
    node_type: String,
    id: String,
    expected_name: String,
    expected_description: String,
    name: String,
    description: String,
}

pub(crate) async fn mobile_update_hierarchy_node(State(state): State<WebhookState>, headers: HeaderMap, Path(product_id): Path<String>, Json(body): Json<MobileUpdateHierarchyNode>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.name.trim().is_empty() || body.name.len() > 200 { return (StatusCode::BAD_REQUEST, "Name must contain 1 to 200 characters").into_response(); }
    let sql = match body.node_type.as_str() {
        "product_area" => "UPDATE product_areas SET name=?,description=?,updated_at=datetime('now') WHERE id=? AND product_id=? AND name=? AND description=?",
        "capability" => "UPDATE capabilities SET name=?,description=?,updated_at=datetime('now') WHERE id=? AND product_area_id IN (SELECT id FROM product_areas WHERE product_id=?) AND name=? AND description=?",
        _ => return (StatusCode::BAD_REQUEST, "Unsupported hierarchy node type").into_response(),
    };
    match sqlx::query(sql).bind(body.name.trim()).bind(&body.description).bind(&body.id).bind(&product_id).bind(&body.expected_name).bind(&body.expected_description).execute(&state.app_state.db).await {
        Ok(result) if result.rows_affected() == 1 => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(_) => (StatusCode::CONFLICT, "Node changed, was removed, or belongs to another product. Refresh and review your edits.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileDeleteHierarchyNode { node_type: String, id: String, expected_name: String }

pub(crate) async fn mobile_delete_hierarchy_node(State(state): State<WebhookState>, headers: HeaderMap, Path(product_id): Path<String>, Json(body): Json<MobileDeleteHierarchyNode>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::persistence::hierarchy_delete_repo::delete_hierarchy_node(&state.app_state.db, &body.node_type, &body.id, Some((&product_id, &body.expected_name))).await {
        Ok(true) => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(false) => (StatusCode::CONFLICT, "Node changed, was removed, or belongs to another product. Refresh before deleting.").into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileResetProductPlan { expected_name: String, delete_delivery: bool }

pub(crate) async fn mobile_reset_product_plan(State(state): State<WebhookState>, headers: HeaderMap, Path(product_id): Path<String>, Json(body): Json<MobileResetProductPlan>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match product_repo::reset_product_plan_checked(&state.app_state.db, &product_id, body.delete_delivery, Some(&body.expected_name)).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_get_product_tree(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match product_repo::get_product_tree(&state.app_state.db, &product_id).await {
        Ok(tree) => Json(tree).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_get_product_tree_summary(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match product_repo::summarize_product_tree(&state.app_state.db, &product_id).await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_create_planner_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobilePlannerSessionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match create_planner_session(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        body.provider_id,
        body.model_name,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_update_planner_session(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerUpdateRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match update_planner_session(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        session_id,
        body.provider_id,
        body.model_name,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_submit_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerTurnRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match submit_planner_turn(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
        body.user_input,
        body.selected_draft_node_id,
        body.product_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_submit_planner_voice_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<MobilePlannerTurnRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match submit_planner_voice_turn(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
        body.user_input,
        body.selected_draft_node_id,
        body.product_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_confirm_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match confirm_planner_plan(
        state.app_state.planner_service.clone(),
        &state.app_state,
        session_id,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_clear_planner_turn(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match clear_planner_pending(
        state.app_state.planner_service.clone(),
        &state.app_state.db,
        session_id,
    )
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn mobile_transcribe_audio(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileSpeechTranscriptionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }

    let (default_provider_id, default_model_name, default_locale) =
        match resolve_mobile_speech_defaults(&state.app_state).await {
            Ok(values) => values,
            Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
        };
    let provider_id = body
        .provider_id
        .filter(|value| !value.trim().is_empty())
        .or(default_provider_id);
    let Some(provider_id) = provider_id else {
        return (
            StatusCode::BAD_REQUEST,
            "A speech transcription provider is required.",
        )
            .into_response();
    };
    let model_name = body
        .model_name
        .filter(|value| !value.trim().is_empty())
        .or(default_model_name)
        .unwrap_or_else(|| "whisper-1".to_string());
    let request = SpeechToTextRequest {
        audio_bytes_base64: body.audio_bytes_base64,
        mime_type: body.mime_type,
        locale: body.locale.or(default_locale),
    };
    let provider = match model_repo::get_provider(&state.app_state.db, &provider_id).await {
        Ok(provider) => provider,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    match transcribe_audio_with_provider(&provider, &model_name, request).await {
        Ok(response) => Json::<SpeechToTextResponse>(response).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileSpeechGenerationRequest {
    provider_id: String,
    model: String,
    text: String,
    voice: Option<String>,
    speed: Option<f32>,
}

pub(crate) async fn mobile_generate_speech(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileSpeechGenerationRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let provider = match model_repo::get_provider(&state.app_state.db, &body.provider_id).await {
        Ok(provider) => provider,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    match crate::services::speech_service::synthesize_remote_speech(&provider, body.model, body.text, body.voice, body.speed).await {
        Ok(audio) => ([(axum::http::header::CONTENT_TYPE, "audio/wav"), (axum::http::header::CACHE_CONTROL, "no-store")], audio).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn resolve_mobile_speech_defaults(
    state: &AppState,
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let provider_id =
        std::env::var("ARUVI_SPEECH_PROVIDER_ID")
            .ok()
            .or(settings_repo::get_setting(&state.db, SPEECH_PROVIDER_KEY)
                .await
                .map_err(|error| error.to_string())?);
    let model_name = std::env::var("ARUVI_SPEECH_MODEL_NAME")
        .ok()
        .or(settings_repo::get_setting(&state.db, SPEECH_MODEL_KEY)
            .await
            .map_err(|error| error.to_string())?);
    let locale = std::env::var("ARUVI_SPEECH_LOCALE")
        .ok()
        .or(settings_repo::get_setting(&state.db, SPEECH_LOCALE_KEY)
            .await
            .map_err(|error| error.to_string())?);
    Ok((provider_id, model_name, locale))
}

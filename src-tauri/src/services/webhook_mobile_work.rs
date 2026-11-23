use crate::domain::work_item::WorkItemStatus;
use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::persistence::{approval_repo, artifact_repo, finding_repo, work_item_repo, workflow_repo};
use crate::services::webhook_bridge::ensure_mobile_api_authorized;
use crate::services::webhook_service::WebhookState;
use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct MobileWorkItemListQuery {
    #[serde(alias = "productId")]
    product_id: Option<String>,
    #[serde(alias = "sourceNodeId")]
    source_node_id: Option<String>,
    #[serde(alias = "sourceNodeType")]
    source_node_type: Option<String>,
    #[serde(alias = "topLevelOnly")]
    top_level_only: Option<bool>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileCreateWorkItemRequest {
    #[serde(alias = "productId")]
    product_id: String,
    title: String,
    description: Option<String>,
    #[serde(alias = "problemStatement")]
    problem_statement: Option<String>,
    #[serde(alias = "acceptanceCriteria")]
    acceptance_criteria: Option<String>,
    constraints: Option<String>,
    #[serde(alias = "workItemType")]
    work_item_type: Option<String>,
    priority: Option<String>,
    complexity: Option<String>,
    source_node_id: Option<String>,
    source_node_type: Option<String>,
    parent_work_item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileTaskFields {
    title: String,
    description: String,
    problem_statement: String,
    acceptance_criteria: String,
    constraints: String,
    #[serde(default)]
    status: Option<crate::domain::work_item::WorkItemStatus>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileUpdateWorkItemRequest {
    expected: MobileTaskFields,
    fields: MobileTaskFields,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkspaceAssignment {
    repository_id: Option<String>,
    branch_name: Option<String>,
    expected: crate::services::work_item_service::WorkspaceBaseline,
}

pub(crate) async fn mobile_assign_work_item_workspace(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<MobileWorkspaceAssignment>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::services::work_item_service::assign_workspace(&state.app_state, &id, body.repository_id.as_deref(), body.branch_name.as_deref(), Some(&body.expected)).await {
        Ok(item) => Json(item).into_response(),
        Err(AppError::Validation(error)) => (StatusCode::CONFLICT, error).into_response(),
        Err(error) => bad_request(error),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileDeleteWorkItem { expected: crate::services::work_item_service::WorkItemBaseline }

pub(crate) async fn mobile_delete_work_item(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<MobileDeleteWorkItem>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::services::work_item_service::delete_work_item(&state.app_state, &id, Some(&body.expected)).await {
        Ok(()) => Json(json!({"ok":true})).into_response(),
        Err(AppError::Validation(error)) => (StatusCode::CONFLICT, error).into_response(),
        Err(error) => bad_request(error),
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct MobileWorkItemActionRequest {
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileTaskDecision {
    action: String,
    notes: String,
    expected: crate::services::work_item_service::WorkItemBaseline,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileBatchTaskDecision {
    product_id: String,
    action: String,
    notes: String,
    items: Vec<crate::services::approval_service::BatchDecisionItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkMove {
    product_id: String,
    offset: i64,
    items: Vec<work_item_repo::WorkOrderEntry>,
    source_id: String,
}

pub(crate) async fn mobile_move_work_item(
    State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileWorkMove>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match work_item_repo::move_work_item(&state.app_state.db, &body.product_id, body.offset, &body.items, &body.source_id).await {
        Ok(()) => Json(json!({"ok":true})).into_response(),
        Err(AppError::Validation(error)) => (StatusCode::CONFLICT, error).into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_batch_task_decision(
    State(state): State<WebhookState>, headers: HeaderMap, Json(body): Json<MobileBatchTaskDecision>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let approved = match body.action.as_str() { "approve" => true, "reject" => false, _ => return (StatusCode::BAD_REQUEST, "Invalid decision").into_response() };
    let auto_start = if approved {
        match crate::persistence::settings_repo::get_bool_setting(&state.app_state.db, "workflow.auto_start_after_work_item_approval", true).await {
            Ok(value) => value, Err(error) => return bad_request(error),
        }
    } else { false };
    let approvals = match crate::services::approval_service::record_batch_task_decisions(&state.app_state.db, &body.product_id, approved, &body.notes, &body.items).await {
        Ok(value) => value,
        Err(AppError::Validation(message)) => return (StatusCode::CONFLICT, message).into_response(),
        Err(error) => return bad_request(error),
    };
    if auto_start { crate::services::approval_service::schedule_batch_workflows(&state.app_state, body.items.into_iter().map(|item| item.id).collect()); }
    Json(json!({"approvals":approvals,"auto_start_requested":auto_start})).into_response()
}

pub(crate) async fn mobile_task_decision(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<MobileTaskDecision>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let approved = match body.action.as_str() { "approve" => true, "reject" => false, _ => return (StatusCode::BAD_REQUEST, "Invalid decision").into_response() };
    if body.notes.len() > 4000 || (!approved && body.notes.trim().is_empty()) { return (StatusCode::BAD_REQUEST, "A rejection reason is required and notes must be at most 4000 bytes").into_response(); }
    let auto_start = if approved {
        match crate::persistence::settings_repo::get_bool_setting(&state.app_state.db, "workflow.auto_start_after_work_item_approval", true).await {
            Ok(value) => value, Err(error) => return bad_request(error),
        }
    } else { false };
    let approval = match crate::services::approval_service::record_task_decision(&state.app_state.db, &id, approved, &body.notes, Some(&body.expected)).await {
        Ok(value) => value,
        Err(AppError::Validation(message)) => return (StatusCode::CONFLICT, message).into_response(),
        Err(error) => return bad_request(error),
    };
    if auto_start {
        let service = state.app_state.workflow_service.clone();
        tokio::spawn(async move {
            if let Err(error) = service.lock().await.start_work_item_workflow(&id).await {
                tracing::error!(work_item_id=%id, error=%error, "auto-start after browser approval failed");
            }
        });
    }
    Json(json!({"approval":approval,"auto_start_requested":auto_start})).into_response()
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkflowActionRequest {
    action: String,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkflowFailureRequest {
    expected_stage: String,
    expected_status: String,
    reason: String,
}

pub(crate) async fn mobile_mark_workflow_failed(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<MobileWorkflowFailureRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::services::workflow_failure::mark_workflow_failed_by_operator(&state.app_state.db, &id, &body.expected_stage, &body.expected_status, &body.reason).await {
        Ok(()) => Json(json!({"status":"ok", "workflow_run_id":id})).into_response(),
        Err(AppError::Validation(message)) => (StatusCode::CONFLICT, message).into_response(),
        Err(error) => bad_request(error),
    }
}

fn bad_request(error: impl ToString) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, error.to_string()).into_response()
}

#[derive(Deserialize)]
pub(crate) struct MobileExternalCliRequest { provider: String }

pub(crate) async fn mobile_external_cli_runs(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if let Err(error) = work_item_repo::get_work_item(&state.app_state.db, &id).await { return bad_request(error); }
    match crate::persistence::external_cli_repo::list_recent_external_cli_runs_for_work_item(&state.app_state.db, &id).await {
        Ok(runs) => Json(runs).into_response(), Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_run_external_cli(
    State(state): State<WebhookState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<MobileExternalCliRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    match crate::services::external_cli_service::run_remote_external_cli_for_work_item(&state.app_state.db, &state.app_state.artifact_base_path, &id, &body.provider).await {
        Ok(run) => Json(run).into_response(), Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_external_cli_events(
    State(state): State<WebhookState>, headers: HeaderMap, Path((work_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let run = match crate::persistence::external_cli_repo::get_external_cli_run(&state.app_state.db, &run_id).await {
        Ok(run) if run.work_item_id == work_id => run,
        _ => return (StatusCode::NOT_FOUND, "External CLI run not found").into_response(),
    };
    match crate::persistence::external_cli_repo::list_external_cli_run_events(&state.app_state.db, &run.id, 500).await {
        Ok(events) => Json(events).into_response(), Err(error) => bad_request(error),
    }
}

fn workflow_start_error(
    error: AppError,
    workflow_run: Option<WorkflowRun>,
) -> axum::response::Response {
    let status = match &error {
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Validation(_) => StatusCode::CONFLICT,
        AppError::Provider(_) | AppError::ProviderHttp { .. } | AppError::ProviderTransport { .. } => StatusCode::BAD_GATEWAY,
        AppError::Database(_)
        | AppError::Git(_)
        | AppError::Io(_)
        | AppError::Serialization(_)
        | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "error": error.to_string(),
            "status": "failed",
            "workflow_run": workflow_run,
        })),
    )
        .into_response()
}

async fn start_workflow_response(
    state: &WebhookState,
    work_item_id: &str,
) -> axum::response::Response {
    let service = state.app_state.workflow_service.lock().await;
    let started = match service.start_work_item_workflow(work_item_id).await {
        Ok(run) => run,
        Err(error) => {
            let workflow_run = workflow_repo::get_latest_workflow_run_for_work_item(
                &state.app_state.db,
                work_item_id,
            )
            .await
            .ok()
            .flatten();
            return workflow_start_error(error, workflow_run);
        }
    };
    let current = match service.get_workflow_run(&started.id).await {
        Ok(run) => run,
        Err(error) => return workflow_start_error(error, Some(started)),
    };

    Json(json!({
        "status": "started",
        "work_item_id": work_item_id,
        "workflow_run": current,
    }))
    .into_response()
}

pub(crate) async fn mobile_list_work_items(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Query(query): Query<MobileWorkItemListQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    if query.source_node_id.is_some() != query.source_node_type.is_some()
        || (query.source_node_id.is_some()
            && query.product_id.as_deref().is_none_or(|id| id.trim().is_empty()))
        || query.source_node_id.as_deref().is_some_and(|id| id.trim().is_empty())
    {
        return (
            StatusCode::BAD_REQUEST,
            "A scoped work list requires a product, source node ID and source node type",
        ).into_response();
    }
    let filters = work_item_repo::WorkItemListQuery {
        product_id: query.product_id.as_deref(),
        source_node_id: query.source_node_id.as_deref(),
        source_node_type: query.source_node_type.as_deref(),
        status: query.status.as_deref(),
        limit: query.limit.or(Some(100)),
        offset: query.offset,
        ..Default::default()
    };
    let page = if query.top_level_only.unwrap_or(false) {
        work_item_repo::list_top_level_work_items_page_with_metadata(&state.app_state.db, filters).await
    } else {
        work_item_repo::list_work_items_page_with_metadata(&state.app_state.db, filters).await
    };
    match page {
        Ok(page) => Json(json!({
            "items": page.items,
            "limit": page.limit,
            "offset": page.offset,
            "has_more": page.has_more,
        }))
        .into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_get_work_item(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    match work_item_repo::get_work_item(&state.app_state.db, &work_item_id).await {
        Ok(work_item) => Json(work_item).into_response(),
        Err(AppError::NotFound(_)) => (StatusCode::NOT_FOUND, "Work item not found").into_response(),
        Err(error) => bad_request(error),
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct MobileChildWorkQuery { offset: Option<i64> }

pub(crate) async fn mobile_list_child_work_items(
    State(state): State<WebhookState>, headers: HeaderMap, Path(work_item_id): Path<String>, Query(query): Query<MobileChildWorkQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let offset = query.offset.unwrap_or(0);
    if !(0..=100_000).contains(&offset) { return bad_request("Invalid child work offset"); }
    if work_item_repo::get_work_item(&state.app_state.db, &work_item_id).await.is_err() { return bad_request("Parent work item not found"); }
    match work_item_repo::get_sub_work_items_page(&state.app_state.db, &work_item_id, Some(51), Some(offset)).await {
        Ok(mut items) => {
            let has_more = items.len() > 50;
            items.truncate(50);
            Json(json!({"items":items,"offset":offset,"limit":50,"has_more":has_more})).into_response()
        },
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_update_work_item(
    State(state): State<WebhookState>, headers: HeaderMap, Path(work_item_id): Path<String>, Json(body): Json<MobileUpdateWorkItemRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    if body.fields.title.trim().is_empty() { return bad_request("A task title is required"); }
    if body.expected.status.is_some() != body.fields.status.is_some() { return bad_request("Expected and updated status must be provided together"); }
    let f = &body.fields;
    let e = &body.expected;
    let status = f.status.as_ref().map(ToString::to_string);
    let expected_status = e.status.as_ref().map(ToString::to_string);
    let result = sqlx::query("UPDATE work_items SET title=?, description=?, problem_statement=?, acceptance_criteria=?, constraints=?, status=COALESCE(?, status), updated_at=datetime('now') WHERE id=? AND title=? AND description=? AND problem_statement=? AND acceptance_criteria=? AND constraints=? AND (? IS NULL OR status=?)")
        .bind(f.title.trim()).bind(&f.description).bind(&f.problem_statement).bind(&f.acceptance_criteria).bind(&f.constraints).bind(status)
        .bind(&work_item_id).bind(&e.title).bind(&e.description).bind(&e.problem_statement).bind(&e.acceptance_criteria).bind(&e.constraints).bind(&expected_status).bind(&expected_status)
        .execute(&state.app_state.db).await;
    match result {
        Ok(result) if result.rows_affected() == 1 => match work_item_repo::get_work_item(&state.app_state.db, &work_item_id).await {
            Ok(item) => Json(item).into_response(), Err(error) => bad_request(error),
        },
        Ok(_) => (StatusCode::CONFLICT, "Task changed or was removed. Refresh and review your edits before saving.").into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_create_work_item(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(body): Json<MobileCreateWorkItemRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let product_id = body.product_id.trim();
    let title = body.title.trim();
    if product_id.is_empty() || title.is_empty() {
        return bad_request("product_id and title are required");
    }
    let description = body.description.unwrap_or_default();
    let problem_statement = body
        .problem_statement
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if description.trim().is_empty() {
                title.to_string()
            } else {
                description.clone()
            }
        });
    let acceptance_criteria = body.acceptance_criteria.unwrap_or_default();
    let constraints = body.constraints.unwrap_or_default();
    let work_item_type = body.work_item_type.unwrap_or_else(|| "task".to_string());
    let priority = body.priority.unwrap_or_else(|| "medium".to_string());
    let complexity = body.complexity.unwrap_or_else(|| "medium".to_string());
    let id = uuid::Uuid::new_v4().to_string();

    match work_item_repo::create_work_item(
        &state.app_state.db,
        work_item_repo::CreateWorkItemInput {
            id: &id,
            product_id,
            product_area_id: None,
            capability_id: None,
            source_node_id: body.source_node_id.as_deref(),
            source_node_type: body.source_node_type.as_deref(),
            parent_work_item_id: body.parent_work_item_id.as_deref(),
            title,
            problem_statement: &problem_statement,
            description: &description,
            acceptance_criteria: &acceptance_criteria,
            constraints: &constraints,
            work_item_type: &work_item_type,
            priority: &priority,
            complexity: &complexity,
        },
    )
    .await
    {
        Ok(work_item) => (StatusCode::CREATED, Json(work_item)).into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_approve_work_item(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
    Json(body): Json<MobileWorkItemActionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let work_item = match work_item_repo::get_work_item(&state.app_state.db, &work_item_id).await {
        Ok(work_item) => work_item,
        Err(error) => return bad_request(error),
    };
    if work_item.status != WorkItemStatus::Approved {
        if let Err(error) = crate::services::approval_service::record_task_decision(
            &state.app_state.db, &work_item_id, true,
            body.notes.as_deref().unwrap_or("Approved from remote Studio"), None,
        ).await {
            return bad_request(error);
        }
    }

    start_workflow_response(&state, &work_item_id).await
}

pub(crate) async fn mobile_start_workflow(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    start_workflow_response(&state, &work_item_id).await
}

pub(crate) async fn mobile_get_work_item_delivery(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let work_item = match work_item_repo::get_work_item(&state.app_state.db, &work_item_id).await {
        Ok(work_item) => work_item,
        Err(error) => return bad_request(error),
    };
    let workflow_run = match workflow_repo::get_latest_workflow_run_for_work_item(
        &state.app_state.db,
        &work_item_id,
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return bad_request(error),
    };
    let artifacts =
        match artifact_repo::list_work_item_artifacts(&state.app_state.db, &work_item_id).await {
            Ok(artifacts) => artifacts,
            Err(error) => return bad_request(error),
        };
    let approvals = match approval_repo::list_approvals(&state.app_state.db, &work_item_id).await {
        Ok(approvals) => approvals,
        Err(error) => return bad_request(error),
    };
    let findings = match finding_repo::list_work_item_findings(&state.app_state.db, &work_item_id).await {
        Ok(findings) => findings,
        Err(error) => return bad_request(error),
    };
    let retryable_git_delivery = match &workflow_run {
        Some(run) => match crate::services::workflow_git_push::is_retryable_git_delivery(&state.app_state.db, &run.id).await {
            Ok(value) => value, Err(error) => return bad_request(error),
        },
        None => false,
    };
    Json(json!({
        "work_item": work_item,
        "workflow_run": workflow_run,
        "retryable_git_delivery": retryable_git_delivery,
        "artifacts": artifacts,
        "approvals": approvals,
        "findings": findings,
    }))
    .into_response()
}

pub(crate) async fn mobile_get_workflow_test_result(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path((work_item_id, artifact_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let artifact = match artifact_repo::get_artifact(&state.app_state.db, &artifact_id).await {
        Ok(value) if value.work_item_id == work_item_id && value.artifact_type == "workflow_test_result" => value,
        _ => return (StatusCode::NOT_FOUND, "Test result not found").into_response(),
    };
    match crate::services::artifact_service::read_text(
        &state.app_state.artifact_base_path, std::path::Path::new(&artifact.storage_path),
        crate::services::artifact_service::TEST_RESULT_LIMIT,
    ).await {
        Ok(content) => Json(json!({"content":content})).into_response(),
        Err(AppError::NotFound(_)) => (StatusCode::NOT_FOUND, "Test result not found").into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_get_work_item_artifact(
    State(state): State<WebhookState>, headers: HeaderMap,
    Path((work_item_id, artifact_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let artifact = match artifact_repo::get_artifact(&state.app_state.db, &artifact_id).await {
        Ok(value) if value.work_item_id == work_item_id => value,
        _ => return (StatusCode::NOT_FOUND, "Artifact not found").into_response(),
    };
    match crate::services::artifact_service::read_text(
        &state.app_state.artifact_base_path, std::path::Path::new(&artifact.storage_path),
        crate::services::artifact_service::TEXT_LIMIT,
    ).await {
        Ok(content) => Json(json!({"content":content})).into_response(),
        Err(AppError::NotFound(_)) => (StatusCode::NOT_FOUND, "Artifact not found").into_response(),
        Err(error) => bad_request(error),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileArtifactPageQuery {
    #[serde(default)]
    offset: u64,
    version: Option<String>,
}

pub(crate) async fn mobile_get_work_item_artifact_page(
    State(state): State<WebhookState>, headers: HeaderMap,
    Path((work_item_id, artifact_id)): Path<(String, String)>, Query(query): Query<MobileArtifactPageQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let artifact = match artifact_repo::get_artifact(&state.app_state.db, &artifact_id).await {
        Ok(value) if value.work_item_id == work_item_id => value,
        _ => return (StatusCode::NOT_FOUND, "Artifact not found").into_response(),
    };
    match crate::services::artifact_service::read_page(
        &state.app_state.artifact_base_path, std::path::Path::new(&artifact.storage_path), query.offset, query.version.as_deref(),
    ).await {
        Ok(page) => Json(page).into_response(),
        Err(AppError::NotFound(_)) => (StatusCode::NOT_FOUND, "Artifact not found").into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_retry_git_delivery(State(state): State<WebhookState>, headers: HeaderMap, Path(workflow_run_id): Path<String>) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await { return response; }
    let service = state.app_state.workflow_service.lock().await;
    match service.retry_git_delivery(&workflow_run_id).await {
        Ok(()) => Json(json!({"status":"ok","workflow_run_id":workflow_run_id})).into_response(),
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn mobile_handle_workflow_action(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(workflow_run_id): Path<String>,
    Json(body): Json<MobileWorkflowActionRequest>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let action = match body.action.as_str() {
        "approve" => crate::domain::workflow::UserAction::Approve,
        "reject" => crate::domain::workflow::UserAction::Reject,
        "pause" => crate::domain::workflow::UserAction::Pause,
        "resume" => crate::domain::workflow::UserAction::Resume,
        "cancel" => crate::domain::workflow::UserAction::Cancel,
        _ => return bad_request(format!("Unsupported workflow action: {}", body.action)),
    };
    let service = state.app_state.workflow_service.lock().await;
    match service
        .handle_user_action(&workflow_run_id, action, body.notes)
        .await
    {
        Ok(()) => {
            Json(json!({ "status": "ok", "workflow_run_id": workflow_run_id })).into_response()
        }
        Err(error) => bad_request(error),
    }
}

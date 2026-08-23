use crate::domain::work_item::WorkItemStatus;
use crate::persistence::{approval_repo, artifact_repo, work_item_repo, workflow_repo};
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
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct MobileWorkItemActionRequest {
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MobileWorkflowActionRequest {
    action: String,
    notes: Option<String>,
}

fn bad_request(error: impl ToString) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, error.to_string()).into_response()
}

pub(crate) async fn mobile_list_work_items(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Query(query): Query<MobileWorkItemListQuery>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let page = work_item_repo::list_work_items_page_with_metadata(
        &state.app_state.db,
        work_item_repo::WorkItemListQuery {
            product_id: query.product_id.as_deref(),
            status: query.status.as_deref(),
            limit: query.limit.or(Some(100)),
            offset: query.offset,
            ..Default::default()
        },
    )
    .await;
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
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
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
        let approval_id = uuid::Uuid::new_v4().to_string();
        if let Err(error) = approval_repo::create_approval(
            &state.app_state.db,
            &approval_id,
            &work_item_id,
            None,
            "task_approval",
            "approved",
            body.notes
                .as_deref()
                .unwrap_or("Approved from remote Studio"),
        )
        .await
        {
            return bad_request(error);
        }
        if let Err(error) = work_item_repo::update_work_item(
            &state.app_state.db,
            work_item_repo::UpdateWorkItemPatch {
                id: &work_item_id,
                status: Some("approved"),
                title: None,
                description: None,
                problem_statement: None,
                acceptance_criteria: None,
                constraints: None,
            },
        )
        .await
        {
            return bad_request(error);
        }
    }

    let workflow_service = state.app_state.workflow_service.clone();
    let work_item_id_for_start = work_item_id.clone();
    tokio::spawn(async move {
        let service = workflow_service.lock().await;
        if let Err(error) = service
            .start_work_item_workflow(&work_item_id_for_start)
            .await
        {
            tracing::error!(
                work_item_id = %work_item_id_for_start,
                error = %error,
                "remote work item auto-start failed"
            );
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "accepted",
            "work_item_id": work_item_id,
            "message": "Work item approved; workflow start has been queued."
        })),
    )
        .into_response()
}

pub(crate) async fn mobile_start_workflow(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = ensure_mobile_api_authorized(&state.app_state, &headers).await {
        return response;
    }
    let workflow_service = state.app_state.workflow_service.clone();
    let work_item_id_for_start = work_item_id.clone();
    tokio::spawn(async move {
        let service = workflow_service.lock().await;
        if let Err(error) = service
            .start_work_item_workflow(&work_item_id_for_start)
            .await
        {
            tracing::error!(
                work_item_id = %work_item_id_for_start,
                error = %error,
                "remote workflow start failed"
            );
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "accepted",
            "work_item_id": work_item_id,
            "message": "Workflow start has been queued."
        })),
    )
        .into_response()
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
    Json(json!({
        "work_item": work_item,
        "workflow_run": workflow_run,
        "artifacts": artifacts,
        "approvals": approvals,
    }))
    .into_response()
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

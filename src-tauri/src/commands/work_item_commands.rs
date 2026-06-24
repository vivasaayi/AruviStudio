use crate::domain::work_item::{ProductWorkItemSummary, WorkItem, WorkItemScopeSummary};
use crate::error::AppError;
use crate::persistence::work_item_repo;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info, warn};

fn resolve_required(value: Option<String>, field_name: &str) -> Result<String, AppError> {
    value.ok_or_else(|| AppError::Validation(format!("missing {}", field_name)))
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkItemCommand {
    #[serde(alias = "productId")]
    pub(crate) product_id: Option<String>,
    #[serde(alias = "productAreaId")]
    pub(crate) product_area_id: Option<String>,
    #[serde(alias = "capabilityId")]
    pub(crate) capability_id: Option<String>,
    #[serde(alias = "sourceNodeId")]
    pub(crate) source_node_id: Option<String>,
    #[serde(alias = "sourceNodeType")]
    pub(crate) source_node_type: Option<String>,
    #[serde(alias = "parentWorkItemId")]
    pub(crate) parent_work_item_id: Option<String>,
    pub(crate) title: String,
    #[serde(alias = "problemStatement")]
    pub(crate) problem_statement: String,
    pub(crate) description: String,
    #[serde(alias = "acceptanceCriteria")]
    pub(crate) acceptance_criteria: String,
    pub(crate) constraints: String,
    #[serde(alias = "workItemType")]
    pub(crate) work_item_type: String,
    pub(crate) priority: String,
    pub(crate) complexity: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListWorkItemsCommand {
    #[serde(alias = "productId")]
    pub(crate) product_id: Option<String>,
    #[serde(alias = "productAreaId")]
    pub(crate) product_area_id: Option<String>,
    #[serde(alias = "capabilityId")]
    pub(crate) capability_id: Option<String>,
    #[serde(alias = "sourceNodeId")]
    pub(crate) source_node_id: Option<String>,
    #[serde(alias = "sourceNodeType")]
    pub(crate) source_node_type: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
    #[serde(alias = "topLevelOnly")]
    pub(crate) top_level_only: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct WorkItemPageCommand {
    pub(crate) items: Vec<WorkItem>,
    pub(crate) limit: i64,
    pub(crate) offset: i64,
    pub(crate) has_more: bool,
}

fn work_item_query_from_request(
    request: &ListWorkItemsCommand,
) -> work_item_repo::WorkItemListQuery<'_> {
    work_item_repo::WorkItemListQuery {
        product_id: request.product_id.as_deref(),
        product_area_id: request.product_area_id.as_deref(),
        capability_id: request.capability_id.as_deref(),
        source_node_id: request.source_node_id.as_deref(),
        source_node_type: request.source_node_type.as_deref(),
        status: request.status.as_deref(),
        limit: request.limit,
        offset: request.offset,
    }
}

fn bounded_legacy_work_item_query(
    request: &ListWorkItemsCommand,
) -> work_item_repo::WorkItemListQuery<'_> {
    let query = work_item_query_from_request(request);
    let (limit, offset) = query.bounded_page();
    work_item_repo::WorkItemListQuery {
        limit: Some(limit),
        offset: Some(offset),
        ..query
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkItemCommand {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) status: Option<String>,
    #[serde(alias = "problemStatement")]
    pub(crate) problem_statement: Option<String>,
    #[serde(alias = "acceptanceCriteria")]
    pub(crate) acceptance_criteria: Option<String>,
    pub(crate) constraints: Option<String>,
}

#[tauri::command]
pub async fn create_work_item(
    state: State<'_, AppState>,
    request: CreateWorkItemCommand,
) -> Result<WorkItem, AppError> {
    let product_id = resolve_required(request.product_id, "product id")?;
    let work_item_type = if request.work_item_type.trim().is_empty() {
        "story".to_string()
    } else {
        request.work_item_type
    };
    info!(product_id = %product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, parent_work_item_id = ?request.parent_work_item_id, title = %request.title, "create_work_item requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = work_item_repo::create_work_item(
        &state.db,
        work_item_repo::CreateWorkItemInput {
            id: &id,
            product_id: &product_id,
            product_area_id: request.product_area_id.as_deref(),
            capability_id: request.capability_id.as_deref(),
            source_node_id: request.source_node_id.as_deref(),
            source_node_type: request.source_node_type.as_deref(),
            parent_work_item_id: request.parent_work_item_id.as_deref(),
            title: &request.title,
            problem_statement: &request.problem_statement,
            description: &request.description,
            acceptance_criteria: &request.acceptance_criteria,
            constraints: &request.constraints,
            work_item_type: &work_item_type,
            priority: &request.priority,
            complexity: &request.complexity,
        },
    )
    .await;
    match &result {
        Ok(work_item) => {
            info!(work_item_id = %work_item.id, product_id = ?work_item.product_id, "create_work_item succeeded")
        }
        Err(err) => {
            error!(work_item_id = %id, product_id = %product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, parent_work_item_id = ?request.parent_work_item_id, error = %err, "create_work_item failed")
        }
    }
    result
}

#[tauri::command]
pub async fn get_work_item(state: State<'_, AppState>, id: String) -> Result<WorkItem, AppError> {
    work_item_repo::get_work_item(&state.db, &id).await
}

#[tauri::command]
pub async fn list_work_items(
    state: State<'_, AppState>,
    request: ListWorkItemsCommand,
) -> Result<Vec<WorkItem>, AppError> {
    let query = bounded_legacy_work_item_query(&request);
    debug!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, requested_limit = ?request.limit, requested_offset = ?request.offset, limit = ?query.limit, offset = ?query.offset, "list_work_items requested");
    if request.limit != query.limit || request.offset.is_some_and(|offset| offset < 0) {
        warn!(product_id = ?request.product_id, requested_limit = ?request.limit, requested_offset = ?request.offset, limit = ?query.limit, offset = ?query.offset, default_limit = work_item_repo::DEFAULT_LIST_WORK_ITEMS_LIMIT, max_limit = work_item_repo::MAX_LIST_WORK_ITEMS_LIMIT, "legacy list_work_items request normalized to bounded pagination; use list_work_items_page for pagination metadata");
    }
    let result = work_item_repo::list_work_items_page(&state.db, query).await;
    if let Err(err) = &result {
        error!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, limit = ?request.limit, offset = ?request.offset, error = %err, "list_work_items failed");
    }
    result
}

#[tauri::command]
pub async fn list_work_items_page(
    state: State<'_, AppState>,
    request: ListWorkItemsCommand,
) -> Result<WorkItemPageCommand, AppError> {
    debug!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, limit = ?request.limit, offset = ?request.offset, "list_work_items_page requested");
    let query = work_item_query_from_request(&request);
    let result = if request.top_level_only.unwrap_or(false) {
        work_item_repo::list_top_level_work_items_page_with_metadata(&state.db, query).await
    } else {
        work_item_repo::list_work_items_page_with_metadata(&state.db, query).await
    }
    .map(|page| WorkItemPageCommand {
        items: page.items,
        limit: page.limit,
        offset: page.offset,
        has_more: page.has_more,
    });
    if let Err(err) = &result {
        error!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, limit = ?request.limit, offset = ?request.offset, error = %err, "list_work_items_page failed");
    }
    result
}

#[tauri::command]
pub async fn summarize_work_items_by_product(
    state: State<'_, AppState>,
) -> Result<Vec<ProductWorkItemSummary>, AppError> {
    debug!("summarize_work_items_by_product requested");
    let result = work_item_repo::summarize_work_items_by_product(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "summarize_work_items_by_product failed");
    }
    result
}

#[tauri::command]
pub async fn summarize_work_items_by_scope(
    state: State<'_, AppState>,
    product_id: Option<String>,
) -> Result<Vec<WorkItemScopeSummary>, AppError> {
    debug!(product_id = ?product_id, "summarize_work_items_by_scope requested");
    let result =
        work_item_repo::summarize_work_items_by_scope(&state.db, product_id.as_deref()).await;
    if let Err(err) = &result {
        error!(product_id = ?product_id, error = %err, "summarize_work_items_by_scope failed");
    }
    result
}

#[tauri::command]
pub async fn update_work_item(
    state: State<'_, AppState>,
    request: UpdateWorkItemCommand,
) -> Result<WorkItem, AppError> {
    info!(work_item_id = %request.id, "update_work_item requested");
    debug!(work_item_id = %request.id, has_title = request.title.is_some(), has_description = request.description.is_some(), has_status = request.status.is_some(), has_problem_statement = request.problem_statement.is_some(), has_acceptance_criteria = request.acceptance_criteria.is_some(), has_constraints = request.constraints.is_some(), "update_work_item payload summary");
    let result = work_item_repo::update_work_item(
        &state.db,
        work_item_repo::UpdateWorkItemPatch {
            id: &request.id,
            title: request.title.as_deref(),
            description: request.description.as_deref(),
            status: request.status.as_deref(),
            problem_statement: request.problem_statement.as_deref(),
            acceptance_criteria: request.acceptance_criteria.as_deref(),
            constraints: request.constraints.as_deref(),
        },
    )
    .await;
    match &result {
        Ok(_) => info!(work_item_id = %request.id, "update_work_item succeeded"),
        Err(err) => error!(work_item_id = %request.id, error = %err, "update_work_item failed"),
    }
    result
}

#[tauri::command]
pub async fn assign_work_item_workspace(
    state: State<'_, AppState>,
    id: String,
    repository_id: Option<String>,
    branch_name: Option<String>,
) -> Result<WorkItem, AppError> {
    info!(work_item_id = %id, repository_id = ?repository_id, "assign_work_item_workspace requested");
    let repository_id = repository_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let branch_name = branch_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(repository_id) = repository_id.as_deref() {
        crate::persistence::repository_repo::get_repository(&state.db, repository_id).await?;
    }

    let result = work_item_repo::assign_work_item_workspace(
        &state.db,
        &id,
        repository_id.as_deref(),
        branch_name.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(work_item_id = %id, "assign_work_item_workspace succeeded"),
        Err(err) => error!(work_item_id = %id, error = %err, "assign_work_item_workspace failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_work_item(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    info!(work_item_id = %id, "delete_work_item requested");
    let result = work_item_repo::delete_work_item(&state.db, &id).await;
    match &result {
        Ok(_) => info!(work_item_id = %id, "delete_work_item succeeded"),
        Err(err) => error!(work_item_id = %id, error = %err, "delete_work_item failed"),
    }
    result
}

#[tauri::command]
pub async fn get_sub_work_items(
    state: State<'_, AppState>,
    work_item_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<WorkItem>, AppError> {
    debug!(work_item_id = %work_item_id, limit = ?limit, offset = ?offset, "get_sub_work_items requested");
    let result =
        work_item_repo::get_sub_work_items_page(&state.db, &work_item_id, limit, offset).await;
    if let Err(err) = &result {
        error!(work_item_id = %work_item_id, limit = ?limit, offset = ?offset, error = %err, "get_sub_work_items failed");
    }
    result
}

#[tauri::command]
pub async fn reorder_work_items(
    state: State<'_, AppState>,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    info!(
        item_count = ordered_ids.len(),
        "reorder_work_items requested"
    );
    let result = work_item_repo::reorder_work_items(&state.db, &ordered_ids).await;
    match &result {
        Ok(_) => info!(
            item_count = ordered_ids.len(),
            "reorder_work_items succeeded"
        ),
        Err(err) => {
            error!(item_count = ordered_ids.len(), error = %err, "reorder_work_items failed")
        }
    }
    result
}

#[cfg(test)]
mod tests;

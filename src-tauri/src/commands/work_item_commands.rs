use crate::domain::work_item::{ProductWorkItemSummary, WorkItem};
use crate::error::AppError;
use crate::persistence::work_item_repo;
use crate::state::AppState;
use tauri::State;
use tracing::{debug, error, info};

fn resolve_required(
    value: Option<String>,
    legacy: Option<String>,
    field_name: &str,
) -> Result<String, AppError> {
    value
        .or(legacy)
        .ok_or_else(|| AppError::Validation(format!("missing {}", field_name)))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn create_work_item(
    state: State<'_, AppState>,
    product_id: Option<String>,
    productId: Option<String>,
    module_id: Option<String>,
    moduleId: Option<String>,
    capability_id: Option<String>,
    capabilityId: Option<String>,
    parent_work_item_id: Option<String>,
    parentWorkItemId: Option<String>,
    title: String,
    problem_statement: String,
    problemStatement: Option<String>,
    description: String,
    acceptance_criteria: String,
    acceptanceCriteria: Option<String>,
    constraints: String,
    work_item_type: String,
    workItemType: Option<String>,
    priority: String,
    complexity: String,
) -> Result<WorkItem, AppError> {
    let product_id = resolve_required(product_id, productId, "product id")?;
    let module_id = module_id.or(moduleId);
    let capability_id = capability_id.or(capabilityId);
    let parent_work_item_id = parent_work_item_id.or(parentWorkItemId);
    let problem_statement = if problem_statement.trim().is_empty() {
        problemStatement.unwrap_or_default()
    } else {
        problem_statement
    };
    let acceptance_criteria = if acceptance_criteria.trim().is_empty() {
        acceptanceCriteria.unwrap_or_default()
    } else {
        acceptance_criteria
    };
    let work_item_type = if work_item_type.trim().is_empty() {
        workItemType.unwrap_or_else(|| "feature".to_string())
    } else {
        work_item_type
    };
    info!(product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, parent_work_item_id = ?parent_work_item_id, title = %title, "create_work_item requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = work_item_repo::create_work_item(
        &state.db,
        &id,
        &product_id,
        module_id.as_deref(),
        capability_id.as_deref(),
        parent_work_item_id.as_deref(),
        &title,
        &problem_statement,
        &description,
        &acceptance_criteria,
        &constraints,
        &work_item_type,
        &priority,
        &complexity,
    )
    .await;
    match &result {
        Ok(work_item) => {
            info!(work_item_id = %work_item.id, product_id = ?work_item.product_id, "create_work_item succeeded")
        }
        Err(err) => {
            error!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, parent_work_item_id = ?parent_work_item_id, error = %err, "create_work_item failed")
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
    product_id: Option<String>,
    module_id: Option<String>,
    capability_id: Option<String>,
    status: Option<String>,
) -> Result<Vec<WorkItem>, AppError> {
    debug!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, status = ?status, "list_work_items requested");
    let result = work_item_repo::list_work_items(
        &state.db,
        product_id.as_deref(),
        module_id.as_deref(),
        capability_id.as_deref(),
        status.as_deref(),
    )
    .await;
    if let Err(err) = &result {
        error!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, status = ?status, error = %err, "list_work_items failed");
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
pub async fn update_work_item(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    problem_statement: Option<String>,
    acceptance_criteria: Option<String>,
    constraints: Option<String>,
) -> Result<WorkItem, AppError> {
    info!(work_item_id = %id, "update_work_item requested");
    debug!(work_item_id = %id, has_title = title.is_some(), has_description = description.is_some(), has_status = status.is_some(), has_problem_statement = problem_statement.is_some(), has_acceptance_criteria = acceptance_criteria.is_some(), has_constraints = constraints.is_some(), "update_work_item payload summary");
    let result = work_item_repo::update_work_item(
        &state.db,
        &id,
        title.as_deref(),
        description.as_deref(),
        status.as_deref(),
        problem_statement.as_deref(),
        acceptance_criteria.as_deref(),
        constraints.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(work_item_id = %id, "update_work_item succeeded"),
        Err(err) => error!(work_item_id = %id, error = %err, "update_work_item failed"),
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
) -> Result<Vec<WorkItem>, AppError> {
    debug!(work_item_id = %work_item_id, "get_sub_work_items requested");
    let result = work_item_repo::get_sub_work_items(&state.db, &work_item_id).await;
    if let Err(err) = &result {
        error!(work_item_id = %work_item_id, error = %err, "get_sub_work_items failed");
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

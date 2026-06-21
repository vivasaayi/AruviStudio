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
    source_node_id: Option<String>,
    sourceNodeId: Option<String>,
    source_node_type: Option<String>,
    sourceNodeType: Option<String>,
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
    let source_node_id = source_node_id.or(sourceNodeId);
    let source_node_type = source_node_type.or(sourceNodeType);
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
        workItemType.unwrap_or_else(|| "story".to_string())
    } else {
        work_item_type
    };
    info!(product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, title = %title, "create_work_item requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = work_item_repo::create_work_item(
        &state.db,
        &id,
        &product_id,
        module_id.as_deref(),
        capability_id.as_deref(),
        source_node_id.as_deref(),
        source_node_type.as_deref(),
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
            error!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, error = %err, "create_work_item failed")
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
    source_node_id: Option<String>,
    source_node_type: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<WorkItem>, AppError> {
    debug!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, status = ?status, limit = ?limit, offset = ?offset, "list_work_items requested");
    let result = work_item_repo::list_work_items_page(
        &state.db,
        product_id.as_deref(),
        module_id.as_deref(),
        capability_id.as_deref(),
        source_node_id.as_deref(),
        source_node_type.as_deref(),
        status.as_deref(),
        limit,
        offset,
    )
    .await;
    if let Err(err) = &result {
        error!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, status = ?status, limit = ?limit, offset = ?offset, error = %err, "list_work_items failed");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::product_commands;
    use crate::commands::test_helpers::make_test_app;
    use tauri::Manager;
    use tauri::test::MockRuntime;

    #[tokio::test]
    async fn create_work_item_accepts_legacy_aliases_and_fallback_fields() {
        let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_create").await;
        let state = app.state::<AppState>();

        let product = product_commands::create_product(
            state.clone(),
            "Work Item Product".to_string(),
            "".to_string(),
            "".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("product should be created");
        let module = product_commands::create_module(
            state.clone(),
            product.id.clone(),
            "Area".to_string(),
            "".to_string(),
            "".to_string(),
            Some("product_area".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("module should be created");
        let capability = product_commands::create_capability(
            state.clone(),
            module.id.clone(),
            None,
            "Capability".to_string(),
            "".to_string(),
            "".to_string(),
            "medium".to_string(),
            "low".to_string(),
            "".to_string(),
            Some("capability".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("capability should be created");

        let work_item = create_work_item(
            state,
            None,
            Some(product.id.clone()),
            None,
            Some(module.id.clone()),
            None,
            Some(capability.id.clone()),
            None,
            Some(capability.id.clone()),
            None,
            Some("capability".to_string()),
            None,
            None,
            "Legacy Work Item".to_string(),
            "".to_string(),
            Some("Problem from alias".to_string()),
            "".to_string(),
            "".to_string(),
            Some("Acceptance from alias".to_string()),
            "".to_string(),
            "".to_string(),
            Some("story".to_string()),
            "high".to_string(),
            "medium".to_string(),
        )
        .await
        .expect("work item should be created");

        assert_eq!(work_item.product_id.as_deref(), Some(product.id.as_str()));
        assert_eq!(work_item.module_id.as_deref(), Some(module.id.as_str()));
        assert_eq!(work_item.capability_id.as_deref(), Some(capability.id.as_str()));
        assert_eq!(work_item.problem_statement, "Problem from alias");
        assert_eq!(work_item.acceptance_criteria, "Acceptance from alias");
        assert_eq!(work_item.work_item_type.to_string(), "story");
    }

    #[tokio::test]
    async fn list_work_items_applies_filters_and_pagination_from_command_layer() {
        let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_list").await;
        let state = app.state::<AppState>();

        let product = product_commands::create_product(
            state.clone(),
            "List Product".to_string(),
            "".to_string(),
            "".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("product should be created");

        for (index, status) in ["draft", "done", "draft"].iter().enumerate() {
            let item = create_work_item(
                state.clone(),
                Some(product.id.clone()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                format!("Item {index}"),
                "".to_string(),
                None,
                "".to_string(),
                "".to_string(),
                None,
                "".to_string(),
                "story".to_string(),
                None,
                "medium".to_string(),
                "medium".to_string(),
            )
            .await
            .expect("work item should be created");

            if *status != "draft" {
                update_work_item(
                    state.clone(),
                    item.id,
                    None,
                    None,
                    Some((*status).to_string()),
                    None,
                    None,
                    None,
                )
                .await
                .expect("status should update");
            }
        }

        let page = list_work_items(
            state,
            Some(product.id),
            None,
            None,
            None,
            None,
            Some("draft".to_string()),
            Some(1),
            Some(1),
        )
        .await
        .expect("work items should list");

        assert_eq!(page.len(), 1);
        assert_eq!(page[0].title, "Item 2");
        assert_eq!(page[0].status.to_string(), "draft");
    }
}

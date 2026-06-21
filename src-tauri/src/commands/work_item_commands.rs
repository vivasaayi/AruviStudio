use crate::domain::work_item::{ProductWorkItemSummary, WorkItem, WorkItemScopeSummary};
use crate::error::AppError;
use crate::persistence::work_item_repo;
use crate::state::AppState;
use serde::Deserialize;
use tauri::State;
use tracing::{debug, error, info};

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
    debug!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, limit = ?request.limit, offset = ?request.offset, "list_work_items requested");
    let result = work_item_repo::list_work_items_page(
        &state.db,
        work_item_repo::WorkItemListQuery {
            product_id: request.product_id.as_deref(),
            product_area_id: request.product_area_id.as_deref(),
            capability_id: request.capability_id.as_deref(),
            source_node_id: request.source_node_id.as_deref(),
            source_node_type: request.source_node_type.as_deref(),
            status: request.status.as_deref(),
            limit: request.limit,
            offset: request.offset,
        },
    )
    .await;
    if let Err(err) = &result {
        error!(product_id = ?request.product_id, product_area_id = ?request.product_area_id, capability_id = ?request.capability_id, source_node_id = ?request.source_node_id, source_node_type = ?request.source_node_type, status = ?request.status, limit = ?request.limit, offset = ?request.offset, error = %err, "list_work_items failed");
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
mod tests {
    use super::*;
    use crate::commands::product_commands;
    use crate::commands::test_helpers::make_test_app;
    use crate::domain::product::{Capability, Product, ProductArea};
    use tauri::test::MockRuntime;
    use tauri::Manager;

    async fn create_test_product(state: State<'_, AppState>, name: &str) -> Product {
        product_commands::create_product(
            state,
            product_commands::CreateProductCommand {
                name: name.to_string(),
                description: "".to_string(),
                vision: "".to_string(),
                goals: "[]".to_string(),
                tags: "[]".to_string(),
                lifecycle: None,
                health: None,
                owner_label: None,
                investment_status: None,
                roadmap: None,
                evidence: None,
            },
        )
        .await
        .expect("product should be created")
    }

    async fn create_test_product_area(
        state: State<'_, AppState>,
        product_id: String,
        name: &str,
    ) -> ProductArea {
        product_commands::create_product_area(
            state,
            product_commands::CreateProductAreaCommand {
                product_id,
                name: name.to_string(),
                description: "".to_string(),
                purpose: "".to_string(),
                node_kind: Some("product_area".to_string()),
                explanation: None,
                examples: None,
                implementation_notes: None,
                test_guidance: None,
            },
        )
        .await
        .expect("product_area should be created")
    }

    async fn create_test_capability(
        state: State<'_, AppState>,
        product_area_id: String,
        name: &str,
    ) -> Capability {
        product_commands::create_capability(
            state,
            product_commands::CreateCapabilityCommand {
                product_area_id,
                parent_capability_id: None,
                name: name.to_string(),
                description: "".to_string(),
                acceptance_criteria: "".to_string(),
                priority: "medium".to_string(),
                risk: "low".to_string(),
                technical_notes: "".to_string(),
                node_kind: Some("capability".to_string()),
                explanation: None,
                examples: None,
                implementation_notes: None,
                test_guidance: None,
            },
        )
        .await
        .expect("capability should be created")
    }

    #[tokio::test]
    async fn create_work_item_accepts_canonical_scope_fields() {
        let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_create").await;
        let state = app.state::<AppState>();

        let product = create_test_product(state.clone(), "Work Item Product").await;
        let product_area =
            create_test_product_area(state.clone(), product.id.clone(), "Area").await;
        let capability =
            create_test_capability(state.clone(), product_area.id.clone(), "Capability").await;

        let work_item = create_work_item(
            state,
            CreateWorkItemCommand {
                product_id: Some(product.id.clone()),
                product_area_id: Some(product_area.id.clone()),
                capability_id: Some(capability.id.clone()),
                source_node_id: Some(capability.id.clone()),
                source_node_type: Some("capability".to_string()),
                parent_work_item_id: None,
                title: "Canonical Work Item".to_string(),
                problem_statement: "Problem from canonical field".to_string(),
                description: "".to_string(),
                acceptance_criteria: "Acceptance from canonical field".to_string(),
                constraints: "".to_string(),
                work_item_type: "story".to_string(),
                priority: "high".to_string(),
                complexity: "medium".to_string(),
            },
        )
        .await
        .expect("work item should be created");

        assert_eq!(work_item.product_id.as_deref(), Some(product.id.as_str()));
        assert_eq!(
            work_item.product_area_id.as_deref(),
            Some(product_area.id.as_str())
        );
        assert_eq!(
            work_item.capability_id.as_deref(),
            Some(capability.id.as_str())
        );
        assert_eq!(work_item.problem_statement, "Problem from canonical field");
        assert_eq!(
            work_item.acceptance_criteria,
            "Acceptance from canonical field"
        );
        assert_eq!(work_item.work_item_type.to_string(), "story");
    }

    #[tokio::test]
    async fn list_work_items_applies_filters_and_pagination_from_command_layer() {
        let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_list").await;
        let state = app.state::<AppState>();

        let product = create_test_product(state.clone(), "List Product").await;

        for (index, status) in ["draft", "done", "draft"].iter().enumerate() {
            let item = create_work_item(
                state.clone(),
                CreateWorkItemCommand {
                    product_id: Some(product.id.clone()),
                    product_area_id: None,
                    capability_id: None,
                    source_node_id: None,
                    source_node_type: None,
                    parent_work_item_id: None,
                    title: format!("Item {index}"),
                    problem_statement: "".to_string(),
                    description: "".to_string(),
                    acceptance_criteria: "".to_string(),
                    constraints: "".to_string(),
                    work_item_type: "story".to_string(),
                    priority: "medium".to_string(),
                    complexity: "medium".to_string(),
                },
            )
            .await
            .expect("work item should be created");

            if *status != "draft" {
                update_work_item(
                    state.clone(),
                    UpdateWorkItemCommand {
                        id: item.id,
                        title: None,
                        description: None,
                        status: Some((*status).to_string()),
                        problem_statement: None,
                        acceptance_criteria: None,
                        constraints: None,
                    },
                )
                .await
                .expect("status should update");
            }
        }

        let page = list_work_items(
            state,
            ListWorkItemsCommand {
                product_id: Some(product.id),
                product_area_id: None,
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                status: Some("draft".to_string()),
                limit: Some(1),
                offset: Some(1),
            },
        )
        .await
        .expect("work items should list");

        assert_eq!(page.len(), 1);
        assert_eq!(page[0].title, "Item 2");
        assert_eq!(page[0].status.to_string(), "draft");
    }
}

use crate::domain::strategy::{ProductDependency, ProductStrategyLink, StrategyNode};
use crate::error::AppError;
use crate::persistence::strategy_repo;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_strategy_nodes(
    state: State<'_, AppState>,
) -> Result<Vec<StrategyNode>, AppError> {
    strategy_repo::list_strategy_nodes(&state.db).await
}

#[tauri::command]
pub async fn create_strategy_node(
    state: State<'_, AppState>,
    parent_node_id: Option<String>,
    node_kind: Option<String>,
    name: String,
    description: String,
    owner_label: Option<String>,
) -> Result<StrategyNode, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let node_kind = node_kind
        .ok_or_else(|| AppError::Validation("Strategy node kind is required.".to_string()))?;
    strategy_repo::create_strategy_node(
        &state.db,
        &id,
        parent_node_id.as_deref(),
        &node_kind,
        &name,
        &description,
        owner_label.as_deref().unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub async fn update_strategy_node(
    state: State<'_, AppState>,
    id: String,
    parent_node_id: Option<String>,
    clear_parent: Option<bool>,
    node_kind: Option<String>,
    name: Option<String>,
    description: Option<String>,
    owner_label: Option<String>,
) -> Result<StrategyNode, AppError> {
    let clear_parent = clear_parent.unwrap_or(false);
    let next_parent = if clear_parent {
        Some(None)
    } else {
        parent_node_id.as_deref().map(Some)
    };
    strategy_repo::update_strategy_node(
        &state.db,
        &id,
        next_parent,
        node_kind.as_deref(),
        name.as_deref(),
        description.as_deref(),
        owner_label.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn delete_strategy_node(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    strategy_repo::delete_strategy_node(&state.db, &id).await
}

#[tauri::command]
pub async fn list_product_strategy_links(
    state: State<'_, AppState>,
) -> Result<Vec<ProductStrategyLink>, AppError> {
    strategy_repo::list_product_strategy_links(&state.db).await
}

#[tauri::command]
pub async fn link_product_to_strategy(
    state: State<'_, AppState>,
    product_id: Option<String>,
    strategy_node_id: Option<String>,
    is_primary: Option<bool>,
) -> Result<ProductStrategyLink, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let product_id = product_id
        .ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
    let strategy_node_id = strategy_node_id
        .ok_or_else(|| AppError::Validation("Strategy node id is required.".to_string()))?;
    strategy_repo::link_product_to_strategy(
        &state.db,
        &id,
        &product_id,
        &strategy_node_id,
        is_primary.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn unlink_product_from_strategy(
    state: State<'_, AppState>,
    product_id: Option<String>,
    strategy_node_id: Option<String>,
) -> Result<(), AppError> {
    let product_id = product_id
        .ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
    let strategy_node_id = strategy_node_id
        .ok_or_else(|| AppError::Validation("Strategy node id is required.".to_string()))?;
    strategy_repo::unlink_product_from_strategy(&state.db, &product_id, &strategy_node_id).await
}

#[tauri::command]
pub async fn list_product_dependencies(
    state: State<'_, AppState>,
) -> Result<Vec<ProductDependency>, AppError> {
    strategy_repo::list_product_dependencies(&state.db).await
}

#[tauri::command]
pub async fn create_product_dependency(
    state: State<'_, AppState>,
    product_id: Option<String>,
    capability_id: Option<String>,
    depends_on_product_id: Option<String>,
    depends_on_capability_id: Option<String>,
    dependency_kind: Option<String>,
    description: String,
    status: Option<String>,
) -> Result<ProductDependency, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let product_id = product_id
        .ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
    let depends_on_product_id = depends_on_product_id
        .ok_or_else(|| AppError::Validation("Dependency product id is required.".to_string()))?;
    strategy_repo::create_product_dependency(
        &state.db,
        &id,
        &product_id,
        capability_id.as_deref(),
        &depends_on_product_id,
        depends_on_capability_id.as_deref(),
        dependency_kind.as_deref().unwrap_or("platform"),
        &description,
        status.as_deref().unwrap_or("active"),
    )
    .await
}

#[tauri::command]
pub async fn delete_product_dependency(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    strategy_repo::delete_product_dependency(&state.db, &id).await
}

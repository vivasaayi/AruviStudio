use crate::domain::strategy::{ProductDependency, ProductStrategyLink, StrategyNode};
use crate::error::AppError;
use crate::persistence::strategy_repo;
use crate::state::AppState;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct CreateProductDependencyCommand {
    #[serde(alias = "productId")]
    pub(crate) product_id: Option<String>,
    #[serde(alias = "capabilityId")]
    pub(crate) capability_id: Option<String>,
    #[serde(alias = "dependsOnProductId")]
    pub(crate) depends_on_product_id: Option<String>,
    #[serde(alias = "dependsOnCapabilityId")]
    pub(crate) depends_on_capability_id: Option<String>,
    #[serde(alias = "dependencyKind")]
    pub(crate) dependency_kind: Option<String>,
    pub(crate) description: String,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStrategyNodeCommand {
    pub(crate) id: String,
    #[serde(alias = "parentNodeId")]
    pub(crate) parent_node_id: Option<String>,
    #[serde(alias = "clearParent")]
    pub(crate) clear_parent: Option<bool>,
    #[serde(alias = "nodeKind")]
    pub(crate) node_kind: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(alias = "ownerLabel")]
    pub(crate) owner_label: Option<String>,
}

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
    request: UpdateStrategyNodeCommand,
) -> Result<StrategyNode, AppError> {
    let clear_parent = request.clear_parent.unwrap_or(false);
    let next_parent = if clear_parent {
        Some(None)
    } else {
        request.parent_node_id.as_deref().map(Some)
    };
    strategy_repo::update_strategy_node(
        &state.db,
        &request.id,
        next_parent,
        request.node_kind.as_deref(),
        request.name.as_deref(),
        request.description.as_deref(),
        request.owner_label.as_deref(),
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
    let product_id =
        product_id.ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
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
    let product_id =
        product_id.ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
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
    request: CreateProductDependencyCommand,
) -> Result<ProductDependency, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let product_id = request
        .product_id
        .ok_or_else(|| AppError::Validation("Product id is required.".to_string()))?;
    let depends_on_product_id = request
        .depends_on_product_id
        .ok_or_else(|| AppError::Validation("Dependency product id is required.".to_string()))?;
    strategy_repo::create_product_dependency(
        &state.db,
        strategy_repo::CreateProductDependencyInput {
            id: &id,
            product_id: &product_id,
            capability_id: request.capability_id.as_deref(),
            depends_on_product_id: &depends_on_product_id,
            depends_on_capability_id: request.depends_on_capability_id.as_deref(),
            dependency_kind: request.dependency_kind.as_deref().unwrap_or("platform"),
            description: &request.description,
            status: request.status.as_deref().unwrap_or("active"),
        },
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

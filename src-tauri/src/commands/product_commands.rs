use crate::domain::product::{Capability, Module, Product, ProductTree};
use crate::error::AppError;
use crate::persistence::product_repo;
use crate::state::AppState;
use tauri::State;
use tracing::{debug, error, info};

#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    name: String,
    description: String,
    vision: String,
    goals: String,
    tags: String,
) -> Result<Product, AppError> {
    info!(product_name = %name, "create_product requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        product_repo::create_product(&state.db, &id, &name, &description, &vision, &goals, &tags)
            .await;
    match &result {
        Ok(product) => info!(product_id = %product.id, "create_product succeeded"),
        Err(err) => error!(product_id = %id, error = %err, "create_product failed"),
    }
    result
}

#[tauri::command]
pub async fn get_product(state: State<'_, AppState>, id: String) -> Result<Product, AppError> {
    product_repo::get_product(&state.db, &id).await
}

#[tauri::command]
pub async fn list_products(state: State<'_, AppState>) -> Result<Vec<Product>, AppError> {
    product_repo::list_products(&state.db).await
}

#[tauri::command]
pub async fn update_product(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    vision: Option<String>,
    goals: Option<String>,
    tags: Option<String>,
) -> Result<Product, AppError> {
    info!(product_id = %id, "update_product requested");
    debug!(product_id = %id, has_name = name.is_some(), has_description = description.is_some(), has_vision = vision.is_some(), has_goals = goals.is_some(), has_tags = tags.is_some(), "update_product payload summary");
    let result = product_repo::update_product(
        &state.db,
        &id,
        name.as_deref(),
        description.as_deref(),
        vision.as_deref(),
        goals.as_deref(),
        tags.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(product_id = %id, "update_product succeeded"),
        Err(err) => error!(product_id = %id, error = %err, "update_product failed"),
    }
    result
}

#[tauri::command]
pub async fn archive_product(state: State<'_, AppState>, id: String) -> Result<Product, AppError> {
    product_repo::archive_product(&state.db, &id).await
}

#[tauri::command]
pub async fn create_module(
    state: State<'_, AppState>,
    product_id: String,
    name: String,
    description: String,
    purpose: String,
) -> Result<Module, AppError> {
    info!(product_id = %product_id, module_name = %name, "create_module requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        product_repo::create_module(&state.db, &id, &product_id, &name, &description, &purpose)
            .await;
    match &result {
        Ok(module) => {
            info!(module_id = %module.id, product_id = %module.product_id, "create_module succeeded")
        }
        Err(err) => {
            error!(module_id = %id, product_id = %product_id, error = %err, "create_module failed")
        }
    }
    result
}

#[tauri::command]
pub async fn list_modules(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<Module>, AppError> {
    product_repo::list_modules(&state.db, &product_id).await
}

#[tauri::command]
pub async fn update_module(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    purpose: Option<String>,
) -> Result<Module, AppError> {
    info!(module_id = %id, "update_module requested");
    debug!(module_id = %id, has_name = name.is_some(), has_description = description.is_some(), has_purpose = purpose.is_some(), "update_module payload summary");
    let result = product_repo::update_module(
        &state.db,
        &id,
        name.as_deref(),
        description.as_deref(),
        purpose.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(module_id = %id, "update_module succeeded"),
        Err(err) => error!(module_id = %id, error = %err, "update_module failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_module(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    product_repo::delete_module(&state.db, &id).await
}

#[tauri::command]
pub async fn reorder_modules(
    state: State<'_, AppState>,
    product_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    info!(product_id = %product_id, item_count = ordered_ids.len(), "reorder_modules requested");
    let result = product_repo::reorder_modules(&state.db, &product_id, &ordered_ids).await;
    match &result {
        Ok(_) => info!(product_id = %product_id, "reorder_modules succeeded"),
        Err(err) => error!(product_id = %product_id, error = %err, "reorder_modules failed"),
    }
    result
}

#[tauri::command]
pub async fn create_capability(
    state: State<'_, AppState>,
    module_id: String,
    parent_capability_id: Option<String>,
    name: String,
    description: String,
    acceptance_criteria: String,
    priority: String,
    risk: String,
    technical_notes: String,
) -> Result<Capability, AppError> {
    info!(module_id = %module_id, parent_capability_id = ?parent_capability_id, capability_name = %name, "create_capability requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = product_repo::create_capability(
        &state.db,
        &id,
        &module_id,
        parent_capability_id.as_deref(),
        &name,
        &description,
        &acceptance_criteria,
        &priority,
        &risk,
        &technical_notes,
    )
    .await;
    match &result {
        Ok(capability) => {
            info!(capability_id = %capability.id, module_id = %capability.module_id, parent_capability_id = ?capability.parent_capability_id, "create_capability succeeded")
        }
        Err(err) => {
            error!(capability_id = %id, module_id = %module_id, parent_capability_id = ?parent_capability_id, error = %err, "create_capability failed")
        }
    }
    result
}

#[tauri::command]
pub async fn list_capabilities(
    state: State<'_, AppState>,
    module_id: String,
) -> Result<Vec<Capability>, AppError> {
    product_repo::list_capabilities(&state.db, &module_id).await
}

#[tauri::command]
pub async fn update_capability(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    acceptance_criteria: Option<String>,
    priority: Option<String>,
    risk: Option<String>,
    technical_notes: Option<String>,
) -> Result<Capability, AppError> {
    info!(capability_id = %id, "update_capability requested");
    debug!(capability_id = %id, has_name = name.is_some(), has_description = description.is_some(), has_acceptance_criteria = acceptance_criteria.is_some(), has_priority = priority.is_some(), has_risk = risk.is_some(), has_technical_notes = technical_notes.is_some(), "update_capability payload summary");
    let result = product_repo::update_capability(
        &state.db,
        &id,
        name.as_deref(),
        description.as_deref(),
        acceptance_criteria.as_deref(),
        priority.as_deref(),
        risk.as_deref(),
        technical_notes.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(capability_id = %id, "update_capability succeeded"),
        Err(err) => error!(capability_id = %id, error = %err, "update_capability failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_capability(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    product_repo::delete_capability(&state.db, &id).await
}

#[tauri::command]
pub async fn reorder_capabilities(
    state: State<'_, AppState>,
    module_id: String,
    parent_capability_id: Option<String>,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    info!(module_id = %module_id, parent_capability_id = ?parent_capability_id, item_count = ordered_ids.len(), "reorder_capabilities requested");
    let result = product_repo::reorder_capabilities(
        &state.db,
        &module_id,
        parent_capability_id.as_deref(),
        &ordered_ids,
    )
    .await;
    match &result {
        Ok(_) => {
            info!(module_id = %module_id, parent_capability_id = ?parent_capability_id, "reorder_capabilities succeeded")
        }
        Err(err) => {
            error!(module_id = %module_id, parent_capability_id = ?parent_capability_id, error = %err, "reorder_capabilities failed")
        }
    }
    result
}

#[tauri::command]
pub async fn get_product_tree(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<ProductTree, AppError> {
    debug!(product_id = %product_id, "get_product_tree requested");
    let result = product_repo::get_product_tree(&state.db, &product_id).await;
    if let Err(err) = &result {
        error!(product_id = %product_id, error = %err, "get_product_tree failed");
    }
    result
}

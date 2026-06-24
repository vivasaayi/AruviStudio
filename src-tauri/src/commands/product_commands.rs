use crate::domain::product::{Product, ProductPlanResetResult, ProductReference};
use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::state::AppState;
use tauri::State;
use tracing::{debug, error, info};

pub mod bulk_import;
pub mod hierarchy;
mod payloads;
pub use payloads::{
    ApplySemanticTemplateCommand, CreateCapabilityCommand, CreateProductAreaCommand,
    CreateProductCommand, CreateProductReferenceCommand, UpdateCapabilityCommand,
    UpdateProductAreaCommand, UpdateProductCommand,
};

#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    request: CreateProductCommand,
) -> Result<Product, AppError> {
    info!(product_name = %request.name, "create_product requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = product_repo::create_product(
        &state.db,
        product_repo::CreateProductInput {
            id: &id,
            name: &request.name,
            description: &request.description,
            vision: &request.vision,
            goals: &request.goals,
            tags: &request.tags,
            lifecycle: request.lifecycle.as_deref(),
            health: request.health.as_deref(),
            owner_label: request.owner_label.as_deref(),
            investment_status: request.investment_status.as_deref(),
            roadmap: request.roadmap.as_deref(),
            evidence: request.evidence.as_deref(),
        },
    )
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
    let hide_examples =
        settings_repo::get_bool_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true).await?;
    let products = product_repo::list_products(&state.db).await?;
    if hide_examples {
        Ok(products
            .into_iter()
            .filter(|product| !product.is_example_product())
            .collect())
    } else {
        Ok(products)
    }
}

#[tauri::command]
pub async fn seed_example_products(state: State<'_, AppState>) -> Result<(), AppError> {
    info!("seed_example_products requested");
    product_service::initialize_example_catalog(&state.db).await
}

#[tauri::command]
pub async fn update_product(
    state: State<'_, AppState>,
    request: UpdateProductCommand,
) -> Result<Product, AppError> {
    info!(product_id = %request.id, "update_product requested");
    debug!(product_id = %request.id, has_name = request.name.is_some(), has_description = request.description.is_some(), has_vision = request.vision.is_some(), has_goals = request.goals.is_some(), has_tags = request.tags.is_some(), "update_product payload summary");
    let result = product_repo::update_product(
        &state.db,
        product_repo::UpdateProductPatch {
            id: &request.id,
            name: request.name.as_deref(),
            description: request.description.as_deref(),
            vision: request.vision.as_deref(),
            goals: request.goals.as_deref(),
            tags: request.tags.as_deref(),
            lifecycle: request.lifecycle.as_deref(),
            health: request.health.as_deref(),
            owner_label: request.owner_label.as_deref(),
            investment_status: request.investment_status.as_deref(),
            roadmap: request.roadmap.as_deref(),
            evidence: request.evidence.as_deref(),
        },
    )
    .await;
    match &result {
        Ok(_) => info!(product_id = %request.id, "update_product succeeded"),
        Err(err) => error!(product_id = %request.id, error = %err, "update_product failed"),
    }
    result
}

#[tauri::command]
pub async fn archive_product(state: State<'_, AppState>, id: String) -> Result<Product, AppError> {
    product_repo::archive_product(&state.db, &id).await
}

#[tauri::command]
pub async fn reset_product_plan(
    state: State<'_, AppState>,
    product_id: Option<String>,
    delete_delivery: Option<bool>,
) -> Result<ProductPlanResetResult, AppError> {
    let product_id =
        product_id.ok_or_else(|| AppError::Validation("missing product id".to_string()))?;
    let delete_delivery = delete_delivery.unwrap_or(false);
    product_repo::reset_product_plan(&state.db, &product_id, delete_delivery).await
}

#[tauri::command]
pub async fn list_product_references(
    state: State<'_, AppState>,
    scope_type: Option<String>,
    scope_id: Option<String>,
) -> Result<Vec<ProductReference>, AppError> {
    product_repo::list_product_references(&state.db, scope_type.as_deref(), scope_id.as_deref())
        .await
}

#[tauri::command]
pub async fn create_product_reference(
    state: State<'_, AppState>,
    request: CreateProductReferenceCommand,
) -> Result<ProductReference, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    product_repo::create_product_reference(
        &state.db,
        product_repo::CreateProductReferenceInput {
            id: &id,
            scope_type: &request.scope_type,
            scope_id: &request.scope_id,
            title: &request.title,
            reference_kind: &request.reference_kind,
            uri: request.uri.as_deref().unwrap_or_default(),
            content: request.content.as_deref().unwrap_or_default(),
        },
    )
    .await
}

#[tauri::command]
pub async fn delete_product_reference(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    product_repo::delete_product_reference(&state.db, &id).await
}

#[cfg(test)]
mod tests;

use crate::domain::bulk_import::{BulkImportJob, BulkImportJobStatus};
use crate::domain::product::{
    Capability, NodeKindConversionResult, Product, ProductArea, ProductPlanResetResult,
    ProductReference, ProductTree, ProductTreeSummary, SemanticTemplateApplicationResult,
};
use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo};
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::state::AppState;
use serde_json::Value;
use tauri::State;
use tracing::{debug, error, info};

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
pub async fn create_product_area(
    state: State<'_, AppState>,
    request: CreateProductAreaCommand,
) -> Result<ProductArea, AppError> {
    info!(product_id = %request.product_id, product_area_name = %request.name, "create_product_area requested");
    let id = uuid::Uuid::new_v4().to_string();
    let explanation = request.explanation.unwrap_or_default();
    let examples = request.examples.unwrap_or_default();
    let implementation_notes = request.implementation_notes.unwrap_or_default();
    let test_guidance = request.test_guidance.unwrap_or_default();
    let result = product_repo::create_product_area(
        &state.db,
        product_repo::CreateProductAreaInput {
            id: &id,
            product_id: &request.product_id,
            name: &request.name,
            description: &request.description,
            purpose: &request.purpose,
            node_kind: request.node_kind.as_deref(),
            explanation: &explanation,
            examples: &examples,
            implementation_notes: &implementation_notes,
            test_guidance: &test_guidance,
        },
    )
    .await;
    match &result {
        Ok(product_area) => {
            info!(product_area_id = %product_area.id, product_id = %product_area.product_id, "create_product_area succeeded")
        }
        Err(err) => {
            error!(product_area_id = %id, product_id = %request.product_id, error = %err, "create_product_area failed")
        }
    }
    result
}

#[tauri::command]
pub async fn list_product_areas(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ProductArea>, AppError> {
    product_repo::list_product_areas(&state.db, &product_id).await
}

#[tauri::command]
pub async fn update_product_area(
    state: State<'_, AppState>,
    request: UpdateProductAreaCommand,
) -> Result<ProductArea, AppError> {
    info!(product_area_id = %request.id, "update_product_area requested");
    debug!(product_area_id = %request.id, has_name = request.name.is_some(), has_description = request.description.is_some(), has_purpose = request.purpose.is_some(), has_node_kind = request.node_kind.is_some(), "update_product_area payload summary");
    let result = product_repo::update_product_area(
        &state.db,
        product_repo::UpdateProductAreaPatch {
            id: &request.id,
            name: request.name.as_deref(),
            description: request.description.as_deref(),
            purpose: request.purpose.as_deref(),
            node_kind: request.node_kind.as_deref(),
            explanation: request.explanation.as_deref(),
            examples: request.examples.as_deref(),
            implementation_notes: request.implementation_notes.as_deref(),
            test_guidance: request.test_guidance.as_deref(),
        },
    )
    .await;
    match &result {
        Ok(_) => info!(product_area_id = %request.id, "update_product_area succeeded"),
        Err(err) => {
            error!(product_area_id = %request.id, error = %err, "update_product_area failed")
        }
    }
    result
}

#[tauri::command]
pub async fn delete_product_area(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    product_repo::delete_product_area(&state.db, &id).await
}

#[tauri::command]
pub async fn reorder_product_areas(
    state: State<'_, AppState>,
    product_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    info!(product_id = %product_id, item_count = ordered_ids.len(), "reorder_product_areas requested");
    let result = product_repo::reorder_product_areas(&state.db, &product_id, &ordered_ids).await;
    match &result {
        Ok(_) => info!(product_id = %product_id, "reorder_product_areas succeeded"),
        Err(err) => error!(product_id = %product_id, error = %err, "reorder_product_areas failed"),
    }
    result
}

#[tauri::command]
pub async fn create_capability(
    state: State<'_, AppState>,
    request: CreateCapabilityCommand,
) -> Result<Capability, AppError> {
    info!(product_area_id = %request.product_area_id, parent_capability_id = ?request.parent_capability_id, capability_name = %request.name, "create_capability requested");
    let id = uuid::Uuid::new_v4().to_string();
    let explanation = request.explanation.unwrap_or_default();
    let examples = request.examples.unwrap_or_default();
    let implementation_notes = request.implementation_notes.unwrap_or_default();
    let test_guidance = request.test_guidance.unwrap_or_default();
    let result = product_repo::create_capability(
        &state.db,
        product_repo::CreateCapabilityInput {
            id: &id,
            product_area_id: &request.product_area_id,
            parent_capability_id: request.parent_capability_id.as_deref(),
            name: &request.name,
            description: &request.description,
            acceptance_criteria: &request.acceptance_criteria,
            priority: &request.priority,
            risk: &request.risk,
            technical_notes: &request.technical_notes,
            node_kind: request.node_kind.as_deref(),
            explanation: &explanation,
            examples: &examples,
            implementation_notes: &implementation_notes,
            test_guidance: &test_guidance,
        },
    )
    .await;
    match &result {
        Ok(capability) => {
            info!(capability_id = %capability.id, product_area_id = %capability.product_area_id, parent_capability_id = ?capability.parent_capability_id, "create_capability succeeded")
        }
        Err(err) => {
            error!(capability_id = %id, product_area_id = %request.product_area_id, parent_capability_id = ?request.parent_capability_id, error = %err, "create_capability failed")
        }
    }
    result
}

#[tauri::command]
pub async fn list_capabilities(
    state: State<'_, AppState>,
    product_area_id: String,
) -> Result<Vec<Capability>, AppError> {
    product_repo::list_capabilities(&state.db, &product_area_id).await
}

#[tauri::command]
pub async fn list_product_capabilities(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<Capability>, AppError> {
    product_repo::list_product_capabilities(&state.db, &product_id).await
}

#[tauri::command]
pub async fn get_capability(
    state: State<'_, AppState>,
    id: String,
) -> Result<Capability, AppError> {
    product_repo::get_capability(&state.db, &id).await
}

#[tauri::command]
pub async fn update_capability(
    state: State<'_, AppState>,
    request: UpdateCapabilityCommand,
) -> Result<Capability, AppError> {
    info!(capability_id = %request.id, "update_capability requested");
    debug!(capability_id = %request.id, has_name = request.name.is_some(), has_description = request.description.is_some(), has_acceptance_criteria = request.acceptance_criteria.is_some(), has_priority = request.priority.is_some(), has_risk = request.risk.is_some(), has_technical_notes = request.technical_notes.is_some(), has_node_kind = request.node_kind.is_some(), "update_capability payload summary");
    let result = product_repo::update_capability(
        &state.db,
        product_repo::UpdateCapabilityPatch {
            id: &request.id,
            name: request.name.as_deref(),
            description: request.description.as_deref(),
            acceptance_criteria: request.acceptance_criteria.as_deref(),
            priority: request.priority.as_deref(),
            risk: request.risk.as_deref(),
            technical_notes: request.technical_notes.as_deref(),
            node_kind: request.node_kind.as_deref(),
            explanation: request.explanation.as_deref(),
            examples: request.examples.as_deref(),
            implementation_notes: request.implementation_notes.as_deref(),
            test_guidance: request.test_guidance.as_deref(),
        },
    )
    .await;
    match &result {
        Ok(_) => info!(capability_id = %request.id, "update_capability succeeded"),
        Err(err) => error!(capability_id = %request.id, error = %err, "update_capability failed"),
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
    product_area_id: String,
    parent_capability_id: Option<String>,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    info!(product_area_id = %product_area_id, parent_capability_id = ?parent_capability_id, item_count = ordered_ids.len(), "reorder_capabilities requested");
    let result = product_repo::reorder_capabilities(
        &state.db,
        &product_area_id,
        parent_capability_id.as_deref(),
        &ordered_ids,
    )
    .await;
    match &result {
        Ok(_) => {
            info!(product_area_id = %product_area_id, parent_capability_id = ?parent_capability_id, "reorder_capabilities succeeded")
        }
        Err(err) => {
            error!(product_area_id = %product_area_id, parent_capability_id = ?parent_capability_id, error = %err, "reorder_capabilities failed")
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

#[tauri::command]
pub async fn summarize_product_tree(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<ProductTreeSummary, AppError> {
    product_repo::summarize_product_tree(&state.db, &product_id).await
}

#[tauri::command]
pub async fn get_bulk_import_schema() -> Result<Value, AppError> {
    Ok(bulk_import_service::bulk_import_schema())
}

#[tauri::command]
pub async fn submit_bulk_import(
    state: State<'_, AppState>,
    file_path: Option<String>,
    format: Option<String>,
    product_id: Option<String>,
) -> Result<BulkImportJob, AppError> {
    let file_path =
        file_path.ok_or_else(|| AppError::Validation("missing file path".to_string()))?;
    bulk_import_service::submit_bulk_import(
        state.inner().clone(),
        BulkImportRequest {
            file_path,
            format,
            product_id,
        },
    )
    .await
}

#[tauri::command]
pub async fn get_bulk_import_status(
    state: State<'_, AppState>,
    job_id: Option<String>,
) -> Result<BulkImportJobStatus, AppError> {
    let job_id = job_id.ok_or_else(|| AppError::Validation("missing job id".to_string()))?;
    bulk_import_service::get_bulk_import_status(&state.db, &job_id).await
}

#[tauri::command]
pub async fn list_bulk_import_jobs(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<BulkImportJob>, AppError> {
    bulk_import_service::list_bulk_import_jobs(&state.db, limit).await
}

#[tauri::command]
pub async fn apply_semantic_template(
    state: State<'_, AppState>,
    request: ApplySemanticTemplateCommand,
) -> Result<SemanticTemplateApplicationResult, AppError> {
    let product_area_id = request
        .product_area_id
        .ok_or_else(|| AppError::Validation("missing product_area id".to_string()))?;
    let template_kind = request
        .template_kind
        .ok_or_else(|| AppError::Validation("missing template kind".to_string()))?;
    product_service::apply_semantic_template(
        &state.db,
        product_service::ApplySemanticTemplateInput {
            product_area_id: &product_area_id,
            parent_capability_id: request.parent_capability_id.as_deref(),
            template_kind: &template_kind,
            name: &request.name,
            description: request.description.as_deref().unwrap_or_default(),
            priority: request.priority.as_deref(),
            risk: request.risk.as_deref(),
            explanation: request.explanation.as_deref().unwrap_or_default(),
            examples: request.examples.as_deref().unwrap_or_default(),
            implementation_notes: request.implementation_notes.as_deref().unwrap_or_default(),
            test_guidance: request.test_guidance.as_deref().unwrap_or_default(),
        },
    )
    .await
}

#[tauri::command]
pub async fn convert_capability_kind(
    state: State<'_, AppState>,
    id: String,
    node_kind: Option<String>,
    child_strategy: Option<String>,
) -> Result<NodeKindConversionResult, AppError> {
    let node_kind =
        node_kind.ok_or_else(|| AppError::Validation("missing node kind".to_string()))?;
    product_service::convert_capability_kind(&state.db, &id, &node_kind, child_strategy.as_deref())
        .await
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

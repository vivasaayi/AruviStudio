use crate::domain::bulk_import::{BulkImportJob, BulkImportJobStatus};
use crate::domain::product::{
    Capability, Module, NodeKindConversionResult, Product, ProductPlanResetResult,
    ProductReference, ProductTree, SemanticTemplateApplicationResult,
};
use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo};
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::state::AppState;
use serde_json::Value;
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
    lifecycle: Option<String>,
    health: Option<String>,
    owner_label: Option<String>,
    investment_status: Option<String>,
    roadmap: Option<String>,
    evidence: Option<String>,
) -> Result<Product, AppError> {
    info!(product_name = %name, "create_product requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = product_repo::create_product(
        &state.db,
        &id,
        &name,
        &description,
        &vision,
        &goals,
        &tags,
        lifecycle.as_deref(),
        health.as_deref(),
        owner_label.as_deref(),
        investment_status.as_deref(),
        roadmap.as_deref(),
        evidence.as_deref(),
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
    id: String,
    name: Option<String>,
    description: Option<String>,
    vision: Option<String>,
    goals: Option<String>,
    tags: Option<String>,
    lifecycle: Option<String>,
    health: Option<String>,
    owner_label: Option<String>,
    investment_status: Option<String>,
    roadmap: Option<String>,
    evidence: Option<String>,
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
        lifecycle.as_deref(),
        health.as_deref(),
        owner_label.as_deref(),
        investment_status.as_deref(),
        roadmap.as_deref(),
        evidence.as_deref(),
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
#[allow(non_snake_case)]
pub async fn reset_product_plan(
    state: State<'_, AppState>,
    product_id: Option<String>,
    productId: Option<String>,
    delete_delivery: Option<bool>,
    deleteDelivery: Option<bool>,
) -> Result<ProductPlanResetResult, AppError> {
    let product_id = product_id
        .or(productId)
        .ok_or_else(|| AppError::Validation("missing product id".to_string()))?;
    let delete_delivery = delete_delivery.or(deleteDelivery).unwrap_or(false);
    product_repo::reset_product_plan(&state.db, &product_id, delete_delivery).await
}

#[tauri::command]
pub async fn create_module(
    state: State<'_, AppState>,
    product_id: String,
    name: String,
    description: String,
    purpose: String,
    node_kind: Option<String>,
    explanation: Option<String>,
    examples: Option<String>,
    implementation_notes: Option<String>,
    implementationNotes: Option<String>,
    test_guidance: Option<String>,
    testGuidance: Option<String>,
) -> Result<Module, AppError> {
    info!(product_id = %product_id, module_name = %name, "create_module requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = product_repo::create_module(
        &state.db,
        &id,
        &product_id,
        &name,
        &description,
        &purpose,
        node_kind.as_deref(),
        explanation.as_deref().unwrap_or_default(),
        examples.as_deref().unwrap_or_default(),
        implementation_notes
            .or(implementationNotes)
            .as_deref()
            .unwrap_or_default(),
        test_guidance
            .or(testGuidance)
            .as_deref()
            .unwrap_or_default(),
    )
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
    node_kind: Option<String>,
    explanation: Option<String>,
    examples: Option<String>,
    implementation_notes: Option<String>,
    implementationNotes: Option<String>,
    test_guidance: Option<String>,
    testGuidance: Option<String>,
) -> Result<Module, AppError> {
    info!(module_id = %id, "update_module requested");
    debug!(module_id = %id, has_name = name.is_some(), has_description = description.is_some(), has_purpose = purpose.is_some(), has_node_kind = node_kind.is_some(), "update_module payload summary");
    let result = product_repo::update_module(
        &state.db,
        &id,
        name.as_deref(),
        description.as_deref(),
        purpose.as_deref(),
        node_kind.as_deref(),
        explanation.as_deref(),
        examples.as_deref(),
        implementation_notes
            .as_deref()
            .or(implementationNotes.as_deref()),
        test_guidance.as_deref().or(testGuidance.as_deref()),
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
    node_kind: Option<String>,
    explanation: Option<String>,
    examples: Option<String>,
    implementation_notes: Option<String>,
    implementationNotes: Option<String>,
    test_guidance: Option<String>,
    testGuidance: Option<String>,
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
        node_kind.as_deref(),
        explanation.as_deref().unwrap_or_default(),
        examples.as_deref().unwrap_or_default(),
        implementation_notes
            .or(implementationNotes)
            .as_deref()
            .unwrap_or_default(),
        test_guidance
            .or(testGuidance)
            .as_deref()
            .unwrap_or_default(),
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
    node_kind: Option<String>,
    explanation: Option<String>,
    examples: Option<String>,
    implementation_notes: Option<String>,
    implementationNotes: Option<String>,
    test_guidance: Option<String>,
    testGuidance: Option<String>,
) -> Result<Capability, AppError> {
    info!(capability_id = %id, "update_capability requested");
    debug!(capability_id = %id, has_name = name.is_some(), has_description = description.is_some(), has_acceptance_criteria = acceptance_criteria.is_some(), has_priority = priority.is_some(), has_risk = risk.is_some(), has_technical_notes = technical_notes.is_some(), has_node_kind = node_kind.is_some(), "update_capability payload summary");
    let result = product_repo::update_capability(
        &state.db,
        &id,
        name.as_deref(),
        description.as_deref(),
        acceptance_criteria.as_deref(),
        priority.as_deref(),
        risk.as_deref(),
        technical_notes.as_deref(),
        node_kind.as_deref(),
        explanation.as_deref(),
        examples.as_deref(),
        implementation_notes
            .as_deref()
            .or(implementationNotes.as_deref()),
        test_guidance.as_deref().or(testGuidance.as_deref()),
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

#[tauri::command]
pub async fn get_bulk_import_schema() -> Result<Value, AppError> {
    Ok(bulk_import_service::bulk_import_schema())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn submit_bulk_import(
    state: State<'_, AppState>,
    file_path: Option<String>,
    filePath: Option<String>,
    format: Option<String>,
    product_id: Option<String>,
    productId: Option<String>,
) -> Result<BulkImportJob, AppError> {
    let file_path = file_path
        .or(filePath)
        .ok_or_else(|| AppError::Validation("missing file path".to_string()))?;
    bulk_import_service::submit_bulk_import(
        state.inner().clone(),
        BulkImportRequest {
            file_path,
            format,
            product_id: product_id.or(productId),
        },
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_bulk_import_status(
    state: State<'_, AppState>,
    job_id: Option<String>,
    jobId: Option<String>,
) -> Result<BulkImportJobStatus, AppError> {
    let job_id = job_id
        .or(jobId)
        .ok_or_else(|| AppError::Validation("missing job id".to_string()))?;
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
#[allow(non_snake_case)]
pub async fn apply_semantic_template(
    state: State<'_, AppState>,
    module_id: Option<String>,
    moduleId: Option<String>,
    parent_capability_id: Option<String>,
    parentCapabilityId: Option<String>,
    template_kind: Option<String>,
    templateKind: Option<String>,
    name: String,
    description: Option<String>,
    priority: Option<String>,
    risk: Option<String>,
    explanation: Option<String>,
    examples: Option<String>,
    implementation_notes: Option<String>,
    implementationNotes: Option<String>,
    test_guidance: Option<String>,
    testGuidance: Option<String>,
) -> Result<SemanticTemplateApplicationResult, AppError> {
    let module_id = module_id
        .or(moduleId)
        .ok_or_else(|| AppError::Validation("missing module id".to_string()))?;
    let template_kind = template_kind
        .or(templateKind)
        .ok_or_else(|| AppError::Validation("missing template kind".to_string()))?;
    product_service::apply_semantic_template(
        &state.db,
        &module_id,
        parent_capability_id.or(parentCapabilityId).as_deref(),
        &template_kind,
        &name,
        description.as_deref().unwrap_or_default(),
        priority.as_deref(),
        risk.as_deref(),
        explanation.as_deref().unwrap_or_default(),
        examples.as_deref().unwrap_or_default(),
        implementation_notes
            .or(implementationNotes)
            .as_deref()
            .unwrap_or_default(),
        test_guidance
            .or(testGuidance)
            .as_deref()
            .unwrap_or_default(),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn convert_capability_kind(
    state: State<'_, AppState>,
    id: String,
    node_kind: Option<String>,
    nodeKind: Option<String>,
    child_strategy: Option<String>,
    childStrategy: Option<String>,
) -> Result<NodeKindConversionResult, AppError> {
    let node_kind = node_kind
        .or(nodeKind)
        .ok_or_else(|| AppError::Validation("missing node kind".to_string()))?;
    product_service::convert_capability_kind(
        &state.db,
        &id,
        &node_kind,
        child_strategy.or(childStrategy).as_deref(),
    )
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
    scope_type: String,
    scope_id: String,
    title: String,
    reference_kind: String,
    uri: Option<String>,
    content: Option<String>,
) -> Result<ProductReference, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    product_repo::create_product_reference(
        &state.db,
        &id,
        &scope_type,
        &scope_id,
        &title,
        &reference_kind,
        uri.as_deref().unwrap_or_default(),
        content.as_deref().unwrap_or_default(),
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

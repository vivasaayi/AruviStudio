use crate::domain::bulk_import::{
    BulkImportCapabilityRow, BulkImportProductAreaRow, BulkImportProductRow, BulkImportRows,
    BulkImportWorkItemRow,
};
use crate::error::AppError;
use crate::services::bulk_import_context::{
    CapabilityScope, ImportBuildContext, ProductAreaScope, WorkItemScope,
};
use crate::services::bulk_import_csv_builder;
use crate::services::bulk_import_document::{
    ImportCapability, ImportDocument, ImportProduct, ImportProductArea, ImportWorkItem,
};
use crate::services::bulk_import_normalization::{
    capability_sort_key, clean_option, clean_ref, new_id, next_sort, normalize_node_kind,
    normalize_source_node_type, normalize_value, normalize_work_item_type, required_clean,
    work_item_sort_key, COMPLEXITIES, PRIORITIES, PRODUCT_HEALTHS, PRODUCT_INVESTMENT_STATUSES,
    PRODUCT_LIFECYCLES, RISKS, WORK_ITEM_STATUSES,
};
use std::collections::HashSet;

pub(super) struct PreparedImport {
    pub(super) rows: BulkImportRows,
    pub(super) required_existing_product_ids: HashSet<String>,
}

pub(super) fn prepare_json_import(
    content: &str,
    request_product_id: Option<&str>,
) -> Result<PreparedImport, AppError> {
    let document: ImportDocument = serde_json::from_str(content)?;
    let mut ctx = ImportBuildContext::default();
    let product_id = match document.product {
        Some(product) => push_product(&mut ctx, product, request_product_id)?,
        None => request_product_id.map(ToString::to_string).ok_or_else(|| {
            AppError::Validation(
                "JSON import must include product or submit with productId.".to_string(),
            )
        })?,
    };
    if !ctx.imported_product_ids.contains(&product_id) {
        ctx.referenced_product_ids.insert(product_id.clone());
    }

    for area in document.product_areas {
        push_product_area(&mut ctx, &product_id, area)?;
    }

    let product_scope = WorkItemScope {
        product_id: product_id.clone(),
        product_area_id: None,
        capability_id: None,
        source_node_id: None,
        source_node_type: None,
    };
    push_work_items(&mut ctx, product_scope, document.work_items, None, "story")?;

    Ok(finish_prepared_import(ctx))
}

pub(super) fn prepare_csv_import(
    content: &str,
    request_product_id: Option<&str>,
) -> Result<PreparedImport, AppError> {
    bulk_import_csv_builder::prepare_csv_import(content, request_product_id)
}

pub(super) fn finish_prepared_import(ctx: ImportBuildContext) -> PreparedImport {
    let required_existing_product_ids = ctx
        .referenced_product_ids
        .difference(&ctx.imported_product_ids)
        .cloned()
        .collect();
    PreparedImport {
        rows: ctx.rows,
        required_existing_product_ids,
    }
}

pub(super) fn push_product(
    ctx: &mut ImportBuildContext,
    product: ImportProduct,
    request_product_id: Option<&str>,
) -> Result<String, AppError> {
    let explicit_id = clean_option(product.id);
    if let (Some(request_id), Some(product_id)) = (request_product_id, explicit_id.as_deref()) {
        if request_id != product_id {
            return Err(AppError::Validation(format!(
                "Submitted productId '{request_id}' does not match import product id '{product_id}'."
            )));
        }
    }
    let id = explicit_id
        .or_else(|| request_product_id.map(ToString::to_string))
        .unwrap_or_else(new_id);
    let name = required_clean(product.name, "product.name")?;
    ctx.rows.products.push(BulkImportProductRow {
        id: id.clone(),
        name,
        description: clean_option(product.description).unwrap_or_default(),
        vision: clean_option(product.vision).unwrap_or_default(),
        goals_json: serde_json::to_string(&product.goals)?,
        tags_json: serde_json::to_string(&product.tags)?,
        lifecycle: normalize_value(
            product.lifecycle.as_deref(),
            "incubating",
            PRODUCT_LIFECYCLES,
            "product.lifecycle",
        )?,
        health: normalize_value(
            product.health.as_deref(),
            "unknown",
            PRODUCT_HEALTHS,
            "product.health",
        )?,
        owner_label: clean_option(product.owner_label).unwrap_or_default(),
        investment_status: normalize_value(
            product.investment_status.as_deref(),
            "evaluate",
            PRODUCT_INVESTMENT_STATUSES,
            "product.investmentStatus",
        )?,
        roadmap: clean_option(product.roadmap).unwrap_or_default(),
        evidence: clean_option(product.evidence).unwrap_or_default(),
    });
    ctx.imported_product_ids.insert(id.clone());
    Ok(id)
}

pub(super) fn push_product_area(
    ctx: &mut ImportBuildContext,
    product_id: &str,
    area: ImportProductArea,
) -> Result<String, AppError> {
    let id = clean_option(area.id).unwrap_or_else(new_id);
    let sort_order = next_sort(&mut ctx.product_area_sort, product_id);
    ctx.rows.product_areas.push(BulkImportProductAreaRow {
        id: id.clone(),
        product_id: product_id.to_string(),
        name: required_clean(area.name, "productArea.name")?,
        description: clean_option(area.description).unwrap_or_default(),
        purpose: clean_option(area.purpose).unwrap_or_default(),
        explanation: clean_option(area.explanation).unwrap_or_default(),
        examples: clean_option(area.examples).unwrap_or_default(),
        implementation_notes: clean_option(area.implementation_notes).unwrap_or_default(),
        test_guidance: clean_option(area.test_guidance).unwrap_or_default(),
        sort_order,
    });
    ctx.referenced_product_ids.insert(product_id.to_string());
    ctx.product_areas.insert(
        id.clone(),
        ProductAreaScope {
            product_id: product_id.to_string(),
            product_area_id: id.clone(),
        },
    );

    for capability in area.capabilities {
        push_capability(ctx, product_id, &id, None, 0, "capability", capability)?;
    }
    let area_scope = WorkItemScope {
        product_id: product_id.to_string(),
        product_area_id: Some(id.clone()),
        capability_id: None,
        source_node_id: Some(id.clone()),
        source_node_type: Some("product_area".to_string()),
    };
    push_work_items(ctx, area_scope, area.work_items, None, "story")?;
    Ok(id)
}

pub(super) fn push_capability(
    ctx: &mut ImportBuildContext,
    product_id: &str,
    product_area_id: &str,
    parent_capability_id: Option<&str>,
    level: i64,
    default_kind: &str,
    capability: ImportCapability,
) -> Result<String, AppError> {
    let node_kind = normalize_node_kind(capability.node_kind.as_deref(), default_kind)?;
    if parent_capability_id.is_none() && node_kind != "capability" {
        return Err(AppError::Validation(
            "Product areas can only contain capability nodes.".to_string(),
        ));
    }
    if parent_capability_id.is_some() && node_kind != "feature" {
        return Err(AppError::Validation(
            "Capabilities can only contain feature nodes.".to_string(),
        ));
    }

    let id = clean_option(capability.id).unwrap_or_else(new_id);
    let sort_key = capability_sort_key(product_area_id, parent_capability_id);
    let sort_order = next_sort(&mut ctx.capability_sort, &sort_key);
    ctx.rows.capabilities.push(BulkImportCapabilityRow {
        id: id.clone(),
        product_area_id: product_area_id.to_string(),
        parent_capability_id: parent_capability_id.map(ToString::to_string),
        level,
        node_kind: node_kind.clone(),
        sort_order,
        name: required_clean(capability.name, "capability.name")?,
        description: clean_option(capability.description).unwrap_or_default(),
        acceptance_criteria: clean_option(capability.acceptance_criteria).unwrap_or_default(),
        explanation: clean_option(capability.explanation).unwrap_or_default(),
        examples: clean_option(capability.examples).unwrap_or_default(),
        priority: normalize_value(
            capability.priority.as_deref(),
            "medium",
            PRIORITIES,
            "capability.priority",
        )?,
        risk: normalize_value(
            capability.risk.as_deref(),
            "medium",
            RISKS,
            "capability.risk",
        )?,
        technical_notes: clean_option(capability.technical_notes).unwrap_or_default(),
        implementation_notes: clean_option(capability.implementation_notes).unwrap_or_default(),
        test_guidance: clean_option(capability.test_guidance).unwrap_or_default(),
    });
    ctx.capabilities.insert(
        id.clone(),
        CapabilityScope {
            product_id: product_id.to_string(),
            product_area_id: product_area_id.to_string(),
            capability_id: id.clone(),
        },
    );

    if node_kind == "feature" {
        if !capability.features.is_empty() || !capability.capabilities.is_empty() {
            return Err(AppError::Validation(format!(
                "Feature '{}' cannot contain child capability or feature nodes.",
                id
            )));
        }
    } else {
        for feature in capability.features {
            push_capability(
                ctx,
                product_id,
                product_area_id,
                Some(&id),
                level + 1,
                "feature",
                feature,
            )?;
        }
        for feature in capability.capabilities {
            push_capability(
                ctx,
                product_id,
                product_area_id,
                Some(&id),
                level + 1,
                "feature",
                feature,
            )?;
        }
    }

    let work_scope = WorkItemScope {
        product_id: product_id.to_string(),
        product_area_id: Some(product_area_id.to_string()),
        capability_id: Some(id.clone()),
        source_node_id: Some(id.clone()),
        source_node_type: Some("capability".to_string()),
    };
    push_work_items(ctx, work_scope, capability.work_items, None, "story")?;
    Ok(id)
}

pub(super) fn push_work_items(
    ctx: &mut ImportBuildContext,
    inherited_scope: WorkItemScope,
    items: Vec<ImportWorkItem>,
    parent_work_item_id: Option<&str>,
    default_type: &str,
) -> Result<(), AppError> {
    for item in items {
        let scope = resolve_work_item_scope(ctx, &inherited_scope, &item)?;
        let id = clean_option(item.id).unwrap_or_else(new_id);
        let title = required_clean(item.title.or(item.name), "workItem.title")?;
        let raw_type = item.work_item_type.as_deref().unwrap_or(default_type);
        let work_item_type = normalize_work_item_type(raw_type)?;
        let parent = clean_option(item.parent_work_item_id)
            .or_else(|| parent_work_item_id.map(ToString::to_string));
        let sort_key = work_item_sort_key(
            &scope.product_id,
            scope.source_node_type.as_deref(),
            scope.source_node_id.as_deref(),
            parent.as_deref(),
        );
        let sort_order = next_sort(&mut ctx.work_item_sort, &sort_key);
        ctx.rows.work_items.push(BulkImportWorkItemRow {
            id: id.clone(),
            product_id: scope.product_id.clone(),
            product_area_id: scope.product_area_id.clone(),
            capability_id: scope.capability_id.clone(),
            source_node_id: scope.source_node_id.clone(),
            source_node_type: scope.source_node_type.clone(),
            parent_work_item_id: parent.clone(),
            title,
            problem_statement: clean_option(item.problem_statement).unwrap_or_default(),
            description: clean_option(item.description).unwrap_or_default(),
            acceptance_criteria: clean_option(item.acceptance_criteria).unwrap_or_default(),
            constraints: clean_option(item.constraints).unwrap_or_default(),
            work_item_type,
            priority: normalize_value(
                item.priority.as_deref(),
                "medium",
                PRIORITIES,
                "workItem.priority",
            )?,
            complexity: normalize_value(
                item.complexity.as_deref(),
                "medium",
                COMPLEXITIES,
                "workItem.complexity",
            )?,
            status: normalize_value(
                item.status.as_deref(),
                "draft",
                WORK_ITEM_STATUSES,
                "workItem.status",
            )?,
            sort_order,
        });
        ctx.referenced_product_ids.insert(scope.product_id.clone());
        ctx.work_items.insert(id.clone(), scope.clone());
        push_work_items(ctx, scope, item.tasks, Some(&id), "task")?;
    }
    Ok(())
}

fn resolve_work_item_scope(
    ctx: &ImportBuildContext,
    inherited_scope: &WorkItemScope,
    item: &ImportWorkItem,
) -> Result<WorkItemScope, AppError> {
    if let Some(parent_work_item_id) = clean_ref(item.parent_work_item_id.as_deref()) {
        if let Some(scope) = ctx.work_items.get(&parent_work_item_id) {
            return Ok(scope.clone());
        }
    }
    if let Some(feature_id) =
        clean_ref(item.feature_id.as_deref()).or_else(|| clean_ref(item.capability_id.as_deref()))
    {
        if let Some(scope) = ctx.capabilities.get(&feature_id) {
            return Ok(WorkItemScope {
                product_id: clean_ref(item.product_id.as_deref())
                    .unwrap_or_else(|| scope.product_id.clone()),
                product_area_id: Some(scope.product_area_id.clone()),
                capability_id: Some(scope.capability_id.clone()),
                source_node_id: Some(scope.capability_id.clone()),
                source_node_type: Some("capability".to_string()),
            });
        }
        let product_id = clean_ref(item.product_id.as_deref())
            .unwrap_or_else(|| inherited_scope.product_id.clone());
        return Ok(WorkItemScope {
            product_id,
            product_area_id: clean_ref(item.product_area_id.as_deref()),
            capability_id: Some(feature_id.clone()),
            source_node_id: Some(feature_id),
            source_node_type: Some("capability".to_string()),
        });
    }
    if let Some(area_id) = clean_ref(item.product_area_id.as_deref()) {
        if let Some(scope) = ctx.product_areas.get(&area_id) {
            return Ok(WorkItemScope {
                product_id: clean_ref(item.product_id.as_deref())
                    .unwrap_or_else(|| scope.product_id.clone()),
                product_area_id: Some(scope.product_area_id.clone()),
                capability_id: None,
                source_node_id: Some(scope.product_area_id.clone()),
                source_node_type: Some("product_area".to_string()),
            });
        }
        return Ok(WorkItemScope {
            product_id: clean_ref(item.product_id.as_deref())
                .unwrap_or_else(|| inherited_scope.product_id.clone()),
            product_area_id: Some(area_id.clone()),
            capability_id: None,
            source_node_id: Some(area_id),
            source_node_type: Some("product_area".to_string()),
        });
    }
    if let Some(source_node_id) = clean_ref(item.source_node_id.as_deref()) {
        let source_node_type =
            normalize_source_node_type(item.source_node_type.as_deref().unwrap_or("capability"))?;
        return Ok(WorkItemScope {
            product_id: clean_ref(item.product_id.as_deref())
                .unwrap_or_else(|| inherited_scope.product_id.clone()),
            product_area_id: inherited_scope.product_area_id.clone(),
            capability_id: if source_node_type == "capability" {
                Some(source_node_id.clone())
            } else {
                None
            },
            source_node_id: Some(source_node_id),
            source_node_type: Some(source_node_type),
        });
    }
    Ok(WorkItemScope {
        product_id: clean_ref(item.product_id.as_deref())
            .unwrap_or_else(|| inherited_scope.product_id.clone()),
        ..inherited_scope.clone()
    })
}

use crate::error::AppError;
use crate::services::bulk_import_builder::{
    finish_prepared_import, push_capability, push_product, push_product_area, push_work_items,
    PreparedImport,
};
use crate::services::bulk_import_context::{ImportBuildContext, WorkItemScope};
use crate::services::bulk_import_csv::{
    csv_error, csv_field, csv_list_field, normalize_record_type, parse_csv_records, CsvRecord,
};
use crate::services::bulk_import_document::{
    ImportCapability, ImportProduct, ImportProductArea, ImportWorkItem,
};
use crate::services::bulk_import_normalization::new_id;

pub(super) fn prepare_csv_import(
    content: &str,
    request_product_id: Option<&str>,
) -> Result<PreparedImport, AppError> {
    let records = parse_csv_records(content)?;
    let mut ctx = ImportBuildContext::default();
    for record in records {
        let record_type = csv_field(&record, &["record_type", "type", "kind"])
            .ok_or_else(|| csv_error(&record, "missing record_type"))?;
        match normalize_record_type(&record_type).as_str() {
            "product" => push_csv_product(&mut ctx, &record, request_product_id)?,
            "product_area" => push_csv_product_area(&mut ctx, &record, request_product_id)?,
            "capability" => push_csv_capability(&mut ctx, &record, request_product_id)?,
            "feature" => push_csv_feature(&mut ctx, &record, request_product_id)?,
            "work_item" | "story" | "task" => {
                push_csv_work_item(&mut ctx, &record, request_product_id)?
            }
            other => {
                return Err(csv_error(
                    &record,
                    &format!("unsupported record_type '{other}'"),
                ))
            }
        }
    }
    Ok(finish_prepared_import(ctx))
}

fn push_csv_product(
    ctx: &mut ImportBuildContext,
    record: &CsvRecord,
    request_product_id: Option<&str>,
) -> Result<(), AppError> {
    let id = csv_field(record, &["id", "product_id"])
        .or_else(|| request_product_id.map(ToString::to_string))
        .unwrap_or_else(new_id);
    let name =
        csv_field(record, &["name"]).ok_or_else(|| csv_error(record, "missing product name"))?;
    let product = ImportProduct {
        id: Some(id),
        name: Some(name),
        description: csv_field(record, &["description"]),
        vision: csv_field(record, &["vision"]),
        goals: csv_list_field(record, &["goals"]),
        tags: csv_list_field(record, &["tags"]),
        lifecycle: csv_field(record, &["lifecycle"]),
        health: csv_field(record, &["health"]),
        owner_label: csv_field(record, &["owner_label", "ownerLabel"]),
        investment_status: csv_field(record, &["investment_status", "investmentStatus"]),
        roadmap: csv_field(record, &["roadmap"]),
        evidence: csv_field(record, &["evidence"]),
    };
    push_product(ctx, product, request_product_id)?;
    Ok(())
}

fn push_csv_product_area(
    ctx: &mut ImportBuildContext,
    record: &CsvRecord,
    request_product_id: Option<&str>,
) -> Result<(), AppError> {
    let product_id = csv_field(record, &["product_id", "productId"])
        .or_else(|| request_product_id.map(ToString::to_string))
        .ok_or_else(|| csv_error(record, "missing product_id for product_area"))?;
    let id = csv_field(record, &["id", "product_area_id"]).unwrap_or_else(new_id);
    let area = ImportProductArea {
        id: Some(id),
        name: csv_field(record, &["name"]),
        description: csv_field(record, &["description"]),
        purpose: csv_field(record, &["purpose"]),
        explanation: csv_field(record, &["explanation"]),
        examples: csv_field(record, &["examples"]),
        implementation_notes: csv_field(record, &["implementation_notes", "implementationNotes"]),
        test_guidance: csv_field(record, &["test_guidance", "testGuidance"]),
        capabilities: Vec::new(),
        work_items: Vec::new(),
    };
    push_product_area(ctx, &product_id, area)?;
    Ok(())
}

fn push_csv_capability(
    ctx: &mut ImportBuildContext,
    record: &CsvRecord,
    request_product_id: Option<&str>,
) -> Result<(), AppError> {
    let product_area_id = csv_field(record, &["product_area_id", "parent_id"])
        .ok_or_else(|| csv_error(record, "missing product_area_id for capability"))?;
    let product_id = ctx
        .product_areas
        .get(&product_area_id)
        .map(|scope| scope.product_id.clone())
        .or_else(|| csv_field(record, &["product_id", "productId"]))
        .or_else(|| request_product_id.map(ToString::to_string))
        .ok_or_else(|| csv_error(record, "missing product_id for capability"))?;
    let capability = ImportCapability {
        id: Some(csv_field(record, &["id", "capability_id"]).unwrap_or_else(new_id)),
        name: csv_field(record, &["name"]),
        description: csv_field(record, &["description"]),
        acceptance_criteria: csv_field(record, &["acceptance_criteria", "acceptanceCriteria"]),
        explanation: csv_field(record, &["explanation"]),
        examples: csv_field(record, &["examples"]),
        priority: csv_field(record, &["priority"]),
        risk: csv_field(record, &["risk"]),
        technical_notes: csv_field(record, &["technical_notes", "technicalNotes"]),
        implementation_notes: csv_field(record, &["implementation_notes", "implementationNotes"]),
        test_guidance: csv_field(record, &["test_guidance", "testGuidance"]),
        node_kind: Some("capability".to_string()),
        features: Vec::new(),
        capabilities: Vec::new(),
        work_items: Vec::new(),
    };
    push_capability(
        ctx,
        &product_id,
        &product_area_id,
        None,
        0,
        "capability",
        capability,
    )?;
    Ok(())
}

fn push_csv_feature(
    ctx: &mut ImportBuildContext,
    record: &CsvRecord,
    request_product_id: Option<&str>,
) -> Result<(), AppError> {
    let parent_id = csv_field(record, &["capability_id", "parent_id"])
        .ok_or_else(|| csv_error(record, "missing capability_id for feature"))?;
    let parent_scope = ctx.capabilities.get(&parent_id).cloned();
    let product_area_id = parent_scope
        .as_ref()
        .map(|scope| scope.product_area_id.clone())
        .or_else(|| csv_field(record, &["product_area_id"]))
        .ok_or_else(|| csv_error(record, "missing product_area_id for feature"))?;
    let product_id = parent_scope
        .as_ref()
        .map(|scope| scope.product_id.clone())
        .or_else(|| csv_field(record, &["product_id", "productId"]))
        .or_else(|| request_product_id.map(ToString::to_string))
        .ok_or_else(|| csv_error(record, "missing product_id for feature"))?;
    let feature = ImportCapability {
        id: Some(csv_field(record, &["id", "feature_id"]).unwrap_or_else(new_id)),
        name: csv_field(record, &["name"]),
        description: csv_field(record, &["description"]),
        acceptance_criteria: csv_field(record, &["acceptance_criteria", "acceptanceCriteria"]),
        explanation: csv_field(record, &["explanation"]),
        examples: csv_field(record, &["examples"]),
        priority: csv_field(record, &["priority"]),
        risk: csv_field(record, &["risk"]),
        technical_notes: csv_field(record, &["technical_notes", "technicalNotes"]),
        implementation_notes: csv_field(record, &["implementation_notes", "implementationNotes"]),
        test_guidance: csv_field(record, &["test_guidance", "testGuidance"]),
        node_kind: Some("feature".to_string()),
        features: Vec::new(),
        capabilities: Vec::new(),
        work_items: Vec::new(),
    };
    push_capability(
        ctx,
        &product_id,
        &product_area_id,
        Some(&parent_id),
        1,
        "feature",
        feature,
    )?;
    Ok(())
}

fn push_csv_work_item(
    ctx: &mut ImportBuildContext,
    record: &CsvRecord,
    request_product_id: Option<&str>,
) -> Result<(), AppError> {
    let record_type = normalize_record_type(
        &csv_field(record, &["record_type", "type", "kind"])
            .ok_or_else(|| csv_error(record, "missing record_type"))?,
    );
    let parent_work_item_id = csv_field(record, &["parent_work_item_id", "parent_id"]);
    let inherited_scope = if let Some(parent_id) = parent_work_item_id.as_deref() {
        ctx.work_items
            .get(parent_id)
            .cloned()
            .unwrap_or_else(|| WorkItemScope {
                product_id: csv_field(record, &["product_id", "productId"])
                    .or_else(|| request_product_id.map(ToString::to_string))
                    .unwrap_or_default(),
                product_area_id: csv_field(record, &["product_area_id"]),
                capability_id: csv_field(record, &["feature_id", "capability_id"]),
                source_node_id: csv_field(record, &["feature_id", "capability_id"]),
                source_node_type: csv_field(record, &["feature_id", "capability_id"])
                    .map(|_| "capability".to_string()),
            })
    } else {
        WorkItemScope {
            product_id: csv_field(record, &["product_id", "productId"])
                .or_else(|| request_product_id.map(ToString::to_string))
                .unwrap_or_default(),
            product_area_id: csv_field(record, &["product_area_id"]),
            capability_id: csv_field(record, &["feature_id", "capability_id"]),
            source_node_id: None,
            source_node_type: None,
        }
    };
    if inherited_scope.product_id.trim().is_empty() {
        return Err(csv_error(record, "missing product_id for work item"));
    }

    let mut item = ImportWorkItem {
        id: Some(csv_field(record, &["id", "work_item_id"]).unwrap_or_else(new_id)),
        title: csv_field(record, &["title"]),
        name: csv_field(record, &["name"]),
        product_id: Some(inherited_scope.product_id.clone()),
        product_area_id: inherited_scope.product_area_id.clone(),
        capability_id: inherited_scope.capability_id.clone(),
        feature_id: csv_field(record, &["feature_id"]),
        source_node_id: csv_field(record, &["source_node_id", "sourceNodeId"]),
        source_node_type: csv_field(record, &["source_node_type", "sourceNodeType"]),
        parent_work_item_id: parent_work_item_id.clone(),
        problem_statement: csv_field(record, &["problem_statement", "problemStatement"]),
        description: csv_field(record, &["description"]),
        acceptance_criteria: csv_field(record, &["acceptance_criteria", "acceptanceCriteria"]),
        constraints: csv_field(record, &["constraints"]),
        work_item_type: csv_field(record, &["work_item_type", "workItemType"]),
        priority: csv_field(record, &["priority"]),
        complexity: csv_field(record, &["complexity"]),
        status: csv_field(record, &["status"]),
        tasks: Vec::new(),
    };
    if item.work_item_type.is_none() {
        item.work_item_type = Some(
            if record_type == "task" {
                "task"
            } else {
                "story"
            }
            .to_string(),
        );
    }
    push_work_items(
        ctx,
        inherited_scope,
        vec![item],
        parent_work_item_id.as_deref(),
        if record_type == "task" {
            "task"
        } else {
            "story"
        },
    )
}

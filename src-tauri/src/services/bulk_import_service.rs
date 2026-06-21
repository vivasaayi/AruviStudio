use crate::domain::bulk_import::{
    BulkImportCapabilityRow, BulkImportJob, BulkImportJobStatus, BulkImportProductAreaRow,
    BulkImportProductRow, BulkImportRows, BulkImportWorkItemRow,
};
use crate::error::AppError;
use crate::persistence::bulk_import_repo;
use crate::state::AppState;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{error, info};

const DEFAULT_BATCH_SIZE: usize = 1_000;

#[derive(Debug, Clone)]
pub struct BulkImportRequest {
    pub file_path: String,
    pub format: Option<String>,
    pub product_id: Option<String>,
}

struct PreparedImport {
    rows: BulkImportRows,
    required_existing_product_ids: HashSet<String>,
}

#[derive(Debug, Clone)]
struct ProductAreaScope {
    product_id: String,
    product_area_id: String,
}

#[derive(Debug, Clone)]
struct CapabilityScope {
    product_id: String,
    product_area_id: String,
    capability_id: String,
}

#[derive(Debug, Clone)]
struct WorkItemScope {
    product_id: String,
    module_id: Option<String>,
    capability_id: Option<String>,
    source_node_id: Option<String>,
    source_node_type: Option<String>,
}

#[derive(Default)]
struct ImportBuildContext {
    rows: BulkImportRows,
    referenced_product_ids: HashSet<String>,
    imported_product_ids: HashSet<String>,
    product_areas: HashMap<String, ProductAreaScope>,
    capabilities: HashMap<String, CapabilityScope>,
    work_items: HashMap<String, WorkItemScope>,
    product_area_sort: HashMap<String, i64>,
    capability_sort: HashMap<String, i64>,
    work_item_sort: HashMap<String, i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImportDocument {
    #[serde(default)]
    product: Option<ImportProduct>,
    #[serde(default, alias = "product_areas")]
    product_areas: Vec<ImportProductArea>,
    #[serde(default, alias = "work_items")]
    work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImportProduct {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    vision: Option<String>,
    #[serde(default)]
    goals: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    lifecycle: Option<String>,
    #[serde(default)]
    health: Option<String>,
    #[serde(default, alias = "owner_label")]
    owner_label: Option<String>,
    #[serde(default, alias = "investment_status")]
    investment_status: Option<String>,
    #[serde(default)]
    roadmap: Option<String>,
    #[serde(default)]
    evidence: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImportProductArea {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default)]
    examples: Option<String>,
    #[serde(default, alias = "implementation_notes")]
    implementation_notes: Option<String>,
    #[serde(default, alias = "test_guidance")]
    test_guidance: Option<String>,
    #[serde(default)]
    capabilities: Vec<ImportCapability>,
    #[serde(default, alias = "work_items")]
    work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImportCapability {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "acceptance_criteria")]
    acceptance_criteria: Option<String>,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default)]
    examples: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    risk: Option<String>,
    #[serde(default, alias = "technical_notes")]
    technical_notes: Option<String>,
    #[serde(default, alias = "implementation_notes")]
    implementation_notes: Option<String>,
    #[serde(default, alias = "test_guidance")]
    test_guidance: Option<String>,
    #[serde(default, alias = "node_kind")]
    node_kind: Option<String>,
    #[serde(default)]
    features: Vec<ImportCapability>,
    #[serde(default, alias = "children")]
    capabilities: Vec<ImportCapability>,
    #[serde(default, alias = "work_items")]
    work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImportWorkItem {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, alias = "product_id")]
    product_id: Option<String>,
    #[serde(default, alias = "product_area_id", alias = "module_id")]
    product_area_id: Option<String>,
    #[serde(default, alias = "capability_id")]
    capability_id: Option<String>,
    #[serde(default, alias = "feature_id")]
    feature_id: Option<String>,
    #[serde(default, alias = "source_node_id")]
    source_node_id: Option<String>,
    #[serde(default, alias = "source_node_type")]
    source_node_type: Option<String>,
    #[serde(default, alias = "parent_work_item_id")]
    parent_work_item_id: Option<String>,
    #[serde(default, alias = "problem_statement")]
    problem_statement: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "acceptance_criteria")]
    acceptance_criteria: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default, alias = "work_item_type")]
    work_item_type: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    complexity: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default, alias = "subtasks", alias = "children")]
    tasks: Vec<ImportWorkItem>,
}

struct CsvRecord {
    row_index: i64,
    fields: HashMap<String, String>,
}

pub fn bulk_import_schema() -> Value {
    json!({
        "formats": ["json", "csv"],
        "json": {
            "description": "Canonical nested import format. Product Areas contain capabilities, capabilities contain features, and workItems contain stories/tasks.",
            "requiredTopLevel": ["product or submit productId", "productAreas"],
            "shape": {
                "product": {
                    "id": "optional stable product id",
                    "name": "required when creating a product",
                    "description": "optional",
                    "vision": "optional",
                    "goals": ["optional goal"],
                    "tags": ["optional tag"],
                    "lifecycle": "idea|incubating|active|maturing|sunsetting|retired",
                    "health": "unknown|healthy|watch|at_risk|blocked",
                    "ownerLabel": "optional",
                    "investmentStatus": "evaluate|invest|maintain|pause|retire",
                    "roadmap": "optional",
                    "evidence": "optional"
                },
                "productAreas": [{
                    "id": "optional stable area id",
                    "name": "required",
                    "description": "optional",
                    "purpose": "optional",
                    "capabilities": [{
                        "id": "optional stable capability id",
                        "name": "required",
                        "description": "optional",
                        "acceptanceCriteria": "optional",
                        "priority": "critical|high|medium|low",
                        "risk": "high|medium|low",
                        "features": [{
                            "id": "optional stable feature id",
                            "name": "required",
                            "description": "optional",
                            "workItems": [{
                                "id": "optional stable work item id",
                                "title": "required",
                                "workItemType": "story|task|setup|bug|refactor|test|review|security_fix|performance_improvement",
                                "priority": "critical|high|medium|low",
                                "complexity": "trivial|low|medium|high|very_high",
                                "tasks": [{
                                    "title": "required child task title",
                                    "workItemType": "task"
                                }]
                            }]
                        }]
                    }]
                }]
            },
            "example": {
                "product": {
                    "id": "payments-platform",
                    "name": "Payments Platform",
                    "goals": ["Reduce payment failure rate"]
                },
                "productAreas": [{
                    "id": "payments-checkout",
                    "name": "Checkout",
                    "capabilities": [{
                        "id": "cap-card-payments",
                        "name": "Card Payments",
                        "features": [{
                            "id": "feat-3ds",
                            "name": "3DS Challenge Flow",
                            "workItems": [{
                                "id": "story-3ds-browser",
                                "title": "Implement browser challenge handoff",
                                "workItemType": "story",
                                "tasks": [{
                                    "id": "task-3ds-tests",
                                    "title": "Add challenge handoff tests",
                                    "workItemType": "task"
                                }]
                            }]
                        }]
                    }]
                }]
            }
        },
        "csv": {
            "description": "Flat CSV format. Parent records must appear before child records. Use stable ids for parent references.",
            "columns": [
                "record_type",
                "id",
                "parent_id",
                "product_id",
                "product_area_id",
                "capability_id",
                "feature_id",
                "parent_work_item_id",
                "name",
                "title",
                "description",
                "problem_statement",
                "acceptance_criteria",
                "constraints",
                "priority",
                "risk",
                "complexity",
                "work_item_type",
                "status"
            ],
            "recordTypes": ["product", "product_area", "capability", "feature", "work_item", "story", "task"],
            "exampleRows": [
                "record_type,id,parent_id,product_id,product_area_id,capability_id,feature_id,parent_work_item_id,name,title,description,problem_statement,acceptance_criteria,constraints,priority,risk,complexity,work_item_type,status",
                "product,payments-platform,,,,,,,Payments Platform,,,,,,,,,",
                "product_area,payments-checkout,,payments-platform,,,,,Checkout,,,,,,,,,",
                "capability,cap-card-payments,payments-checkout,payments-platform,payments-checkout,,,,Card Payments,,,,,high,medium,,,",
                "feature,feat-3ds,cap-card-payments,payments-platform,payments-checkout,cap-card-payments,,,3DS Challenge Flow,,,,,high,medium,,,",
                "story,story-3ds-browser,,payments-platform,payments-checkout,,feat-3ds,,,Implement browser challenge handoff,,,,high,,medium,story,draft",
                "task,task-3ds-tests,story-3ds-browser,payments-platform,,,,story-3ds-browser,,Add challenge handoff tests,,,,medium,,low,task,draft"
            ]
        },
        "submitTool": {
            "name": "catalog.bulk_import.submit",
            "arguments": {
                "filePath": "absolute or process-relative JSON/CSV file path",
                "format": "optional json|csv, inferred from extension when omitted",
                "productId": "optional existing product id when the file does not define product"
            }
        },
        "statusTool": {
            "name": "catalog.bulk_import.get_status",
            "arguments": {
                "jobId": "bulk import job id returned by submit"
            }
        }
    })
}

pub async fn submit_bulk_import(
    state: AppState,
    request: BulkImportRequest,
) -> Result<BulkImportJob, AppError> {
    let import_format = resolve_format(&request.file_path, request.format.as_deref())?;
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = bulk_import_repo::create_job(
        &state.db,
        &job_id,
        request.file_path.as_str(),
        import_format.as_str(),
    )
    .await?;
    let worker_state = state.clone();
    let worker_request = BulkImportRequest {
        file_path: request.file_path,
        format: Some(import_format),
        product_id: request.product_id,
    };
    let worker_job_id = job.id.clone();

    tokio::spawn(async move {
        if let Err(error) =
            run_bulk_import_job(worker_state.clone(), worker_job_id.clone(), worker_request).await
        {
            let message = error.to_string();
            error!(job_id = %worker_job_id, error = %message, "bulk import job failed");
            let _ = bulk_import_repo::add_job_error(
                &worker_state.db,
                &worker_job_id,
                None,
                "job",
                "",
                &message,
            )
            .await;
            let _ =
                bulk_import_repo::mark_job_failed(&worker_state.db, &worker_job_id, &message).await;
        }
    });

    Ok(job)
}

pub async fn get_bulk_import_status(
    pool: &SqlitePool,
    job_id: &str,
) -> Result<BulkImportJobStatus, AppError> {
    Ok(BulkImportJobStatus {
        job: bulk_import_repo::get_job(pool, job_id).await?,
        errors: bulk_import_repo::list_job_errors(pool, job_id, 25).await?,
    })
}

pub async fn list_bulk_import_jobs(
    pool: &SqlitePool,
    limit: Option<i64>,
) -> Result<Vec<BulkImportJob>, AppError> {
    bulk_import_repo::list_jobs(pool, limit.unwrap_or(20)).await
}

async fn run_bulk_import_job(
    state: AppState,
    job_id: String,
    request: BulkImportRequest,
) -> Result<(), AppError> {
    let import_format = request
        .format
        .as_deref()
        .ok_or_else(|| AppError::Validation("missing import format".to_string()))?;
    info!(job_id = %job_id, file_path = %request.file_path, import_format = %import_format, "bulk import job started");
    let content = tokio::fs::read_to_string(&request.file_path).await?;
    let prepared = match import_format {
        "json" => prepare_json_import(&content, request.product_id.as_deref())?,
        "csv" => prepare_csv_import(&content, request.product_id.as_deref())?,
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported bulk import format '{other}'. Use json or csv."
            )))
        }
    };
    if prepared.rows.total_records() == 0 {
        return Err(AppError::Validation(
            "Bulk import file did not contain any records.".to_string(),
        ));
    }

    bulk_import_repo::mark_job_running(&state.db, &job_id, prepared.rows.total_records()).await?;
    for product_id in &prepared.required_existing_product_ids {
        bulk_import_repo::ensure_product_exists(&state.db, product_id).await?;
    }
    bulk_import_repo::upsert_all(&state.db, &job_id, &prepared.rows, DEFAULT_BATCH_SIZE).await?;
    let completed = bulk_import_repo::mark_job_completed(&state.db, &job_id).await?;
    info!(
        job_id = %job_id,
        total_records = completed.total_records,
        processed_records = completed.processed_records,
        "bulk import job completed"
    );
    Ok(())
}

fn prepare_json_import(
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
        module_id: None,
        capability_id: None,
        source_node_id: None,
        source_node_type: None,
    };
    push_work_items(&mut ctx, product_scope, document.work_items, None, "story")?;

    Ok(finish_prepared_import(ctx))
}

fn prepare_csv_import(
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

fn finish_prepared_import(ctx: ImportBuildContext) -> PreparedImport {
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

fn push_product(
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

fn push_product_area(
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
        module_id: Some(id.clone()),
        capability_id: None,
        source_node_id: Some(id.clone()),
        source_node_type: Some("product_area".to_string()),
    };
    push_work_items(ctx, area_scope, area.work_items, None, "story")?;
    Ok(id)
}

fn push_capability(
    ctx: &mut ImportBuildContext,
    product_id: &str,
    module_id: &str,
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
    let sort_key = capability_sort_key(module_id, parent_capability_id);
    let sort_order = next_sort(&mut ctx.capability_sort, &sort_key);
    ctx.rows.capabilities.push(BulkImportCapabilityRow {
        id: id.clone(),
        module_id: module_id.to_string(),
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
            product_area_id: module_id.to_string(),
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
                module_id,
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
                module_id,
                Some(&id),
                level + 1,
                "feature",
                feature,
            )?;
        }
    }

    let work_scope = WorkItemScope {
        product_id: product_id.to_string(),
        module_id: Some(module_id.to_string()),
        capability_id: Some(id.clone()),
        source_node_id: Some(id.clone()),
        source_node_type: Some("capability".to_string()),
    };
    push_work_items(ctx, work_scope, capability.work_items, None, "story")?;
    Ok(id)
}

fn push_work_items(
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
        let sort_key = work_item_sort_key(&scope, parent.as_deref());
        let sort_order = next_sort(&mut ctx.work_item_sort, &sort_key);
        ctx.rows.work_items.push(BulkImportWorkItemRow {
            id: id.clone(),
            product_id: scope.product_id.clone(),
            module_id: scope.module_id.clone(),
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
                module_id: Some(scope.product_area_id.clone()),
                capability_id: Some(scope.capability_id.clone()),
                source_node_id: Some(scope.capability_id.clone()),
                source_node_type: Some("capability".to_string()),
            });
        }
        let product_id = clean_ref(item.product_id.as_deref())
            .unwrap_or_else(|| inherited_scope.product_id.clone());
        return Ok(WorkItemScope {
            product_id,
            module_id: clean_ref(item.product_area_id.as_deref()),
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
                module_id: Some(scope.product_area_id.clone()),
                capability_id: None,
                source_node_id: Some(scope.product_area_id.clone()),
                source_node_type: Some("product_area".to_string()),
            });
        }
        return Ok(WorkItemScope {
            product_id: clean_ref(item.product_id.as_deref())
                .unwrap_or_else(|| inherited_scope.product_id.clone()),
            module_id: Some(area_id.clone()),
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
            module_id: inherited_scope.module_id.clone(),
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
    let id = csv_field(record, &["id", "product_area_id", "module_id"]).unwrap_or_else(new_id);
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
    let module_id = csv_field(record, &["product_area_id", "module_id", "parent_id"])
        .ok_or_else(|| csv_error(record, "missing product_area_id for capability"))?;
    let product_id = ctx
        .product_areas
        .get(&module_id)
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
        &module_id,
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
    let module_id = parent_scope
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
        &module_id,
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
                module_id: csv_field(record, &["product_area_id", "module_id"]),
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
            module_id: csv_field(record, &["product_area_id", "module_id"]),
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
        product_area_id: inherited_scope.module_id.clone(),
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

fn parse_csv_records(content: &str) -> Result<Vec<CsvRecord>, AppError> {
    let rows = parse_csv_rows(content)?;
    let Some(headers) = rows.first() else {
        return Ok(Vec::new());
    };
    let headers = headers
        .iter()
        .map(|header| normalize_header(header))
        .collect::<Vec<_>>();
    let mut records = Vec::new();
    for (index, row) in rows.into_iter().enumerate().skip(1) {
        if row.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        let mut fields = HashMap::new();
        for (column_index, value) in row.into_iter().enumerate() {
            let Some(header) = headers.get(column_index) else {
                continue;
            };
            if header.is_empty() {
                continue;
            }
            let value = value.trim().to_string();
            if !value.is_empty() {
                fields.insert(header.clone(), value);
            }
        }
        records.push(CsvRecord {
            row_index: i64::try_from(index + 1).unwrap_or(i64::MAX),
            fields,
        });
    }
    Ok(records)
}

fn parse_csv_rows(content: &str) -> Result<Vec<Vec<String>>, AppError> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = content.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(ch),
        }
    }

    if in_quotes {
        return Err(AppError::Validation(
            "CSV import has an unterminated quoted field.".to_string(),
        ));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn csv_field(record: &CsvRecord, keys: &[&str]) -> Option<String> {
    for key in keys {
        let normalized = normalize_header(key);
        if let Some(value) = record.fields.get(&normalized) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn csv_list_field(record: &CsvRecord, keys: &[&str]) -> Vec<String> {
    csv_field(record, keys)
        .map(|value| {
            value
                .split([';', '|'])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn csv_error(record: &CsvRecord, message: &str) -> AppError {
    AppError::Validation(format!("CSV row {}: {message}", record.row_index))
}

fn normalize_header(value: &str) -> String {
    let mut normalized = String::new();
    for ch in value.trim().trim_start_matches('\u{feff}').chars() {
        if ch == '-' || ch == ' ' {
            if !normalized.ends_with('_') {
                normalized.push('_');
            }
        } else if ch.is_ascii_uppercase() {
            if !normalized.is_empty() && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(ch.to_ascii_lowercase());
        }
    }
    normalized
}

fn normalize_record_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "productarea" => "product_area".to_string(),
        "story" => "story".to_string(),
        "task" => "task".to_string(),
        other => other.to_string(),
    }
}

fn resolve_format(file_path: &str, requested: Option<&str>) -> Result<String, AppError> {
    let value = requested
        .map(ToString::to_string)
        .or_else(|| {
            Path::new(file_path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(ToString::to_string)
        })
        .ok_or_else(|| {
            AppError::Validation(
                "Bulk import format is required when file extension is absent.".to_string(),
            )
        })?;
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => Ok("json".to_string()),
        "csv" => Ok("csv".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported bulk import format '{other}'. Use json or csv."
        ))),
    }
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn clean_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_ref(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn required_clean(value: Option<String>, label: &str) -> Result<String, AppError> {
    clean_option(value).ok_or_else(|| AppError::Validation(format!("missing {label}")))
}

fn next_sort(counters: &mut HashMap<String, i64>, key: &str) -> i64 {
    let entry = counters.entry(key.to_string()).or_insert(0);
    let value = *entry;
    *entry += 1;
    value
}

fn capability_sort_key(module_id: &str, parent_capability_id: Option<&str>) -> String {
    format!(
        "{}\u{1f}{}",
        module_id,
        parent_capability_id.unwrap_or_default()
    )
}

fn work_item_sort_key(scope: &WorkItemScope, parent_work_item_id: Option<&str>) -> String {
    if let Some(parent_id) = parent_work_item_id {
        return format!("parent\u{1f}{parent_id}");
    }
    format!(
        "source\u{1f}{}\u{1f}{}\u{1f}{}",
        scope.product_id,
        scope.source_node_type.as_deref().unwrap_or_default(),
        scope.source_node_id.as_deref().unwrap_or_default()
    )
}

fn normalize_node_kind(value: Option<&str>, default_value: &str) -> Result<String, AppError> {
    normalize_value(value, default_value, NODE_KINDS, "nodeKind")
}

fn normalize_source_node_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "product_area" => Ok("product_area".to_string()),
        "capability" | "feature" => Ok("capability".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported sourceNodeType '{other}'. Use product_area, capability, or feature."
        ))),
    }
}

fn normalize_work_item_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "" | "story" => Ok("story".to_string()),
        "task" => Ok("task".to_string()),
        "setup" => Ok("setup".to_string()),
        "bug" => Ok("bug".to_string()),
        "refactor" => Ok("refactor".to_string()),
        "test" => Ok("test".to_string()),
        "review" => Ok("review".to_string()),
        "security_fix" => Ok("security_fix".to_string()),
        "performance_improvement" => Ok("performance_improvement".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported workItemType '{other}'. Use story, task, setup, bug, refactor, test, review, security_fix, or performance_improvement."
        ))),
    }
}

fn normalize_value(
    value: Option<&str>,
    default_value: &str,
    allowed: &[&str],
    label: &str,
) -> Result<String, AppError> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .to_ascii_lowercase()
        .replace('-', "_");
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(AppError::Validation(format!(
            "Unsupported {label} '{normalized}'. Use one of: {}.",
            allowed.join(", ")
        )))
    }
}

const NODE_KINDS: &[&str] = &["product_area", "capability", "feature"];
const PRODUCT_LIFECYCLES: &[&str] = &[
    "idea",
    "incubating",
    "active",
    "maturing",
    "sunsetting",
    "retired",
];
const PRODUCT_HEALTHS: &[&str] = &["unknown", "healthy", "watch", "at_risk", "blocked"];
const PRODUCT_INVESTMENT_STATUSES: &[&str] = &["evaluate", "invest", "maintain", "pause", "retire"];
const PRIORITIES: &[&str] = &["critical", "high", "medium", "low"];
const RISKS: &[&str] = &["high", "medium", "low"];
const COMPLEXITIES: &[&str] = &["trivial", "low", "medium", "high", "very_high"];
const WORK_ITEM_STATUSES: &[&str] = &[
    "draft",
    "ready_for_review",
    "approved",
    "in_planning",
    "in_progress",
    "in_validation",
    "waiting_human_review",
    "done",
    "blocked",
    "failed",
    "cancelled",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parser_handles_quoted_commas() {
        let records = parse_csv_records(
            "recordType,id,name,description\nproduct,p1,\"Payments, Platform\",\"Line one\"\n",
        )
        .expect("csv parses");
        assert_eq!(records.len(), 1);
        assert_eq!(
            csv_field(&records[0], &["record_type"]).as_deref(),
            Some("product")
        );
        assert_eq!(
            csv_field(&records[0], &["name"]).as_deref(),
            Some("Payments, Platform")
        );
    }

    #[test]
    fn json_import_prepares_nested_work_items() {
        let content = serde_json::to_string(&json!({
            "product": { "id": "p1", "name": "Product" },
            "productAreas": [{
                "id": "a1",
                "name": "Area",
                "capabilities": [{
                    "id": "c1",
                    "name": "Capability",
                    "features": [{
                        "id": "f1",
                        "name": "Feature",
                        "workItems": [{
                            "id": "w1",
                            "title": "Story",
                            "tasks": [{ "id": "t1", "title": "Task" }]
                        }]
                    }]
                }]
            }]
        }))
        .expect("json");
        let prepared = prepare_json_import(&content, None).expect("prepare import");
        assert_eq!(prepared.rows.product_areas.len(), 1);
        assert_eq!(
            prepared
                .rows
                .capabilities
                .iter()
                .filter(|row| row.node_kind == "capability")
                .count(),
            1
        );
        assert_eq!(
            prepared
                .rows
                .capabilities
                .iter()
                .filter(|row| row.node_kind == "feature")
                .count(),
            1
        );
        assert_eq!(prepared.rows.work_items.len(), 2);
        assert_eq!(
            prepared.rows.work_items[1].parent_work_item_id.as_deref(),
            Some("w1")
        );
    }
}

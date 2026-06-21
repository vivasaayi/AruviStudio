use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo};
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_product" => {
            let name = args.required_string(&["name"], "name")?;
            let description = args.string_or_default(&["description"], "")?;
            let vision = args.string_or_default(&["vision"], "")?;
            let goals = args.optional_string_list(&["goals"])?.unwrap_or_default();
            let tags = args.optional_string_list(&["tags"])?.unwrap_or_default();
            let lifecycle = args.optional_string(&["lifecycle"])?;
            let health = args.optional_string(&["health"])?;
            let owner_label = args.optional_string(&["ownerLabel", "owner_label"])?;
            let investment_status =
                args.optional_string(&["investmentStatus", "investment_status"])?;
            let roadmap = args.optional_string(&["roadmap"])?;
            let evidence = args.optional_string(&["evidence"])?;
            let id = uuid::Uuid::new_v4().to_string();
            let goals = serde_json::to_string(&goals)?;
            let tags = serde_json::to_string(&tags)?;
            let product = product_repo::create_product(
                &state.db,
                product_repo::CreateProductInput {
                    id: &id,
                    name: &name,
                    description: &description,
                    vision: &vision,
                    goals: &goals,
                    tags: &tags,
                    lifecycle: lifecycle.as_deref(),
                    health: health.as_deref(),
                    owner_label: owner_label.as_deref(),
                    investment_status: investment_status.as_deref(),
                    roadmap: roadmap.as_deref(),
                    evidence: evidence.as_deref(),
                },
            )
            .await?;
            action_result("create_product", product)
        }
        "get_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "get_product",
                product_repo::get_product(&state.db, &id).await?,
            )
        }
        "list_products" => {
            let hide_examples =
                settings_repo::get_bool_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true).await?;
            let mut products = product_repo::list_products(&state.db).await?;
            if hide_examples {
                products.retain(|product| !product.is_example_product());
            }
            action_result("list_products", products)
        }
        "seed_example_products" => {
            product_service::initialize_example_catalog(&state.db).await?;
            Ok(action_ok("seed_example_products"))
        }
        "update_product" => {
            let id = args.required_string(&["id"], "id")?;
            let goals = args.optional_json_array_string(&["goals"])?;
            let tags = args.optional_json_array_string(&["tags"])?;
            let name = args.optional_string(&["name"])?;
            let description = args.optional_string(&["description"])?;
            let vision = args.optional_string(&["vision"])?;
            let lifecycle = args.optional_string(&["lifecycle"])?;
            let health = args.optional_string(&["health"])?;
            let owner_label = args.optional_string(&["ownerLabel", "owner_label"])?;
            let investment_status =
                args.optional_string(&["investmentStatus", "investment_status"])?;
            let roadmap = args.optional_string(&["roadmap"])?;
            let evidence = args.optional_string(&["evidence"])?;
            let product = product_repo::update_product(
                &state.db,
                product_repo::UpdateProductPatch {
                    id: &id,
                    name: name.as_deref(),
                    description: description.as_deref(),
                    vision: vision.as_deref(),
                    goals: goals.as_deref(),
                    tags: tags.as_deref(),
                    lifecycle: lifecycle.as_deref(),
                    health: health.as_deref(),
                    owner_label: owner_label.as_deref(),
                    investment_status: investment_status.as_deref(),
                    roadmap: roadmap.as_deref(),
                    evidence: evidence.as_deref(),
                },
            )
            .await?;
            action_result("update_product", product)
        }
        "archive_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "archive_product",
                product_repo::archive_product(&state.db, &id).await?,
            )
        }
        action @ "create_product_area" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let name = args.required_string(&["name"], "name")?;
            let id = uuid::Uuid::new_v4().to_string();
            let description = args.string_or_default(&["description"], "")?;
            let purpose = args.string_or_default(&["purpose"], "")?;
            let node_kind = args.optional_string(&["node_kind", "nodeKind"])?;
            let explanation = args.string_or_default(&["explanation"], "")?;
            let examples = args.string_or_default(&["examples"], "")?;
            let implementation_notes =
                args.string_or_default(&["implementation_notes", "implementationNotes"], "")?;
            let test_guidance = args.string_or_default(&["test_guidance", "testGuidance"], "")?;
            let product_area = product_repo::create_product_area(
                &state.db,
                product_repo::CreateProductAreaInput {
                    id: &id,
                    product_id: &product_id,
                    name: &name,
                    description: &description,
                    purpose: &purpose,
                    node_kind: node_kind.as_deref(),
                    explanation: &explanation,
                    examples: &examples,
                    implementation_notes: &implementation_notes,
                    test_guidance: &test_guidance,
                },
            )
            .await?;
            action_result(action, product_area)
        }
        action @ "list_product_areas" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            action_result(
                action,
                product_repo::list_product_areas(&state.db, &product_id).await?,
            )
        }
        action @ "update_product_area" => {
            let id = args.required_string(&["id"], "id")?;
            let name = args.optional_string(&["name"])?;
            let description = args.optional_string(&["description"])?;
            let purpose = args.optional_string(&["purpose"])?;
            let node_kind = args.optional_string(&["node_kind", "nodeKind"])?;
            let explanation = args.optional_string(&["explanation"])?;
            let examples = args.optional_string(&["examples"])?;
            let implementation_notes =
                args.optional_string(&["implementation_notes", "implementationNotes"])?;
            let test_guidance = args.optional_string(&["test_guidance", "testGuidance"])?;
            let product_area = product_repo::update_product_area(
                &state.db,
                product_repo::UpdateProductAreaPatch {
                    id: &id,
                    name: name.as_deref(),
                    description: description.as_deref(),
                    purpose: purpose.as_deref(),
                    node_kind: node_kind.as_deref(),
                    explanation: explanation.as_deref(),
                    examples: examples.as_deref(),
                    implementation_notes: implementation_notes.as_deref(),
                    test_guidance: test_guidance.as_deref(),
                },
            )
            .await?;
            action_result(action, product_area)
        }
        action @ "delete_product_area" => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_product_area(&state.db, &id).await?;
            Ok(action_ok(action))
        }
        action @ "reorder_product_areas" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_product_areas(&state.db, &product_id, &ordered_ids).await?;
            Ok(action_ok(action))
        }
        "create_capability" => {
            let product_area_id =
                args.required_string(&["product_area_id", "productAreaId"], "product_area_id")?;
            let name = args.required_string(&["name"], "name")?;
            let id = uuid::Uuid::new_v4().to_string();
            let parent_capability_id =
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?;
            let description = args.string_or_default(&["description"], "")?;
            let acceptance_criteria =
                args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?;
            let priority = args.string_or_default(&["priority"], "medium")?;
            let risk = args.string_or_default(&["risk"], "medium")?;
            let technical_notes =
                args.string_or_default(&["technical_notes", "technicalNotes"], "")?;
            let node_kind = args.optional_string(&["node_kind", "nodeKind"])?;
            let explanation = args.string_or_default(&["explanation"], "")?;
            let examples = args.string_or_default(&["examples"], "")?;
            let implementation_notes =
                args.string_or_default(&["implementation_notes", "implementationNotes"], "")?;
            let test_guidance = args.string_or_default(&["test_guidance", "testGuidance"], "")?;
            let capability = product_repo::create_capability(
                &state.db,
                product_repo::CreateCapabilityInput {
                    id: &id,
                    product_area_id: &product_area_id,
                    parent_capability_id: parent_capability_id.as_deref(),
                    name: &name,
                    description: &description,
                    acceptance_criteria: &acceptance_criteria,
                    priority: &priority,
                    risk: &risk,
                    technical_notes: &technical_notes,
                    node_kind: node_kind.as_deref(),
                    explanation: &explanation,
                    examples: &examples,
                    implementation_notes: &implementation_notes,
                    test_guidance: &test_guidance,
                },
            )
            .await?;
            action_result("create_capability", capability)
        }
        "list_capabilities" => {
            let product_area_id =
                args.required_string(&["product_area_id", "productAreaId"], "product_area_id")?;
            action_result(
                "list_capabilities",
                product_repo::list_capabilities(&state.db, &product_area_id).await?,
            )
        }
        "update_capability" => {
            let id = args.required_string(&["id"], "id")?;
            let name = args.optional_string(&["name"])?;
            let description = args.optional_string(&["description"])?;
            let acceptance_criteria =
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?;
            let priority = args.optional_string(&["priority"])?;
            let risk = args.optional_string(&["risk"])?;
            let technical_notes = args.optional_string(&["technical_notes", "technicalNotes"])?;
            let node_kind = args.optional_string(&["node_kind", "nodeKind"])?;
            let explanation = args.optional_string(&["explanation"])?;
            let examples = args.optional_string(&["examples"])?;
            let implementation_notes =
                args.optional_string(&["implementation_notes", "implementationNotes"])?;
            let test_guidance = args.optional_string(&["test_guidance", "testGuidance"])?;
            let capability = product_repo::update_capability(
                &state.db,
                product_repo::UpdateCapabilityPatch {
                    id: &id,
                    name: name.as_deref(),
                    description: description.as_deref(),
                    acceptance_criteria: acceptance_criteria.as_deref(),
                    priority: priority.as_deref(),
                    risk: risk.as_deref(),
                    technical_notes: technical_notes.as_deref(),
                    node_kind: node_kind.as_deref(),
                    explanation: explanation.as_deref(),
                    examples: examples.as_deref(),
                    implementation_notes: implementation_notes.as_deref(),
                    test_guidance: test_guidance.as_deref(),
                },
            )
            .await?;
            action_result("update_capability", capability)
        }
        "delete_capability" => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_capability(&state.db, &id).await?;
            Ok(action_ok("delete_capability"))
        }
        "reorder_capabilities" => {
            let product_area_id =
                args.required_string(&["product_area_id", "productAreaId"], "product_area_id")?;
            let parent_capability_id =
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_capabilities(
                &state.db,
                &product_area_id,
                parent_capability_id.as_deref(),
                &ordered_ids,
            )
            .await?;
            Ok(action_ok("reorder_capabilities"))
        }
        "apply_capability_template" => {
            let product_area_id =
                args.required_string(&["product_area_id", "productAreaId"], "product_area_id")?;
            let parent_capability_id =
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?;
            let template_kind =
                args.required_string(&["template_kind", "templateKind"], "template_kind")?;
            let name = args.required_string(&["name"], "name")?;
            let description = args.string_or_default(&["description"], "")?;
            let priority = args.optional_string(&["priority"])?;
            let risk = args.optional_string(&["risk"])?;
            let explanation = args.string_or_default(&["explanation"], "")?;
            let examples = args.string_or_default(&["examples"], "")?;
            let implementation_notes =
                args.string_or_default(&["implementation_notes", "implementationNotes"], "")?;
            let test_guidance = args.string_or_default(&["test_guidance", "testGuidance"], "")?;
            let result = product_service::apply_semantic_template(
                &state.db,
                product_service::ApplySemanticTemplateInput {
                    product_area_id: &product_area_id,
                    parent_capability_id: parent_capability_id.as_deref(),
                    template_kind: &template_kind,
                    name: &name,
                    description: &description,
                    priority: priority.as_deref(),
                    risk: risk.as_deref(),
                    explanation: &explanation,
                    examples: &examples,
                    implementation_notes: &implementation_notes,
                    test_guidance: &test_guidance,
                },
            )
            .await?;
            action_result("apply_capability_template", result)
        }
        "convert_capability_kind" => {
            let result = product_service::convert_capability_kind(
                &state.db,
                &args.required_string(&["id"], "id")?,
                &args.required_string(&["node_kind", "nodeKind"], "node_kind")?,
                args.optional_string(&["child_strategy", "childStrategy"])?
                    .as_deref(),
            )
            .await?;
            action_result("convert_capability_kind", result)
        }
        "get_product_tree" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let tree = product_repo::get_product_tree(&state.db, &product_id).await?;
            let mut tree_value = serde_json::to_value(tree)?;
            if let Some(product_areas) = tree_value.get("product_areas").cloned() {
                tree_value["productAreas"] = product_areas;
            }
            action_result("get_product_tree", tree_value)
        }
        action @ ("list_references" | "list_product_references") => {
            let scope_type = args.optional_string(&["scope_type", "scopeType"])?;
            let scope_id = args.optional_string(&["scope_id", "scopeId"])?;
            action_result(
                action,
                product_repo::list_product_references(
                    &state.db,
                    scope_type.as_deref(),
                    scope_id.as_deref(),
                )
                .await?,
            )
        }
        action @ ("create_reference" | "create_product_reference") => {
            let id = uuid::Uuid::new_v4().to_string();
            let scope_type = args.required_string(&["scope_type", "scopeType"], "scope_type")?;
            let scope_id = args.required_string(&["scope_id", "scopeId"], "scope_id")?;
            let title = args.required_string(&["title"], "title")?;
            let reference_kind =
                args.string_or_default(&["reference_kind", "referenceKind"], "note")?;
            let uri = args.string_or_default(&["uri"], "")?;
            let content = args.string_or_default(&["content"], "")?;
            let reference = product_repo::create_product_reference(
                &state.db,
                product_repo::CreateProductReferenceInput {
                    id: &id,
                    scope_type: &scope_type,
                    scope_id: &scope_id,
                    title: &title,
                    reference_kind: &reference_kind,
                    uri: &uri,
                    content: &content,
                },
            )
            .await?;
            action_result(action, reference)
        }
        action @ ("delete_reference" | "delete_product_reference") => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_product_reference(&state.db, &id).await?;
            Ok(action_ok(action))
        }
        "get_bulk_import_schema" => action_result(
            "get_bulk_import_schema",
            bulk_import_service::bulk_import_schema(),
        ),
        "submit_bulk_import" => {
            let file_path = args.required_string(&["file_path", "filePath"], "file_path")?;
            let job = bulk_import_service::submit_bulk_import(
                (*state).clone(),
                BulkImportRequest {
                    file_path,
                    format: args.optional_string(&["format"])?,
                    product_id: args.optional_string(&["product_id", "productId"])?,
                },
            )
            .await?;
            action_result("submit_bulk_import", job)
        }
        "get_bulk_import_status" => {
            let job_id = args.required_string(&["job_id", "jobId"], "job_id")?;
            action_result(
                "get_bulk_import_status",
                bulk_import_service::get_bulk_import_status(&state.db, &job_id).await?,
            )
        }
        "list_bulk_import_jobs" => action_result(
            "list_bulk_import_jobs",
            bulk_import_service::list_bulk_import_jobs(&state.db, args.optional_i64(&["limit"])?)
                .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_catalog action: {other}"
        ))),
    }
}

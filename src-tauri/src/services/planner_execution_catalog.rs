use crate::error::AppError;
use crate::persistence::product_repo;
use crate::services::planner_action_fields::{
    fields_string, fields_string_array, format_joined, string_array_field, string_field,
    target_field,
};
use crate::services::planner_catalog::{find_capability, find_product, find_product_area};
use crate::services::product_service;
use crate::state::AppState;
use serde_json::Value;

pub(crate) async fn execute_catalog_action(
    state: &AppState,
    action_type: &str,
    action: &Value,
) -> Result<Vec<String>, AppError> {
    match action_type {
        "create_product" => {
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing product name".to_string()))?;
            let description = string_field(action, "description").unwrap_or_default();
            let vision = string_field(action, "vision").unwrap_or_default();
            let goals = format_joined(string_array_field(action, "goals"));
            let tags = format_joined(string_array_field(action, "tags"));
            let lifecycle = string_field(action, "lifecycle");
            let health = string_field(action, "health");
            let owner_label =
                string_field(action, "ownerLabel").or_else(|| string_field(action, "owner_label"));
            let investment_status = string_field(action, "investmentStatus")
                .or_else(|| string_field(action, "investment_status"));
            let roadmap = string_field(action, "roadmap");
            let evidence = string_field(action, "evidence");
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
            Ok(vec![format!("Created product \"{}\".", product.name)])
        }
        "update_product" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let name = fields_string(action, "name");
            let description = fields_string(action, "description");
            let vision = fields_string(action, "vision");
            let goals = fields_string_array(action, "goals").map(|value| value.join(", "));
            let tags = fields_string_array(action, "tags").map(|value| value.join(", "));
            let lifecycle = fields_string(action, "lifecycle");
            let health = fields_string(action, "health");
            let owner_label = fields_string(action, "ownerLabel")
                .or_else(|| fields_string(action, "owner_label"));
            let investment_status = fields_string(action, "investmentStatus")
                .or_else(|| fields_string(action, "investment_status"));
            let roadmap = fields_string(action, "roadmap");
            let evidence = fields_string(action, "evidence");
            let updated = product_repo::update_product(
                &state.db,
                product_repo::UpdateProductPatch {
                    id: &product.id,
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
            Ok(vec![format!("Updated product \"{}\".", updated.name)])
        }
        "archive_product" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            product_repo::archive_product(&state.db, &product.id).await?;
            Ok(vec![format!("Archived product \"{}\".", product.name)])
        }
        "create_product_area" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing product area name".to_string()))?;
            let node_kind =
                string_field(action, "nodeKind").or_else(|| string_field(action, "node_kind"));
            let description = string_field(action, "description").unwrap_or_default();
            let purpose = string_field(action, "purpose").unwrap_or_default();
            let explanation = string_field(action, "explanation").unwrap_or_default();
            let examples = string_field(action, "examples").unwrap_or_default();
            let implementation_notes = string_field(action, "implementationNotes")
                .or_else(|| string_field(action, "implementation_notes"))
                .unwrap_or_default();
            let test_guidance = string_field(action, "testGuidance")
                .or_else(|| string_field(action, "test_guidance"))
                .unwrap_or_default();
            let product_area = product_repo::create_product_area(
                &state.db,
                product_repo::CreateProductAreaInput {
                    id: &id,
                    product_id: &product.id,
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
            Ok(vec![format!(
                "Created product area \"{}\" in \"{}\".",
                product_area.name, product.name
            )])
        }
        "update_product_area" => {
            let product_area = find_product_area(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
            )
            .await?;
            let name = fields_string(action, "name");
            let description = fields_string(action, "description");
            let purpose = fields_string(action, "purpose");
            let node_kind =
                fields_string(action, "nodeKind").or_else(|| fields_string(action, "node_kind"));
            let explanation = fields_string(action, "explanation");
            let examples = fields_string(action, "examples");
            let implementation_notes = fields_string(action, "implementationNotes")
                .or_else(|| fields_string(action, "implementation_notes"));
            let test_guidance = fields_string(action, "testGuidance")
                .or_else(|| fields_string(action, "test_guidance"));
            let updated = product_repo::update_product_area(
                &state.db,
                product_repo::UpdateProductAreaPatch {
                    id: &product_area.id,
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
            Ok(vec![format!("Updated product area \"{}\".", updated.name)])
        }
        "delete_product_area" => {
            let product_area = find_product_area(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
            )
            .await?;
            product_repo::delete_product_area(&state.db, &product_area.id).await?;
            Ok(vec![format!(
                "Deleted product area \"{}\".",
                product_area.name
            )])
        }
        "create_capability" => {
            let product_area = find_product_area(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
            )
            .await?;
            let parent_capability_id = if target_field(action, "capabilityName").is_some() {
                Some(
                    find_capability(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "productAreaName"),
                        target_field(action, "capabilityName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing capability name".to_string()))?;
            let node_kind =
                string_field(action, "nodeKind").or_else(|| string_field(action, "node_kind"));
            let description = string_field(action, "description").unwrap_or_default();
            let acceptance_criteria =
                string_field(action, "acceptanceCriteria").unwrap_or_default();
            let priority = string_field(action, "priority").unwrap_or_else(|| "medium".to_string());
            let risk = string_field(action, "risk").unwrap_or_else(|| "medium".to_string());
            let technical_notes = string_field(action, "technicalNotes").unwrap_or_default();
            let explanation = string_field(action, "explanation").unwrap_or_default();
            let examples = string_field(action, "examples").unwrap_or_default();
            let implementation_notes = string_field(action, "implementationNotes")
                .or_else(|| string_field(action, "implementation_notes"))
                .unwrap_or_default();
            let test_guidance = string_field(action, "testGuidance")
                .or_else(|| string_field(action, "test_guidance"))
                .unwrap_or_default();
            let capability = product_repo::create_capability(
                &state.db,
                product_repo::CreateCapabilityInput {
                    id: &id,
                    product_area_id: &product_area.id,
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
            Ok(vec![format!(
                "Created capability \"{}\" in \"{}\".",
                capability.name, product_area.name
            )])
        }
        "update_capability" => {
            let capability = find_capability(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
                target_field(action, "capabilityName"),
            )
            .await?;
            let name = fields_string(action, "name");
            let description = fields_string(action, "description");
            let acceptance_criteria = fields_string(action, "acceptanceCriteria");
            let priority = fields_string(action, "priority");
            let risk = fields_string(action, "risk");
            let technical_notes = fields_string(action, "technicalNotes");
            let node_kind =
                fields_string(action, "nodeKind").or_else(|| fields_string(action, "node_kind"));
            let explanation = fields_string(action, "explanation");
            let examples = fields_string(action, "examples");
            let implementation_notes = fields_string(action, "implementationNotes")
                .or_else(|| fields_string(action, "implementation_notes"));
            let test_guidance = fields_string(action, "testGuidance")
                .or_else(|| fields_string(action, "test_guidance"));
            let updated = product_repo::update_capability(
                &state.db,
                product_repo::UpdateCapabilityPatch {
                    id: &capability.id,
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
            Ok(vec![format!("Updated capability \"{}\".", updated.name)])
        }
        "delete_capability" => {
            let capability = find_capability(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
                target_field(action, "capabilityName"),
            )
            .await?;
            product_repo::delete_capability(&state.db, &capability.id).await?;
            Ok(vec![format!("Deleted capability \"{}\".", capability.name)])
        }
        "apply_capability_template" => {
            let product_area = find_product_area(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
            )
            .await?;
            let parent_capability_id = if target_field(action, "capabilityName").is_some() {
                Some(
                    find_capability(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "productAreaName"),
                        target_field(action, "capabilityName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let template_kind = string_field(action, "templateKind")
                .or_else(|| string_field(action, "template_kind"))
                .ok_or_else(|| AppError::Validation("Missing template kind".to_string()))?;
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing template topic name".to_string()))?;
            let description = string_field(action, "description").unwrap_or_default();
            let priority = string_field(action, "priority");
            let risk = string_field(action, "risk");
            let explanation = string_field(action, "explanation").unwrap_or_default();
            let examples = string_field(action, "examples").unwrap_or_default();
            let implementation_notes = string_field(action, "implementationNotes")
                .or_else(|| string_field(action, "implementation_notes"))
                .unwrap_or_default();
            let test_guidance = string_field(action, "testGuidance")
                .or_else(|| string_field(action, "test_guidance"))
                .unwrap_or_default();
            let result = product_service::apply_semantic_template(
                &state.db,
                product_service::ApplySemanticTemplateInput {
                    product_area_id: &product_area.id,
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
            Ok(vec![format!(
                "Applied template {} to create chapter root \"{}\".",
                result.template_kind, result.topic_node.name
            )])
        }
        "convert_capability_kind" => {
            let capability = find_capability(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "productAreaName"),
                target_field(action, "capabilityName"),
            )
            .await?;
            let result = product_service::convert_capability_kind(
                &state.db,
                &capability.id,
                &string_field(action, "nodeKind")
                    .or_else(|| string_field(action, "node_kind"))
                    .ok_or_else(|| AppError::Validation("Missing node kind".to_string()))?,
                string_field(action, "childStrategy")
                    .or_else(|| string_field(action, "child_strategy"))
                    .as_deref(),
            )
            .await?;
            Ok(vec![format!(
                "Converted capability \"{}\" from {} to {}.",
                result.capability.name, result.previous_node_kind, result.capability.node_kind
            )])
        }
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

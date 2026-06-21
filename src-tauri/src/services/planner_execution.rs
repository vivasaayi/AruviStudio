use crate::domain::workflow::UserAction;
use crate::error::AppError;
use crate::persistence::{
    approval_repo, product_repo, settings_repo, work_item_repo, workflow_repo,
};
use crate::services::planner_action_fields::{
    fields_string, fields_string_array, format_joined, string_array_field, string_field,
    target_field,
};
use crate::services::planner_catalog::{
    build_tree_nodes, find_capability, find_product, find_product_area, find_work_item,
};
use crate::services::planner_service::{PlannerPlan, PlannerTreeNode};
use crate::services::product_service;
use crate::state::AppState;
use serde_json::Value;
use std::collections::HashMap;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

async fn execute_action(state: &AppState, action: &Value) -> Result<Vec<String>, AppError> {
    let action_type = action
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Planner action missing type".to_string()))?;
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
        "create_work_item" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let product_area_id = if target_field(action, "productAreaName").is_some() {
                Some(
                    find_product_area(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "productAreaName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let capability_id = if target_field(action, "capabilityName").is_some() {
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
            let title = string_field(action, "title")
                .ok_or_else(|| AppError::Validation("Missing work item title".to_string()))?;
            let problem_statement = string_field(action, "problemStatement")
                .or_else(|| string_field(action, "description"))
                .unwrap_or_default();
            let description = string_field(action, "description").unwrap_or_default();
            let acceptance_criteria =
                string_field(action, "acceptanceCriteria").unwrap_or_default();
            let constraints = string_field(action, "constraints").unwrap_or_default();
            let work_item_type =
                string_field(action, "workItemType").unwrap_or_else(|| "story".to_string());
            let priority = string_field(action, "priority").unwrap_or_else(|| "medium".to_string());
            let complexity =
                string_field(action, "complexity").unwrap_or_else(|| "medium".to_string());
            let work_item = work_item_repo::create_work_item(
                &state.db,
                work_item_repo::CreateWorkItemInput {
                    id: &id,
                    product_id: &product.id,
                    product_area_id: product_area_id.as_deref(),
                    capability_id: capability_id.as_deref(),
                    source_node_id: None,
                    source_node_type: None,
                    parent_work_item_id: None,
                    title: &title,
                    problem_statement: &problem_statement,
                    description: &description,
                    acceptance_criteria: &acceptance_criteria,
                    constraints: &constraints,
                    work_item_type: &work_item_type,
                    priority: &priority,
                    complexity: &complexity,
                },
            )
            .await?;
            Ok(vec![format!(
                "Created work item \"{}\" in \"{}\".",
                work_item.title, product.name
            )])
        }
        "update_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let title = fields_string(action, "title");
            let description = fields_string(action, "description");
            let status = fields_string(action, "status");
            let problem_statement = fields_string(action, "problemStatement");
            let acceptance_criteria = fields_string(action, "acceptanceCriteria");
            let constraints = fields_string(action, "constraints");
            let updated = work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    title: title.as_deref(),
                    description: description.as_deref(),
                    status: status.as_deref(),
                    problem_statement: problem_statement.as_deref(),
                    acceptance_criteria: acceptance_criteria.as_deref(),
                    constraints: constraints.as_deref(),
                },
            )
            .await?;
            Ok(vec![format!("Updated work item \"{}\".", updated.title)])
        }
        "delete_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            work_item_repo::delete_work_item(&state.db, &work_item.id).await?;
            Ok(vec![format!("Deleted work item \"{}\".", work_item.title)])
        }
        "approve_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    status: Some("approved"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
            )
            .await?;
            let auto_start = settings_repo::get_bool_setting(
                &state.db,
                AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY,
                true,
            )
            .await?;
            if auto_start {
                let workflow_service = state.workflow_service.lock().await;
                let _ = workflow_service
                    .start_work_item_workflow(&work_item.id)
                    .await?;
            }
            Ok(vec![format!("Approved work item \"{}\".", work_item.title)])
        }
        "reject_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item.id,
                    status: Some("draft"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
            )
            .await?;
            Ok(vec![format!("Rejected work item \"{}\".", work_item.title)])
        }
        "approve_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!("Approved plan for \"{}\".", work_item.title)])
        }
        "reject_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            Ok(vec![format!("Rejected plan for \"{}\".", work_item.title)])
        }
        "approve_work_item_test_review" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "test_review",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!(
                "Approved test review for \"{}\".",
                work_item.title
            )])
        }
        "start_workflow" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .start_work_item_workflow(&work_item.id)
                .await?;
            Ok(vec![format!(
                "Started workflow for \"{}\".",
                work_item.title
            )])
        }
        "workflow_action" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let run =
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(format!(
                            "No workflow run exists for {}",
                            work_item.title
                        ))
                    })?;
            let action_name = string_field(action, "action")
                .ok_or_else(|| AppError::Validation("Missing workflow action".to_string()))?;
            let user_action = match action_name.as_str() {
                "approve" => UserAction::Approve,
                "reject" => UserAction::Reject,
                "pause" => UserAction::Pause,
                "resume" => UserAction::Resume,
                "cancel" => UserAction::Cancel,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action {}",
                        action_name
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&run.id, user_action, string_field(action, "notes"))
                .await?;
            Ok(vec![format!(
                "Applied workflow action \"{}\" to \"{}\".",
                action_name, work_item.title
            )])
        }
        "report_status" => {
            if let Some(work_item_title) = target_field(action, "workItemTitle") {
                let work_item = find_work_item(
                    &state.db,
                    Some(work_item_title),
                    target_field(action, "productName"),
                )
                .await?;
                let run =
                    workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id)
                        .await?;
                let product_name = if let Some(product_id) = work_item.product_id.as_deref() {
                    product_repo::get_product(&state.db, product_id)
                        .await
                        .ok()
                        .map(|p| p.name)
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    "unknown".to_string()
                };
                let mut lines = vec![
                    format!("Status for \"{}\": {}.", work_item.title, work_item.status),
                    format!("Product: {}.", product_name),
                ];
                if let Some(run) = run {
                    lines.push(format!(
                        "Workflow: {} at {}.",
                        run.status, run.current_stage
                    ));
                } else {
                    lines.push("Workflow: not started.".to_string());
                }
                Ok(lines)
            } else {
                let product = find_product(&state.db, target_field(action, "productName")).await?;
                let summaries =
                    work_item_repo::summarize_work_items_by_scope(&state.db, Some(&product.id))
                        .await?;
                let mut counts: HashMap<String, i64> = HashMap::new();
                for summary in summaries {
                    *counts.entry(summary.status).or_insert(0) += summary.total_count;
                }
                let mut lines = vec![format!("Status for \"{}\".", product.name)];
                let mut entries = counts.into_iter().collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (status, count) in entries {
                    lines.push(format!("{}: {}", status, count));
                }
                Ok(lines)
            }
        }
        "report_tree" => {
            let nodes = build_tree_nodes(&state.db, target_field(action, "productName")).await?;
            let mut lines = vec![];
            fn walk(node: &PlannerTreeNode, depth: usize, lines: &mut Vec<String>) {
                lines.push(format!("{}{}", "  ".repeat(depth), node.label));
                for child in &node.children {
                    walk(child, depth + 1, lines);
                }
            }
            for node in nodes {
                walk(&node, 0, &mut lines);
            }
            Ok(lines)
        }
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

pub(crate) async fn execute_plan(
    state: &AppState,
    plan: &PlannerPlan,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut lines = vec![];
    let mut errors = vec![];
    for action in &plan.actions {
        match execute_action(state, action).await {
            Ok(mut action_lines) => lines.append(&mut action_lines),
            Err(error) => errors.push(error.to_string()),
        }
    }
    Ok((lines, errors))
}

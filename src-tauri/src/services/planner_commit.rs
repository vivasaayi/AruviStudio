use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::services::planner_action_fields::{format_joined, string_array_field, string_field};
use crate::services::planner_draft::{build_draft_node_path, find_draft_ancestor_node_by_type};
use crate::services::planner_service::PlannerDraftPlan;
use crate::state::AppState;
use std::collections::HashMap;

pub(crate) async fn commit_draft_plan(
    state: &AppState,
    draft_plan: &PlannerDraftPlan,
) -> Result<Vec<String>, AppError> {
    let mut lines = vec![];
    let mut product_ids: HashMap<String, String> = HashMap::new();
    let mut product_area_ids: HashMap<String, String> = HashMap::new();
    let mut capability_ids: HashMap<String, String> = HashMap::new();

    let mut products = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "product" && node.parent_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    products.sort_by(|left, right| left.name.cmp(&right.name));

    for product_node in products {
        let details = &product_node.details;
        let product_description = product_node.summary.clone().unwrap_or_default();
        let product_vision = string_field(details, "vision").unwrap_or_default();
        let product_goals = format_joined(string_array_field(details, "goals"));
        let product_tags = format_joined(string_array_field(details, "tags"));
        let product_lifecycle = string_field(details, "lifecycle");
        let product_health = string_field(details, "health");
        let product_owner_label =
            string_field(details, "ownerLabel").or_else(|| string_field(details, "owner_label"));
        let product_investment_status = string_field(details, "investmentStatus")
            .or_else(|| string_field(details, "investment_status"));
        let product_roadmap = string_field(details, "roadmap");
        let product_evidence = string_field(details, "evidence");
        let existing_product = match product_repo::get_product(&state.db, &product_node.id).await {
            Ok(product) => Some(product),
            Err(_) => product_repo::list_products(&state.db)
                .await?
                .into_iter()
                .find(|product| {
                    normalize_name(Some(product.name.as_str()))
                        == normalize_name(Some(product_node.name.as_str()))
                }),
        };
        let product = if let Some(existing_product) = existing_product {
            product_repo::update_product(
                &state.db,
                product_repo::UpdateProductPatch {
                    id: &existing_product.id,
                    name: Some(&product_node.name),
                    description: Some(&product_description),
                    vision: Some(&product_vision),
                    goals: Some(&product_goals),
                    tags: Some(&product_tags),
                    lifecycle: product_lifecycle.as_deref(),
                    health: product_health.as_deref(),
                    owner_label: product_owner_label.as_deref(),
                    investment_status: product_investment_status.as_deref(),
                    roadmap: product_roadmap.as_deref(),
                    evidence: product_evidence.as_deref(),
                },
            )
            .await?
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            product_repo::create_product(
                &state.db,
                product_repo::CreateProductInput {
                    id: &id,
                    name: &product_node.name,
                    description: &product_description,
                    vision: &product_vision,
                    goals: &product_goals,
                    tags: &product_tags,
                    lifecycle: product_lifecycle.as_deref(),
                    health: product_health.as_deref(),
                    owner_label: product_owner_label.as_deref(),
                    investment_status: product_investment_status.as_deref(),
                    roadmap: product_roadmap.as_deref(),
                    evidence: product_evidence.as_deref(),
                },
            )
            .await?
        };
        lines.push(format!("Applied design to product \"{}\".", product.name));
        product_ids.insert(product_node.id.clone(), product.id.clone());

        let mut product_areas = draft_plan
            .nodes
            .iter()
            .filter(|node| {
                node.node_type == "product_area"
                    && node.parent_id.as_deref() == Some(&product_node.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        product_areas.sort_by(|left, right| left.name.cmp(&right.name));

        for product_area_node in product_areas {
            let id = uuid::Uuid::new_v4().to_string();
            let description = product_area_node.summary.clone().unwrap_or_default();
            let purpose = string_field(&product_area_node.details, "purpose").unwrap_or_default();
            let node_kind = string_field(&product_area_node.details, "nodeKind");
            let explanation =
                string_field(&product_area_node.details, "explanation").unwrap_or_default();
            let examples = string_field(&product_area_node.details, "examples").unwrap_or_default();
            let implementation_notes =
                string_field(&product_area_node.details, "implementationNotes")
                    .or_else(|| string_field(&product_area_node.details, "implementation_notes"))
                    .unwrap_or_default();
            let test_guidance = string_field(&product_area_node.details, "testGuidance")
                .or_else(|| string_field(&product_area_node.details, "test_guidance"))
                .unwrap_or_default();
            let product_area = product_repo::create_product_area(
                &state.db,
                product_repo::CreateProductAreaInput {
                    id: &id,
                    product_id: &product.id,
                    name: &product_area_node.name,
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
            lines.push(format!(
                "Created product area \"{}\" in \"{}\".",
                product_area.name, product.name
            ));
            product_area_ids.insert(product_area_node.id.clone(), product_area.id.clone());
        }
    }

    let mut pending_capabilities = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "capability")
        .cloned()
        .collect::<Vec<_>>();
    pending_capabilities.sort_by(|left, right| {
        build_draft_node_path(draft_plan, Some(&left.id))
            .len()
            .cmp(&build_draft_node_path(draft_plan, Some(&right.id)).len())
            .then(left.name.cmp(&right.name))
    });

    while !pending_capabilities.is_empty() {
        let mut progressed = false;
        let mut remaining = vec![];

        for capability_node in pending_capabilities {
            let Some(parent_draft_id) = capability_node.parent_id.as_deref() else {
                return Err(AppError::Validation(format!(
                    "Draft capability {} is missing a parent",
                    capability_node.name
                )));
            };

            let (product_area_id, parent_capability_id) = if let Some(product_area_id) =
                product_area_ids.get(parent_draft_id)
            {
                (product_area_id.clone(), None)
            } else if let Some(parent_capability_id) = capability_ids.get(parent_draft_id) {
                let product_area_node =
                    find_draft_ancestor_node_by_type(draft_plan, &capability_node, "product_area")
                        .ok_or_else(|| {
                            AppError::Validation(format!(
                                "Draft capability {} is missing a product_area ancestor",
                                capability_node.name
                            ))
                        })?;
                let product_area_id = product_area_ids.get(&product_area_node.id).cloned().ok_or_else(|| {
                        AppError::Validation(format!(
                            "Draft capability {} could not resolve its persisted product_area parent",
                            capability_node.name
                        ))
                    })?;
                (product_area_id, Some(parent_capability_id.clone()))
            } else {
                remaining.push(capability_node);
                continue;
            };

            let id = uuid::Uuid::new_v4().to_string();
            let description = capability_node.summary.clone().unwrap_or_default();
            let acceptance_criteria = string_field(&capability_node.details, "acceptanceCriteria")
                .or_else(|| string_field(&capability_node.details, "acceptance_criteria"))
                .unwrap_or_default();
            let priority = string_field(&capability_node.details, "priority")
                .unwrap_or_else(|| "medium".into());
            let risk =
                string_field(&capability_node.details, "risk").unwrap_or_else(|| "medium".into());
            let technical_notes = string_field(&capability_node.details, "technicalNotes")
                .or_else(|| string_field(&capability_node.details, "technical_notes"))
                .unwrap_or_default();
            let node_kind = string_field(&capability_node.details, "nodeKind")
                .or_else(|| string_field(&capability_node.details, "node_kind"));
            let explanation =
                string_field(&capability_node.details, "explanation").unwrap_or_default();
            let examples = string_field(&capability_node.details, "examples").unwrap_or_default();
            let implementation_notes =
                string_field(&capability_node.details, "implementationNotes")
                    .or_else(|| string_field(&capability_node.details, "implementation_notes"))
                    .unwrap_or_default();
            let test_guidance = string_field(&capability_node.details, "testGuidance")
                .or_else(|| string_field(&capability_node.details, "test_guidance"))
                .unwrap_or_default();
            let capability = product_repo::create_capability(
                &state.db,
                product_repo::CreateCapabilityInput {
                    id: &id,
                    product_area_id: &product_area_id,
                    parent_capability_id: parent_capability_id.as_deref(),
                    name: &capability_node.name,
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
            lines.push(format!("Created capability \"{}\".", capability.name));
            capability_ids.insert(capability_node.id.clone(), capability.id.clone());
            progressed = true;
        }

        if !progressed {
            let unresolved = remaining
                .iter()
                .map(|node| node.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::Validation(format!(
                "Draft capabilities could not resolve persisted parents: {}",
                unresolved
            )));
        }

        pending_capabilities = remaining;
    }

    let mut work_items = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "work_item")
        .cloned()
        .collect::<Vec<_>>();
    work_items.sort_by(|left, right| left.name.cmp(&right.name));

    for work_item_node in work_items {
        let mut product_id = None;
        let mut product_area_id = None;
        let mut capability_id = None;
        let mut parent = work_item_node.parent_id.as_deref();
        while let Some(parent_id) = parent {
            if let Some(node) = draft_plan
                .nodes
                .iter()
                .find(|candidate| candidate.id == parent_id)
            {
                match node.node_type.as_str() {
                    "capability" => {
                        capability_id = capability_ids.get(&node.id).cloned();
                    }
                    "product_area" => {
                        product_area_id = product_area_ids.get(&node.id).cloned();
                    }
                    "product" => {
                        product_id = product_ids.get(&node.id).cloned();
                    }
                    _ => {}
                }
                parent = node.parent_id.as_deref();
            } else {
                parent = None;
            }
        }
        let product_id = product_id
            .or_else(|| {
                product_area_id.as_ref().and_then(|product_area_id| {
                    draft_plan
                        .nodes
                        .iter()
                        .find(|node| product_area_ids.get(&node.id) == Some(product_area_id))
                        .and_then(|node| node.parent_id.as_ref())
                        .and_then(|parent_id| product_ids.get(parent_id))
                        .cloned()
                })
            })
            .ok_or_else(|| {
                AppError::Validation("Draft work item is missing a product".to_string())
            })?;

        let id = uuid::Uuid::new_v4().to_string();
        let problem_statement = string_field(&work_item_node.details, "problemStatement")
            .or_else(|| work_item_node.summary.clone())
            .unwrap_or_default();
        let description = string_field(&work_item_node.details, "description")
            .or_else(|| work_item_node.summary.clone())
            .unwrap_or_default();
        let acceptance_criteria =
            string_field(&work_item_node.details, "acceptanceCriteria").unwrap_or_default();
        let constraints = string_field(&work_item_node.details, "constraints").unwrap_or_default();
        let work_item_type =
            string_field(&work_item_node.details, "workItemType").unwrap_or_else(|| "story".into());
        let priority =
            string_field(&work_item_node.details, "priority").unwrap_or_else(|| "medium".into());
        let complexity =
            string_field(&work_item_node.details, "complexity").unwrap_or_else(|| "medium".into());
        let work_item = work_item_repo::create_work_item(
            &state.db,
            work_item_repo::CreateWorkItemInput {
                id: &id,
                product_id: &product_id,
                product_area_id: product_area_id.as_deref(),
                capability_id: capability_id.as_deref(),
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: &work_item_node.name,
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
        lines.push(format!("Created work item \"{}\".", work_item.title));
    }

    Ok(lines)
}

fn normalize_name(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

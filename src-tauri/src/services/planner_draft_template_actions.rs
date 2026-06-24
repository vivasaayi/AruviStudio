use crate::domain::product::{ChildReparentStrategy, HierarchyNodeKind, SemanticTemplateKind};
use crate::error::AppError;
use crate::services::planner_action_fields::{
    remove_target_value, set_string_value, set_target_string_value, string_field,
};
use crate::services::planner_draft::{
    draft_node_kind, find_draft_node_by_id, resolve_draft_capability_name,
    resolve_draft_capability_node_id, resolve_draft_product_area_name, resolve_draft_product_name,
};
use crate::services::planner_service::PlannerDraftPlan;
use serde_json::{json, Value};

pub(crate) fn build_capability_template_actions(
    draft_plan: &PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<Vec<Value>, AppError> {
    let product_name =
        resolve_draft_product_name(Some(draft_plan), selected_draft_node_id, action)?
            .ok_or_else(|| AppError::Validation("Draft template needs a product".to_string()))?;
    let product_area_name =
        resolve_draft_product_area_name(Some(draft_plan), selected_draft_node_id, action)?
            .ok_or_else(|| {
                AppError::Validation("Draft template needs a product area".to_string())
            })?;
    let parent_capability_name =
        resolve_draft_capability_name(Some(draft_plan), selected_draft_node_id, action)?;
    let name = string_field(action, "name")
        .ok_or_else(|| AppError::Validation("Draft template name is required".to_string()))?;
    let template_kind_value = string_field(action, "templateKind")
        .or_else(|| string_field(action, "template_kind"))
        .ok_or_else(|| AppError::Validation("Draft template kind is required".to_string()))?;
    let template_kind = SemanticTemplateKind::parse(&template_kind_value).ok_or_else(|| {
        AppError::Validation(
            "Unsupported template kind. Use operator_chapter or technical_topic_book.".to_string(),
        )
    })?;
    let priority = string_field(action, "priority").unwrap_or_else(|| "medium".to_string());
    let risk = string_field(action, "risk").unwrap_or_else(|| "medium".to_string());
    let explanation = string_field(action, "explanation").unwrap_or_default();
    let examples = string_field(action, "examples").unwrap_or_default();
    let implementation_notes = string_field(action, "implementationNotes")
        .or_else(|| string_field(action, "implementation_notes"))
        .unwrap_or_default();
    let test_guidance = string_field(action, "testGuidance")
        .or_else(|| string_field(action, "test_guidance"))
        .unwrap_or_default();
    let description =
        string_field(action, "description").unwrap_or_else(|| format!("{name} book section."));
    let (definition_label, examples_label, implementation_label, tests_label) = match template_kind
    {
        SemanticTemplateKind::OperatorChapter => (
            format!("{name} Definition"),
            format!("{name} Examples"),
            format!("{name} Implementation"),
            format!("{name} Tests"),
        ),
        SemanticTemplateKind::TechnicalTopicBook => (
            format!("{name} Overview"),
            format!("{name} Examples"),
            format!("{name} Implementation"),
            format!("{name} Tests"),
        ),
    };

    Ok(vec![
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": parent_capability_name,
            },
            "name": name,
            "description": description,
            "acceptanceCriteria": format!("{name} has definition, examples, implementation guidance, and test guidance captured."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Template-generated semantic chapter root.",
            "nodeKind": "capability"
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": definition_label,
            "description": format!("Explain what {name} is and when it should be used."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature chapter for explanation and conceptual boundaries.",
            "nodeKind": "feature",
            "explanation": explanation,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": examples_label,
            "description": format!("Capture worked examples and expected behaviors for {name}."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature chapter for examples and concrete edge cases.",
            "nodeKind": "feature",
            "examples": examples,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": implementation_label,
            "description": format!("Describe how {name} should be implemented."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature execution notes for implementation stories.",
            "nodeKind": "feature",
            "implementationNotes": implementation_notes,
        }),
        json!({
            "type": "create_capability",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": name,
            },
            "name": tests_label,
            "description": format!("Describe how {name} should be validated."),
            "priority": priority,
            "risk": risk,
            "technicalNotes": "Feature execution notes for test and verification stories.",
            "nodeKind": "feature",
            "testGuidance": test_guidance,
        }),
        json!({
            "type": "create_work_item",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": implementation_label,
            },
            "title": format!("Implement {name}"),
            "problemStatement": format!("{name} needs implementation aligned to the authored chapter structure."),
            "description": implementation_notes,
            "acceptanceCriteria": format!("{name} is implemented and matches the documented behavior, examples, and edge cases."),
            "constraints": "Preserve the authored semantic structure and keep behavior deterministic.",
            "workItemType": "story",
            "priority": priority,
            "complexity": "medium",
        }),
        json!({
            "type": "create_work_item",
            "target": {
                "productName": product_name,
                "productAreaName": product_area_name,
                "capabilityName": tests_label,
            },
            "title": format!("Write {name} test cases"),
            "problemStatement": format!("{name} needs verification that matches the documented examples and risks."),
            "description": test_guidance,
            "acceptanceCriteria": format!("Coverage validates happy paths, edge cases, and regressions for {name}."),
            "constraints": "Keep tests aligned with the authored examples and implementation notes.",
            "workItemType": "test",
            "priority": priority,
            "complexity": "medium",
        }),
    ])
}

pub(crate) fn convert_draft_capability_kind(
    draft_plan: &mut PlannerDraftPlan,
    selected_draft_node_id: Option<&str>,
    action: &Value,
) -> Result<(), AppError> {
    let capability_id =
        resolve_draft_capability_node_id(draft_plan, selected_draft_node_id, action)?
            .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
    let target_kind_value = string_field(action, "nodeKind")
        .or_else(|| string_field(action, "node_kind"))
        .ok_or_else(|| AppError::Validation("Draft node kind is required".to_string()))?;
    let target_kind = HierarchyNodeKind::parse(&target_kind_value).ok_or_else(|| {
        AppError::Validation(format!("Unsupported node kind {}", target_kind_value))
    })?;
    let strategy = string_field(action, "childStrategy")
        .or_else(|| string_field(action, "child_strategy"))
        .map(|value| {
            ChildReparentStrategy::parse(&value).ok_or_else(|| {
                AppError::Validation(format!("Unsupported child strategy {}", value))
            })
        })
        .transpose()?
        .unwrap_or(ChildReparentStrategy::Reject);
    let target_node = draft_plan
        .nodes
        .iter()
        .find(|node| node.id == capability_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
    let parent_kind = target_node
        .parent_id
        .as_deref()
        .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)))
        .map(|parent| draft_node_kind(draft_plan, parent))
        .unwrap_or(HierarchyNodeKind::Capability);
    if !parent_kind.supports_child_kind(&target_kind) {
        return Err(AppError::Validation(format!(
            "{} cannot contain {}.",
            parent_kind, target_kind
        )));
    }

    let direct_structural_children = draft_plan
        .nodes
        .iter()
        .filter(|child| {
            child.parent_id.as_deref() == Some(capability_id.as_str())
                && child.node_type == "capability"
        })
        .cloned()
        .collect::<Vec<_>>();
    if !direct_structural_children.is_empty() && !target_kind.can_have_children() {
        if strategy != ChildReparentStrategy::ReparentToParent {
            return Err(AppError::Validation(format!(
                "{} cannot contain structural children. Re-run with childStrategy=reparent_to_parent to preserve descendants.",
                target_kind
            )));
        }
        for child in &direct_structural_children {
            let child_kind = draft_node_kind(draft_plan, child);
            if !parent_kind.supports_child_kind(&child_kind) {
                return Err(AppError::Validation(format!(
                    "{} cannot receive existing {} children.",
                    parent_kind, child_kind
                )));
            }
        }
        let new_parent = target_node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)))
            .cloned();
        for child in direct_structural_children {
            if let Some(child_node) = draft_plan.nodes.iter_mut().find(|node| node.id == child.id) {
                child_node.parent_id = target_node.parent_id.clone();
                if let Some(parent) = new_parent.as_ref() {
                    if parent.node_type == "capability" {
                        set_target_string_value(
                            &mut child_node.details,
                            "capabilityName",
                            &parent.name,
                        );
                    } else {
                        remove_target_value(&mut child_node.details, "capabilityName");
                    }
                } else {
                    remove_target_value(&mut child_node.details, "capabilityName");
                }
            }
        }
    }
    if let Some(node) = draft_plan
        .nodes
        .iter_mut()
        .find(|node| node.id == capability_id)
    {
        set_string_value(&mut node.details, "nodeKind", &target_kind.to_string());
    }
    Ok(())
}

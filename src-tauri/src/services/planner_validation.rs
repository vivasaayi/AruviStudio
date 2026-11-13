use crate::error::AppError;
use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_session::{get_or_load_session, lock_planner_session, PlannerService};
use crate::services::planner_types::PlannerTreeNode;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct DraftValidationIssue {
    tone: &'static str,
    title: String,
    detail: String,
}

#[derive(Serialize)]
pub struct DraftValidationSummary {
    score: usize,
    counts: BTreeMap<&'static str, usize>,
    issues: Vec<DraftValidationIssue>,
}

fn validate(nodes: &[PlannerTreeNode]) -> DraftValidationSummary {
    fn visit(
        node: &PlannerTreeNode,
        counts: &mut BTreeMap<&'static str, usize>,
        issues: &mut Vec<DraftValidationIssue>,
    ) {
        let kind = node.node_type.as_deref().unwrap_or("");
        let key = match kind {
            "product" => Some("product"),
            "product_area" => Some("product area"),
            "capability" => Some("capability"),
            "work_item" => Some("work item"),
            _ => None,
        };
        if let Some(key) = key {
            *counts.get_mut(key).unwrap() += 1;
        }
        let mut seen = HashSet::new();
        for child in &node.children {
            if !seen.insert(child.label.trim().to_lowercase()) {
                issues.push(DraftValidationIssue {
                    tone: "warn",
                    title: format!("Duplicate child under {}", node.label),
                    detail: format!(
                        "Multiple children under this branch share the name \"{}\".",
                        child.label
                    ),
                });
            }
        }
        if node.children.is_empty() {
            let issue = match kind {
                "product" => Some((format!("{} needs product areas", node.label), "Products should usually have at least one product area before the design is applied.")),
                "product_area" => Some((format!("{} is empty", node.label), "Product areas should contain capabilities, features, and starter stories so the plan is actionable.")),
                "capability" => Some((format!("{} has no features or stories", node.label), "Capabilities are stronger when they break down into concrete features with implementation-ready stories.")),
                _ => None,
            };
            if let Some((title, detail)) = issue {
                issues.push(DraftValidationIssue {
                    tone: "warn",
                    title,
                    detail: detail.into(),
                });
            }
        }
        for child in &node.children {
            visit(child, counts, issues);
        }
    }
    let mut counts = BTreeMap::from([
        ("product", 0),
        ("product area", 0),
        ("capability", 0),
        ("work item", 0),
    ]);
    let mut issues = Vec::new();
    for node in nodes {
        visit(node, &mut counts, &mut issues);
    }
    if counts["product"] == 0 {
        issues.push(DraftValidationIssue {
            tone: "warn",
            title: "No staged product root".into(),
            detail: "The design needs a product root before it can be applied to the catalog."
                .into(),
        });
    } else {
        issues.insert(0, DraftValidationIssue { tone: "ok", title: "Design tree is structurally valid".into(), detail: "A product root exists and the planner can keep refining the staged hierarchy before apply.".into() });
    }
    let warnings = issues.iter().filter(|issue| issue.tone == "warn").count();
    DraftValidationSummary {
        score: 100usize.saturating_sub(warnings.saturating_mul(12)).max(35),
        counts,
        issues,
    }
}

pub async fn validate_planner_draft(
    service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
) -> Result<DraftValidationSummary, AppError> {
    let _guard = lock_planner_session(&service, &session_id).await;
    let session = get_or_load_session(&service, db, &session_id).await?;
    let nodes = session
        .draft_plan
        .as_ref()
        .map(|draft| build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref()))
        .unwrap_or_default();
    Ok(validate(&nodes))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, label: &str, kind: &str, children: Vec<PlannerTreeNode>) -> PlannerTreeNode {
        PlannerTreeNode {
            id: id.into(),
            label: label.into(),
            node_type: Some(kind.into()),
            meta: None,
            summary: None,
            source: None,
            confidence: None,
            evidence: vec![],
            children,
        }
    }
    #[test]
    fn matches_desktop_validation_for_empty_and_duplicate_branches() {
        let empty = validate(&[]);
        assert_eq!(empty.score, 88);
        assert_eq!(empty.issues[0].title, "No staged product root");
        let tree = vec![node(
            "p",
            "Product",
            "product",
            vec![
                node("a", "Area", "product_area", vec![]),
                node("b", " area ", "product_area", vec![]),
            ],
        )];
        let summary = validate(&tree);
        assert_eq!(summary.counts["product"], 1);
        assert_eq!(summary.counts["product area"], 2);
        assert_eq!(summary.score, 64);
        assert!(summary
            .issues
            .iter()
            .any(|issue| issue.title == "Duplicate child under Product"));
    }
}

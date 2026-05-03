use crate::domain::product::HierarchyNodeKind;
use serde::Serialize;
use std::fmt::Write;

pub const MARKDOWN_RESOURCE_MIME_TYPE: &str = "text/markdown";
pub const JSON_RESOURCE_MIME_TYPE: &str = "application/json";
pub const RESOURCE_LAST_MODIFIED: &str = "2026-04-22T06:15:00Z";
pub const NODE_KIND_CONSTRAINTS_URI: &str = "aruvi://catalog/node-kind-constraints";

const HIGH_PRIORITY_MODEL_CONTEXT_THRESHOLD: f64 = 0.95;

const PRODUCT_PHILOSOPHY: &str = include_str!("../../docs/mcp-resources/product-philosophy.md");
const SEMANTIC_TREE_RULES: &str = include_str!("../../docs/mcp-resources/semantic-tree-rules.md");
const TECHNICAL_DEPTH_RUBRIC: &str =
    include_str!("../../docs/mcp-resources/technical-depth-rubric.md");
const BOOK_GRADE_STRUCTURE_DECISION: &str =
    include_str!("../../docs/mcp-resources/book-grade-structure-decision.md");
const EXAMPLE_DEEP_CALCULATOR_PRODUCT: &str =
    include_str!("../../docs/mcp-resources/example-deep-calculator-product.md");
const EXAMPLE_KUBERNETES_DASHBOARD_PRODUCT: &str =
    include_str!("../../docs/mcp-resources/example-kubernetes-dashboard-product.md");

#[derive(Debug, Clone, Copy)]
pub struct PlanningResource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
    pub priority: f64,
    pub content: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeKindConstraintsResource {
    default_root_kind: String,
    root_kinds: Vec<String>,
    node_kinds: Vec<NodeKindConstraint>,
    modeling_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeKindConstraint {
    node_kind: String,
    is_root_kind: bool,
    can_have_children: bool,
    default_child_kind: Option<String>,
    allowed_child_kinds: Vec<String>,
}

pub fn static_resources() -> &'static [PlanningResource] {
    &[
        PlanningResource {
            uri: "aruvi://guides/product-philosophy",
            name: "product-philosophy",
            title: "Aruvi Product Philosophy",
            description: "Core doctrine for modeling products as readable technical systems instead of shallow planning trees.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: PRODUCT_PHILOSOPHY,
        },
        PlanningResource {
            uri: "aruvi://guides/semantic-tree-rules",
            name: "semantic-tree-rules",
            title: "Aruvi Semantic Tree Rules",
            description: "Canonical node kinds, allowed child rules, and authoring constraints for the product hierarchy.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: SEMANTIC_TREE_RULES,
        },
        PlanningResource {
            uri: "aruvi://guides/technical-depth-rubric",
            name: "technical-depth-rubric",
            title: "Aruvi Technical Depth Rubric",
            description: "Defines what technical depth means in Aruvi and how to recognize shallow product structure.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: TECHNICAL_DEPTH_RUBRIC,
        },
        PlanningResource {
            uri: "aruvi://guides/book-grade-structure-decision",
            name: "book-grade-structure-decision",
            title: "Decision: Keep Rollouts as Execution Leaves",
            description: "Explains the accepted modeling decision for book-grade detail: rollouts stay leaves and richer chapter structure belongs above them.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 0.95,
            content: BOOK_GRADE_STRUCTURE_DECISION,
        },
        PlanningResource {
            uri: "aruvi://examples/deep-calculator-product",
            name: "deep-calculator-product",
            title: "Gold-Standard Example: Scientific Calculator",
            description: "A technically deep calculator product model showing meaningful areas, capabilities, state concerns, and rollouts.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 0.9,
            content: EXAMPLE_DEEP_CALCULATOR_PRODUCT,
        },
        PlanningResource {
            uri: "aruvi://examples/kubernetes-dashboard-product",
            name: "kubernetes-dashboard-product",
            title: "Gold-Standard Example: Kubernetes Dashboard",
            description: "A technically deep operational product model showing systems, subsystems, observability, and safe action design.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 0.85,
            content: EXAMPLE_KUBERNETES_DASHBOARD_PRODUCT,
        },
    ]
}

pub fn build_node_kind_constraints_resource() -> NodeKindConstraintsResource {
    let node_kinds = [
        HierarchyNodeKind::Area,
        HierarchyNodeKind::Domain,
        HierarchyNodeKind::Subdomain,
        HierarchyNodeKind::System,
        HierarchyNodeKind::Subsystem,
        HierarchyNodeKind::FeatureSet,
        HierarchyNodeKind::Capability,
        HierarchyNodeKind::Rollout,
        HierarchyNodeKind::Reference,
    ]
    .into_iter()
    .map(|kind| NodeKindConstraint {
        node_kind: kind.to_string(),
        is_root_kind: kind.is_root_kind(),
        can_have_children: kind.can_have_children(),
        default_child_kind: kind
            .can_have_children()
            .then(|| HierarchyNodeKind::default_child(&kind).to_string()),
        allowed_child_kinds: kind
            .allowed_child_kinds()
            .into_iter()
            .map(|child_kind| child_kind.to_string())
            .collect(),
    })
    .collect();

    NodeKindConstraintsResource {
        default_root_kind: HierarchyNodeKind::default_root().to_string(),
        root_kinds: [
            HierarchyNodeKind::Area,
            HierarchyNodeKind::Domain,
            HierarchyNodeKind::System,
        ]
        .into_iter()
        .map(|kind| kind.to_string())
        .collect(),
        node_kinds,
        modeling_notes: vec![
            "Rollout and reference are execution or context leaves and cannot own structural children."
                .to_string(),
            "For book-grade detail, create a structural parent and use reference or capability children above rollout execution slices."
                .to_string(),
        ],
    }
}

pub fn node_kind_constraints_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&build_node_kind_constraints_resource())
}

pub fn planner_model_context() -> String {
    let mut context = String::new();
    context.push_str("Shared Aruvi planning doctrine. Treat these MCP resources as authoritative when editing draft planning trees.");

    for resource in static_resources()
        .iter()
        .filter(|resource| resource.priority >= HIGH_PRIORITY_MODEL_CONTEXT_THRESHOLD)
    {
        let _ = write!(
            context,
            "\n\n## {} ({})\nURI: {}\nPriority: {}\n\n{}",
            resource.title,
            resource.name,
            resource.uri,
            resource.priority,
            resource.content.trim()
        );
    }

    let _ = write!(
        context,
        "\n\n## Node Kind Constraints\nURI: {}\n\n{}",
        NODE_KIND_CONSTRAINTS_URI,
        node_kind_constraints_json().unwrap_or_else(|_| {
            "Node kind constraints were unavailable due to a serialization error.".to_string()
        })
    );

    let example_summaries = static_resources()
        .iter()
        .filter(|resource| resource.priority < HIGH_PRIORITY_MODEL_CONTEXT_THRESHOLD)
        .map(|resource| {
            format!(
                "- {} ({}): {}",
                resource.title, resource.uri, resource.description
            )
        })
        .collect::<Vec<_>>();
    if !example_summaries.is_empty() {
        context.push_str("\n\n## Lower-Priority Example Summaries\n");
        context.push_str(&example_summaries.join("\n"));
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_model_context_includes_high_priority_doctrine() {
        let context = planner_model_context();
        assert!(context.contains("# Aruvi Product Philosophy"));
        assert!(context.contains("# Aruvi Semantic Tree Rules"));
        assert!(context.contains("# Aruvi Technical Depth Rubric"));
        assert!(context.contains("Decision: Keep Rollouts as Execution Leaves"));
        assert!(context.contains("\"nodeKind\": \"rollout\""));
        assert!(context.contains("Gold-Standard Example: Scientific Calculator"));
    }
}

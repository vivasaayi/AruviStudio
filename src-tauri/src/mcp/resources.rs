use crate::domain::product::HierarchyNodeKind;
use serde::Serialize;

const MARKDOWN_RESOURCE_MIME_TYPE: &str = "text/markdown";
const JSON_RESOURCE_MIME_TYPE: &str = "application/json";
const RESOURCE_LAST_MODIFIED: &str = "2026-04-22T06:15:00Z";

const PRODUCT_PHILOSOPHY: &str =
    include_str!("../../../docs/mcp-resources/product-philosophy.md");
const SEMANTIC_TREE_RULES: &str =
    include_str!("../../../docs/mcp-resources/semantic-tree-rules.md");
const TECHNICAL_DEPTH_RUBRIC: &str =
    include_str!("../../../docs/mcp-resources/technical-depth-rubric.md");
const BOOK_GRADE_STRUCTURE_DECISION: &str =
    include_str!("../../../docs/mcp-resources/book-grade-structure-decision.md");
const EXAMPLE_DEEP_CALCULATOR_PRODUCT: &str =
    include_str!("../../../docs/mcp-resources/example-deep-calculator-product.md");
const EXAMPLE_KUBERNETES_DASHBOARD_PRODUCT: &str =
    include_str!("../../../docs/mcp-resources/example-kubernetes-dashboard-product.md");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ResourceAnnotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAnnotations {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTextContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub text: String,
}

struct StaticResource {
    uri: &'static str,
    name: &'static str,
    title: &'static str,
    description: &'static str,
    mime_type: &'static str,
    priority: f64,
    content: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeKindConstraintsResource {
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

pub fn definitions() -> Vec<ResourceDefinition> {
    let mut resources = static_resources()
        .iter()
        .map(|resource| ResourceDefinition {
            uri: resource.uri.to_string(),
            name: resource.name.to_string(),
            title: Some(resource.title.to_string()),
            description: Some(resource.description.to_string()),
            mime_type: Some(resource.mime_type.to_string()),
            annotations: Some(ResourceAnnotations {
                audience: vec!["user".to_string(), "assistant".to_string()],
                priority: Some(resource.priority),
                last_modified: Some(RESOURCE_LAST_MODIFIED.to_string()),
            }),
            size: Some(resource.content.len()),
        })
        .collect::<Vec<_>>();
    resources.push(ResourceDefinition {
        uri: "aruvi://catalog/node-kind-constraints".to_string(),
        name: "node-kind-constraints".to_string(),
        title: Some("Aruvi Node Kind Constraints".to_string()),
        description: Some(
            "Machine-readable node kind constraints, root kinds, allowed child kinds, and rollout/reference leaf behavior."
                .to_string(),
        ),
        mime_type: Some(JSON_RESOURCE_MIME_TYPE.to_string()),
        annotations: Some(ResourceAnnotations {
            audience: vec!["user".to_string(), "assistant".to_string()],
            priority: Some(1.0),
            last_modified: Some(RESOURCE_LAST_MODIFIED.to_string()),
        }),
        size: None,
    });
    resources
}

pub fn read(uri: &str) -> Option<ResourceTextContent> {
    if uri == "aruvi://catalog/node-kind-constraints" {
        let text = serde_json::to_string_pretty(&build_node_kind_constraints_resource()).ok()?;
        return Some(ResourceTextContent {
            uri: uri.to_string(),
            mime_type: Some(JSON_RESOURCE_MIME_TYPE.to_string()),
            text,
        });
    }

    static_resources()
        .iter()
        .find(|resource| resource.uri == uri)
        .map(|resource| ResourceTextContent {
            uri: resource.uri.to_string(),
            mime_type: Some(resource.mime_type.to_string()),
            text: resource.content.to_string(),
        })
}

fn static_resources() -> &'static [StaticResource] {
    &[
        StaticResource {
            uri: "aruvi://guides/product-philosophy",
            name: "product-philosophy",
            title: "Aruvi Product Philosophy",
            description: "Core doctrine for modeling products as readable technical systems instead of shallow planning trees.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: PRODUCT_PHILOSOPHY,
        },
        StaticResource {
            uri: "aruvi://guides/semantic-tree-rules",
            name: "semantic-tree-rules",
            title: "Aruvi Semantic Tree Rules",
            description: "Canonical node kinds, allowed child rules, and authoring constraints for the product hierarchy.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: SEMANTIC_TREE_RULES,
        },
        StaticResource {
            uri: "aruvi://guides/technical-depth-rubric",
            name: "technical-depth-rubric",
            title: "Aruvi Technical Depth Rubric",
            description: "Defines what technical depth means in Aruvi and how to recognize shallow product structure.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 1.0,
            content: TECHNICAL_DEPTH_RUBRIC,
        },
        StaticResource {
            uri: "aruvi://guides/book-grade-structure-decision",
            name: "book-grade-structure-decision",
            title: "Decision: Keep Rollouts as Execution Leaves",
            description: "Explains the accepted modeling decision for book-grade detail: rollouts stay leaves and richer chapter structure belongs above them.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 0.95,
            content: BOOK_GRADE_STRUCTURE_DECISION,
        },
        StaticResource {
            uri: "aruvi://examples/deep-calculator-product",
            name: "deep-calculator-product",
            title: "Gold-Standard Example: Scientific Calculator",
            description: "A technically deep calculator product model showing meaningful areas, capabilities, state concerns, and rollouts.",
            mime_type: MARKDOWN_RESOURCE_MIME_TYPE,
            priority: 0.9,
            content: EXAMPLE_DEEP_CALCULATOR_PRODUCT,
        },
        StaticResource {
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

fn build_node_kind_constraints_resource() -> NodeKindConstraintsResource {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_include_requested_doctrine_resources() {
        let resources = definitions();
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://guides/product-philosophy")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://guides/semantic-tree-rules")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://guides/technical-depth-rubric")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://guides/book-grade-structure-decision")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://catalog/node-kind-constraints")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://examples/deep-calculator-product")
        );
        assert!(
            resources
                .iter()
                .any(|resource| resource.uri == "aruvi://examples/kubernetes-dashboard-product")
        );
    }

    #[test]
    fn read_returns_markdown_for_known_resource() {
        let content = read("aruvi://guides/product-philosophy").expect("known resource");
        assert_eq!(content.mime_type.as_deref(), Some(MARKDOWN_RESOURCE_MIME_TYPE));
        assert!(content.text.contains("# Aruvi Product Philosophy"));
    }

    #[test]
    fn read_returns_none_for_unknown_resource() {
        assert!(read("aruvi://guides/does-not-exist").is_none());
    }

    #[test]
    fn read_returns_json_for_node_kind_constraints_resource() {
        let content =
            read("aruvi://catalog/node-kind-constraints").expect("node-kind constraints");
        assert_eq!(content.mime_type.as_deref(), Some(JSON_RESOURCE_MIME_TYPE));
        assert!(content.text.contains("\"nodeKind\": \"rollout\""));
        assert!(content.text.contains("\"canHaveChildren\": false"));
    }
}

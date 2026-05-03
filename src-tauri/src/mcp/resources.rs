use crate::planning_doctrine::{
    self, JSON_RESOURCE_MIME_TYPE, NODE_KIND_CONSTRAINTS_URI, RESOURCE_LAST_MODIFIED,
};
use serde::Serialize;

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

pub fn definitions() -> Vec<ResourceDefinition> {
    let mut resources = planning_doctrine::static_resources()
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
        uri: NODE_KIND_CONSTRAINTS_URI.to_string(),
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
    if uri == NODE_KIND_CONSTRAINTS_URI {
        let text = planning_doctrine::node_kind_constraints_json().ok()?;
        return Some(ResourceTextContent {
            uri: uri.to_string(),
            mime_type: Some(JSON_RESOURCE_MIME_TYPE.to_string()),
            text,
        });
    }

    planning_doctrine::static_resources()
        .iter()
        .find(|resource| resource.uri == uri)
        .map(|resource| ResourceTextContent {
            uri: resource.uri.to_string(),
            mime_type: Some(resource.mime_type.to_string()),
            text: resource.content.to_string(),
        })
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
        assert_eq!(
            content.mime_type.as_deref(),
            Some(planning_doctrine::MARKDOWN_RESOURCE_MIME_TYPE)
        );
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

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryManifestSummary {
    pub(crate) path: String,
    pub(crate) ecosystem: String,
    pub(crate) package_name: Option<String>,
    pub(crate) framework_hints: Vec<String>,
    pub(crate) scripts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryDocSummary {
    pub(crate) path: String,
    pub(crate) headings: Vec<String>,
    pub(crate) excerpt: String,
}

pub(crate) fn parse_package_json_manifest(
    path: &str,
    content: &str,
) -> Option<RepositoryManifestSummary> {
    let value: Value = serde_json::from_str(content).ok()?;
    let package_name = value
        .get("name")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let scripts = value
        .get("scripts")
        .and_then(Value::as_object)
        .map(|scripts| scripts.keys().take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let dependency_names = ["dependencies", "devDependencies", "peerDependencies"]
        .into_iter()
        .filter_map(|section| value.get(section).and_then(Value::as_object))
        .flat_map(|deps| deps.keys().cloned().collect::<Vec<_>>())
        .collect::<BTreeSet<_>>();
    let mut framework_hints = vec![];
    if dependency_names.contains("react") {
        framework_hints.push("React".to_string());
    }
    if dependency_names.contains("next") {
        framework_hints.push("Next.js".to_string());
    }
    if dependency_names.contains("@tanstack/react-query") {
        framework_hints.push("React Query".to_string());
    }
    if dependency_names.contains("@tauri-apps/api")
        || dependency_names.contains("@tauri-apps/plugin-dialog")
    {
        framework_hints.push("Tauri".to_string());
    }
    if dependency_names.contains("vite") {
        framework_hints.push("Vite".to_string());
    }
    if dependency_names.contains("playwright") || dependency_names.contains("@playwright/test") {
        framework_hints.push("Playwright".to_string());
    }
    Some(RepositoryManifestSummary {
        path: path.to_string(),
        ecosystem: "node".to_string(),
        package_name,
        framework_hints,
        scripts,
    })
}

pub(crate) fn parse_cargo_manifest(path: &str, content: &str) -> Option<RepositoryManifestSummary> {
    let mut package_name = None;
    let mut framework_hints = vec![];
    for line in content.lines() {
        let trimmed = line.trim();
        if package_name.is_none() && trimmed.starts_with("name") && trimmed.contains('=') {
            let value = trimmed.split('=').nth(1)?.trim().trim_matches('"');
            if !value.is_empty() {
                package_name = Some(value.to_string());
            }
        }
        if trimmed.starts_with("tauri") || trimmed.contains("tauri =") {
            framework_hints.push("Tauri".to_string());
        }
        if trimmed.starts_with("sqlx") || trimmed.contains("sqlx =") {
            framework_hints.push("SQLx".to_string());
        }
        if trimmed.starts_with("tokio") || trimmed.contains("tokio =") {
            framework_hints.push("Tokio".to_string());
        }
    }
    framework_hints.sort();
    framework_hints.dedup();
    Some(RepositoryManifestSummary {
        path: path.to_string(),
        ecosystem: "rust".to_string(),
        package_name,
        framework_hints,
        scripts: vec![],
    })
}

pub(crate) fn extract_markdown_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') {
                return None;
            }
            Some(trimmed.trim_start_matches('#').trim().to_string())
        })
        .filter(|heading| !heading.is_empty())
        .take(8)
        .collect::<Vec<_>>()
}

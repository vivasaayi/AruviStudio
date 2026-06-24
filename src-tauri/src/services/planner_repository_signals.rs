use crate::domain::repository::{Repository, RepositoryTreeNode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

pub(crate) use crate::services::planner_repository_metadata::{
    extract_markdown_headings, parse_cargo_manifest, parse_package_json_manifest,
    RepositoryDocSummary, RepositoryManifestSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryRouteSummary {
    pub(crate) path: String,
    pub(crate) route: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryTestSummary {
    pub(crate) path: String,
    pub(crate) framework_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryAreaCandidate {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) confidence: String,
    pub(crate) rationale: String,
    pub(crate) evidence: Vec<String>,
}

pub(crate) fn flatten_repository_tree_lines(
    nodes: &[RepositoryTreeNode],
    depth: usize,
    remaining: &mut usize,
    output: &mut Vec<String>,
) {
    if *remaining == 0 {
        return;
    }
    let indent = "  ".repeat(depth);
    for node in nodes {
        if *remaining == 0 {
            break;
        }
        let marker = match node.node_type.as_str() {
            "directory" => "dir",
            _ => "file",
        };
        output.push(format!("{indent}- [{}] {}", marker, node.relative_path));
        *remaining -= 1;
        if !node.children.is_empty() {
            flatten_repository_tree_lines(&node.children, depth + 1, remaining, output);
        }
    }
}

pub(crate) fn flatten_repository_paths(nodes: &[RepositoryTreeNode], output: &mut Vec<String>) {
    for node in nodes {
        output.push(node.relative_path.clone());
        if !node.children.is_empty() {
            flatten_repository_paths(&node.children, output);
        }
    }
}

pub(crate) fn collect_repository_candidate_files(
    repo: &Repository,
    all_paths: &[String],
) -> Vec<String> {
    let preferred = [
        "README.md",
        "README",
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "src/App.tsx",
        "src/main.tsx",
        "src-tauri/Cargo.toml",
        "src-tauri/src/lib.rs",
    ];
    let mut seen = BTreeSet::new();
    for relative_path in preferred {
        let candidate = std::path::Path::new(&repo.local_path).join(relative_path);
        if candidate.exists() && candidate.is_file() {
            seen.insert(relative_path.to_string());
        }
    }
    for path in all_paths {
        let lower = path.to_lowercase();
        if lower.ends_with("readme.md")
            || lower.ends_with("package.json")
            || lower.ends_with("cargo.toml")
            || lower.ends_with("pyproject.toml")
        {
            seen.insert(path.clone());
        }
    }
    seen.into_iter().collect::<Vec<_>>()
}

pub(crate) fn detect_route_from_path(path: &str) -> Option<RepositoryRouteSummary> {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_lowercase();
    let route = if lower.starts_with("app/")
        && (lower.ends_with("/page.tsx")
            || lower.ends_with("/page.jsx")
            || lower.ends_with("/page.ts"))
    {
        let mut parts = normalized.split('/').skip(1).collect::<Vec<_>>();
        if parts.last().is_some() {
            parts.pop();
        }
        let route_parts = parts
            .into_iter()
            .filter(|part| {
                !part.starts_with('(') && !part.starts_with('@') && !part.starts_with('_')
            })
            .collect::<Vec<_>>();
        format!("/{}", route_parts.join("/"))
    } else if lower.starts_with("src/pages/") {
        let route = normalized
            .trim_start_matches("src/pages/")
            .trim_end_matches(".tsx")
            .trim_end_matches(".jsx")
            .trim_end_matches(".ts")
            .trim_end_matches(".js");
        format!("/{}", route.trim_end_matches("/index"))
    } else if lower.contains("/api/") {
        let route = normalized
            .split("/api/")
            .nth(1)
            .unwrap_or_default()
            .trim_end_matches(".ts")
            .trim_end_matches(".js");
        format!("/api/{}", route)
    } else {
        return None;
    };
    Some(RepositoryRouteSummary {
        path: normalized,
        route,
        kind: if lower.contains("/api/") {
            "api".to_string()
        } else {
            "page".to_string()
        },
    })
}

pub(crate) fn detect_test_file(path: &str) -> Option<RepositoryTestSummary> {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_lowercase();
    let is_test = lower.contains("/tests/")
        || lower.contains("__tests__")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.tsx")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.tsx")
        || lower.ends_with("_test.rs");
    if !is_test {
        return None;
    }
    let framework_hint = if lower.contains("playwright") {
        Some("Playwright".to_string())
    } else if lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.tsx")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.tsx")
    {
        Some("Vitest/Jest-style".to_string())
    } else if lower.ends_with("_test.rs") {
        Some("Rust test".to_string())
    } else {
        None
    };
    Some(RepositoryTestSummary {
        path: normalized,
        framework_hint,
    })
}

pub(crate) fn build_candidate_areas(
    paths: &[String],
    routes: &[RepositoryRouteSummary],
) -> Vec<RepositoryAreaCandidate> {
    let mut area_evidence: HashMap<String, Vec<String>> = HashMap::new();
    for path in paths {
        let normalized = path.replace('\\', "/");
        let segments = normalized.split('/').collect::<Vec<_>>();
        let area_segment = if segments.len() > 2
            && segments[0] == "src"
            && (segments[1] == "features" || segments[1] == "product_areas")
        {
            Some(segments[2])
        } else if segments.len() > 1 && segments[0] == "app" {
            segments
                .iter()
                .skip(1)
                .find(|segment| {
                    !segment.starts_with('(')
                        && !segment.starts_with('_')
                        && !segment.ends_with(".tsx")
                        && !segment.ends_with(".ts")
                })
                .copied()
        } else if segments.len() > 1 && segments[0] == "packages" {
            Some(segments[1])
        } else {
            None
        };
        if let Some(segment) = area_segment {
            let normalized_segment = normalize_identifier_token(segment);
            if normalized_segment.is_empty() {
                continue;
            }
            area_evidence
                .entry(normalized_segment)
                .or_default()
                .push(format!("path: {}", normalized));
        }
    }
    for route in routes {
        for segment in route.route.split('/') {
            let normalized_segment = normalize_identifier_token(segment);
            if normalized_segment.is_empty() || normalized_segment == "api" {
                continue;
            }
            area_evidence
                .entry(normalized_segment)
                .or_default()
                .push(format!("route: {}", route.route));
            break;
        }
    }

    let mut areas = area_evidence
        .into_iter()
        .map(|(name, mut evidence)| {
            evidence.sort();
            evidence.dedup();
            let confidence = if evidence.len() >= 4 {
                "high"
            } else if evidence.len() >= 2 {
                "medium"
            } else {
                "low"
            };
            RepositoryAreaCandidate {
                name: humanize_identifier(&name),
                kind: "feature_area".to_string(),
                confidence: confidence.to_string(),
                rationale: format!(
                    "Grouped from {} repository signals under a common directory or route segment.",
                    evidence.len()
                ),
                evidence: evidence.into_iter().take(6).collect::<Vec<_>>(),
            }
        })
        .filter(|area| {
            let normalized = normalize_identifier_token(&area.name);
            !normalized.is_empty()
                && !["src", "app", "pages", "components", "lib", "tests", "test"]
                    .contains(&normalized.as_str())
        })
        .collect::<Vec<_>>();
    areas.sort_by(|left, right| {
        right
            .evidence
            .len()
            .cmp(&left.evidence.len())
            .then(left.name.cmp(&right.name))
    });
    areas.truncate(10);
    areas
}

fn normalize_identifier_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| !ch.is_alphanumeric())
        .to_lowercase()
}

fn humanize_identifier(value: &str) -> String {
    let mut spaced = String::new();
    let mut prev_lower_or_digit = false;
    for ch in value.chars() {
        if ch == '-' || ch == '_' || ch == '/' {
            if !spaced.ends_with(' ') {
                spaced.push(' ');
            }
            prev_lower_or_digit = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower_or_digit && !spaced.ends_with(' ') {
            spaced.push(' ');
        }
        spaced.push(ch);
        prev_lower_or_digit = ch.is_lowercase() || ch.is_ascii_digit();
    }
    spaced
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

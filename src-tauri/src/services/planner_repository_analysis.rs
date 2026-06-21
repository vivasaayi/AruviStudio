use crate::domain::repository::{Repository, RepositoryTreeNode};
use crate::error::AppError;
use crate::services::repo_service;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepositoryAnalysisSnapshot {
    pub(crate) repository_name: String,
    pub(crate) local_path: String,
    pub(crate) default_branch: String,
    pub(crate) remote_url: Option<String>,
    pub(crate) tree_overview: Vec<String>,
    pub(crate) manifests: Vec<RepositoryManifestSummary>,
    pub(crate) docs: Vec<RepositoryDocSummary>,
    pub(crate) routes: Vec<RepositoryRouteSummary>,
    pub(crate) tests: Vec<RepositoryTestSummary>,
    pub(crate) candidate_areas: Vec<RepositoryAreaCandidate>,
}

pub(crate) fn repository_analysis_prompt() -> &'static str {
    r#"You are an AI planning lead reverse-engineering a software repository into a staged product plan.
Return exactly one JSON object of type "final". No markdown.

Your task is to inspect the provided structured repository analysis snapshot and convert it into the selected product's staged design tree using these action types only:
update_product,
create_product_area, update_product_area,
create_capability, update_capability,
create_work_item, update_work_item,
report_tree.

Rules:
- Base the structure on the provided evidence, not wishful features.
- The selected product is already created. Do not create another product.
- If a selected design node is provided, merge into that context instead of creating a duplicate branch.
- Prefer a product-first design structure:
  - 1 product root
  - 2-6 product areas
  - capabilities and features where the codebase clearly shows product design structure
  - 1-3 starter stories/tasks per concrete feature or directly executable capability where implementation work is visible or obviously missing
- Keep delivery execution in stories/tasks, not in strategy.
- When a topic deserves book-grade depth, add explanation, examples, implementationNotes, and testGuidance fields instead of stopping at shallow summaries.
- Use create_* when adding inferred structure to the staged design.
- Use update_* when refining an already selected/root design node from repository evidence.
- Keep names concise and product-manager friendly.
- Mention assumptions briefly in assistant_response.
- Prioritize explicit signals from docs, manifests, routes, tests, and candidate areas.
- Avoid generic product areas that are not supported by the evidence.
- Optional but preferred: include an analysis object on actions with:
  {
    "source": "repository_analysis",
    "confidence": "high|medium|low",
    "evidence": ["short evidence line", "..."]
  }
- If the repository evidence is too weak, return actions=[] with a clarification_question asking what to focus on.

Return this shape:
{
  "type": "final",
  "assistant_response": "brief natural-language summary",
  "needs_confirmation": false,
  "clarification_question": null,
  "actions": []
}"#
}

fn flatten_repository_tree_lines(
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

pub(crate) fn truncate_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let truncated = trimmed.chars().take(max_chars).collect::<String>();
    format!("{truncated}\n...[truncated]")
}

fn flatten_repository_paths(nodes: &[RepositoryTreeNode], output: &mut Vec<String>) {
    for node in nodes {
        output.push(node.relative_path.clone());
        if !node.children.is_empty() {
            flatten_repository_paths(&node.children, output);
        }
    }
}

fn collect_repository_candidate_files(repo: &Repository, all_paths: &[String]) -> Vec<String> {
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

fn parse_package_json_manifest(path: &str, content: &str) -> Option<RepositoryManifestSummary> {
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

fn parse_cargo_manifest(path: &str, content: &str) -> Option<RepositoryManifestSummary> {
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

fn extract_markdown_headings(content: &str) -> Vec<String> {
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

fn detect_route_from_path(path: &str) -> Option<RepositoryRouteSummary> {
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

fn detect_test_file(path: &str) -> Option<RepositoryTestSummary> {
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

fn build_candidate_areas(
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

pub(crate) fn build_repository_analysis_snapshot(
    repo: &Repository,
) -> Result<RepositoryAnalysisSnapshot, AppError> {
    let tree = repo_service::list_repository_tree(&repo.local_path, false, Some(6))?;
    let mut remaining = 220usize;
    let mut tree_lines = vec![];
    flatten_repository_tree_lines(&tree, 0, &mut remaining, &mut tree_lines);
    let mut all_paths = vec![];
    flatten_repository_paths(&tree, &mut all_paths);
    all_paths.sort();

    let candidate_files = collect_repository_candidate_files(repo, &all_paths);
    let mut manifests = vec![];
    let mut docs = vec![];
    for relative_path in candidate_files.into_iter().take(16) {
        let Ok(content) = repo_service::read_repository_file(&repo.local_path, &relative_path)
        else {
            continue;
        };
        let lower = relative_path.to_lowercase();
        if lower.ends_with("package.json") {
            if let Some(manifest) = parse_package_json_manifest(&relative_path, &content) {
                manifests.push(manifest);
            }
        } else if lower.ends_with("cargo.toml") {
            if let Some(manifest) = parse_cargo_manifest(&relative_path, &content) {
                manifests.push(manifest);
            }
        } else if lower.ends_with("readme.md") || lower.ends_with("/readme.md") || lower == "readme"
        {
            docs.push(RepositoryDocSummary {
                path: relative_path.clone(),
                headings: extract_markdown_headings(&content),
                excerpt: truncate_text(&content, 1200),
            });
        }
    }

    let routes = all_paths
        .iter()
        .filter_map(|path| detect_route_from_path(path))
        .take(20)
        .collect::<Vec<_>>();
    let tests = all_paths
        .iter()
        .filter_map(|path| detect_test_file(path))
        .take(20)
        .collect::<Vec<_>>();
    let candidate_areas = build_candidate_areas(&all_paths, &routes);

    Ok(RepositoryAnalysisSnapshot {
        repository_name: repo.name.clone(),
        local_path: repo.local_path.clone(),
        default_branch: repo.default_branch.clone(),
        remote_url: if repo.remote_url.is_empty() {
            None
        } else {
            Some(repo.remote_url.clone())
        },
        tree_overview: tree_lines,
        manifests,
        docs,
        routes,
        tests,
        candidate_areas,
    })
}

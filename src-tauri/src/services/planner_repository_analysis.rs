use crate::domain::repository::Repository;
use crate::error::AppError;
use crate::services::planner_repository_signals::{
    build_candidate_areas, collect_repository_candidate_files, detect_route_from_path,
    detect_test_file, extract_markdown_headings, flatten_repository_paths,
    flatten_repository_tree_lines, parse_cargo_manifest, parse_package_json_manifest,
    RepositoryAreaCandidate, RepositoryDocSummary, RepositoryManifestSummary,
    RepositoryRouteSummary, RepositoryTestSummary,
};
use crate::services::repo_service;
use serde::{Deserialize, Serialize};

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

pub(crate) fn truncate_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let truncated = trimmed.chars().take(max_chars).collect::<String>();
    format!("{truncated}\n...[truncated]")
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

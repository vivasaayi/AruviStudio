use super::*;
use crate::domain::repository::Repository;
use crate::services::planner_repository_analysis::{
    build_repository_analysis_snapshot, RepositoryAnalysisSnapshot,
};
use std::fs;
use std::path::Path;

fn make_repository(temp_root: &Path, name: &str) -> Repository {
    Repository {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        local_path: temp_root.display().to_string(),
        remote_url: "".to_string(),
        default_branch: "main".to_string(),
        auth_profile: None,
        created_at: "2026-03-21 00:00:00".to_string(),
        updated_at: "2026-03-21 00:00:00".to_string(),
    }
}

#[test]
fn build_repository_analysis_snapshot_extracts_structured_signals() {
    let temp_root = make_temp_dir("repo_analysis_snapshot");
    fs::create_dir_all(temp_root.join("src/features/planner"))
        .expect("failed to create feature dir");
    fs::create_dir_all(temp_root.join("app/hotels")).expect("failed to create route dir");
    fs::create_dir_all(temp_root.join("e2e")).expect("failed to create e2e dir");
    fs::write(
        temp_root.join("README.md"),
        "# Hotel Management System\n## Planner\nInteractive planning workspace.",
    )
    .expect("failed to write README");
    fs::write(
            temp_root.join("package.json"),
            r#"{
              "name": "hotel-management-system",
              "scripts": { "dev": "vite", "test:e2e": "playwright test" },
              "dependencies": { "react": "^18.0.0", "vite": "^5.0.0", "@tanstack/react-query": "^5.0.0" },
              "devDependencies": { "@playwright/test": "^1.0.0" }
            }"#,
        )
        .expect("failed to write package.json");
    fs::write(
        temp_root.join("src/features/planner/PlannerPage.tsx"),
        "export function PlannerPage() { return null; }",
    )
    .expect("failed to write planner page");
    fs::write(
        temp_root.join("app/hotels/page.tsx"),
        "export default function Hotels() { return null; }",
    )
    .expect("failed to write route");
    fs::write(
        temp_root.join("e2e/planner.spec.ts"),
        "test('planner', () => {});",
    )
    .expect("failed to write test");

    let snapshot: RepositoryAnalysisSnapshot =
        build_repository_analysis_snapshot(&make_repository(&temp_root, "hotel-management-system"))
            .expect("snapshot should build");

    assert!(
        !snapshot.manifests.is_empty(),
        "manifest signals should be extracted"
    );
    assert!(!snapshot.docs.is_empty(), "doc signals should be extracted");
    assert!(
        !snapshot.routes.is_empty(),
        "route signals should be extracted"
    );
    assert!(
        !snapshot.tests.is_empty(),
        "test signals should be extracted"
    );
    assert!(
        snapshot
            .candidate_areas
            .iter()
            .any(|area| area.name.contains("Planner") || area.name.contains("Hotels")),
        "candidate areas should include feature or route-derived areas"
    );
}

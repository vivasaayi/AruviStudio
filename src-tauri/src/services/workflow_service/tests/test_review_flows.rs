use super::helpers::*;
use super::WorkflowService;
use crate::persistence::{db as db_service, repository_repo, work_item_repo};
use crate::services::{agent_service::AgentService, model_service::ModelService};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn auto_test_review_continues_to_done_by_default() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("auto_test_review");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("repo");
    let remote_bare_dir = temp_root.join("origin.git");
    create_empty_calculator_test_repo(&repo_dir, &remote_bare_dir)
        .expect("failed to create repo with bare remote");

    let repository = repository_repo::create_repository(
        &pool,
        "workflow-auto-test-review-repo",
        "Workflow Auto Test Review Repo",
        &repo_dir.to_string_lossy(),
        &remote_bare_dir.to_string_lossy(),
        "main",
    )
    .await
    .expect("failed to create repository");

    let product = create_test_product(
        &pool,
        "workflow-auto-test-review-product",
        "Workflow Product",
    )
    .await;
    let product_area = create_test_product_area(
        &pool,
        "workflow-auto-test-review-product_area",
        &product.id,
        "Delivery ProductArea",
    )
    .await;

    let work_item = work_item_repo::create_work_item(
        &pool,
        test_work_item_input(
            "workflow-auto-test-review-work-item",
            &product.id,
            Some(&product_area.id),
            "Implement workflow continuation",
        ),
    )
    .await
    .expect("failed to create work item");

    sqlx::query("UPDATE work_items SET status='approved', active_repo_id=? WHERE id=?")
        .bind(&repository.id)
        .bind(&work_item.id)
        .execute(&pool)
        .await
        .expect("failed to prepare approved work item");

    let db_arc = Arc::new(pool.clone());
    let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
    let artifact_dir = temp_root.join("artifacts");
    let workspace_dir = temp_root.join("workspaces");
    std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
    std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

    let agent_service = AgentService::new(
        Arc::clone(&db_arc),
        Arc::clone(&model_service),
        artifact_dir.clone(),
        workspace_dir.clone(),
    );
    let workflow_service =
        WorkflowService::new(Arc::clone(&db_arc), Arc::new(Mutex::new(agent_service)));

    AgentService::set_test_model_outputs_for_any_workflow(vec![
        "requirement analysis complete".to_string(),
        "planning complete".to_string(),
        json!({
            "type": "tool_call",
            "tool": "repo.write_file",
            "reason": "implement approved plan",
            "arguments": {
                "path": "src/generated.rs",
                "content": "pub fn generated() -> &'static str { \"ok\" }\n"
            }
        })
        .to_string(),
        json!({
            "type": "final",
            "summary": "coding complete",
            "result": "implemented"
        })
        .to_string(),
        "unit test generation complete".to_string(),
        "integration test generation complete".to_string(),
        "ui test planning complete".to_string(),
        "qa validation complete".to_string(),
        "security review complete".to_string(),
        "performance review complete".to_string(),
    ]);

    let workflow_run = workflow_service
        .start_work_item_workflow(&work_item.id)
        .await
        .expect("failed to start workflow");

    let final_state = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to load workflow after auto test review");
    assert_eq!(final_state.current_stage, "done");

    let approvals = crate::persistence::approval_repo::list_approvals(&pool, &work_item.id)
        .await
        .expect("failed to list approvals");
    assert!(
        approvals.iter().any(|approval| {
            approval.approval_type.to_string() == "test_review"
                && approval.status.to_string() == "approved"
                && approval.notes.contains("auto-approved")
        }),
        "expected auto-approved test review record"
    );

    let history = workflow_service
        .get_workflow_history(&workflow_run.id)
        .await
        .expect("failed to load workflow history");
    assert!(
        history.iter().any(|entry| {
            entry.from_stage == "pending_test_review"
                && entry.to_stage == "push_preparation"
                && entry.notes.contains("auto-approved")
        }),
        "missing auto-approved pending_test_review -> push_preparation transition"
    );

    AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
    let _ = std::fs::remove_dir_all(temp_root);
}

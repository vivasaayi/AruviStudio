use super::WorkflowService;
use crate::domain::workflow::UserAction;
use crate::persistence::{
    artifact_repo, db as db_service, product_repo, repository_repo, settings_repo, work_item_repo,
};
use crate::services::{
    agent_service::AgentService, model_service::ModelService, workflow_approval_gate,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

mod helpers;

use helpers::*;

#[tokio::test]
async fn plan_approval_continues_to_coding_and_records_history_and_artifacts() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("plan_approval");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
    std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
        .expect("failed to seed repository file");

    let repository = repository_repo::create_repository(
        &pool,
        "workflow-test-repo",
        "Workflow Test Repo",
        &repo_dir.to_string_lossy(),
        "",
        "main",
    )
    .await
    .expect("failed to create repository");

    let product = create_test_product(&pool, "workflow-product", "Workflow Product").await;
    let product_area = create_test_product_area(
        &pool,
        "workflow-product_area",
        &product.id,
        "Delivery ProductArea",
    )
    .await;

    let work_item = work_item_repo::create_work_item(
        &pool,
        test_work_item_input(
            "workflow-work-item",
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

    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_PLAN_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto plan approval for manual-gate test");
    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_TEST_REVIEW_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto test review for manual-gate test");

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

    let at_plan_gate = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to refresh workflow after start");
    assert_eq!(at_plan_gate.current_stage, "pending_plan_approval");

    let artifacts_before_approval = artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
        .await
        .expect("failed to list artifacts before plan approval");
    assert!(
        artifacts_before_approval
            .iter()
            .any(|artifact| artifact.artifact_type == "planning_prompt"),
        "missing planning_prompt artifact before plan approval"
    );
    assert!(
        artifacts_before_approval
            .iter()
            .any(|artifact| artifact.artifact_type == "planning_output"),
        "missing planning_output artifact before plan approval"
    );

    workflow_service
        .handle_user_action(
            &workflow_run.id,
            UserAction::Approve,
            Some("approve test plan".to_string()),
        )
        .await
        .expect("failed to approve plan and continue workflow");

    let post_approval = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to refresh workflow after approval");
    assert_eq!(post_approval.current_stage, "pending_test_review");

    let history = workflow_service
        .get_workflow_history(&workflow_run.id)
        .await
        .expect("failed to load workflow history");
    assert!(
        history.iter().any(|entry| {
            entry.from_stage == "planning" && entry.to_stage == "pending_plan_approval"
        }),
        "missing planning -> pending_plan_approval transition"
    );
    assert!(
        history.iter().any(|entry| {
            entry.from_stage == "pending_plan_approval" && entry.to_stage == "coding"
        }),
        "missing pending_plan_approval -> coding transition"
    );

    let artifacts_after_approval = artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
        .await
        .expect("failed to list artifacts after plan approval");
    assert!(
        artifacts_after_approval
            .iter()
            .any(|artifact| artifact.artifact_type == "coding_tool_trace"),
        "missing coding_tool_trace artifact after approval-driven coding"
    );
    assert!(
        artifacts_after_approval
            .iter()
            .any(|artifact| artifact.artifact_type == "coding_output"),
        "missing coding_output artifact after approval-driven coding"
    );

    let generated = std::fs::read_to_string(repo_dir.join("src/generated.rs"))
        .expect("expected generated file from coding stage");
    assert!(
        generated.contains("generated"),
        "generated file content did not match expected coding output"
    );

    AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
    let _ = std::fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn planning_auto_approval_continues_directly_to_coding_by_default() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("auto_plan_approval");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
    std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
        .expect("failed to seed repository file");

    let repository = repository_repo::create_repository(
        &pool,
        "workflow-auto-approval-repo",
        "Workflow Auto Approval Repo",
        &repo_dir.to_string_lossy(),
        "",
        "main",
    )
    .await
    .expect("failed to create repository");

    let product =
        create_test_product(&pool, "workflow-auto-approval-product", "Workflow Product").await;
    let product_area = create_test_product_area(
        &pool,
        "workflow-auto-approval-product_area",
        &product.id,
        "Delivery ProductArea",
    )
    .await;

    let work_item = work_item_repo::create_work_item(
        &pool,
        test_work_item_input(
            "workflow-auto-approval-work-item",
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

    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_TEST_REVIEW_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto test review for auto-plan test");

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

    let after_planning = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to load workflow after auto plan approval");
    assert_eq!(after_planning.current_stage, "pending_test_review");

    let approvals = crate::persistence::approval_repo::list_approvals(&pool, &work_item.id)
        .await
        .expect("failed to list approvals");
    assert!(
        approvals.iter().any(|approval| {
            approval.approval_type.to_string() == "plan_approval"
                && approval.status.to_string() == "approved"
                && approval.notes.contains("auto-approved")
        }),
        "expected auto-approved plan approval record"
    );

    let history = workflow_service
        .get_workflow_history(&workflow_run.id)
        .await
        .expect("failed to load workflow history");
    assert!(
        history.iter().any(|entry| {
            entry.from_stage == "pending_plan_approval"
                && entry.to_stage == "coding"
                && entry.notes.contains("auto-approved")
        }),
        "missing auto-approved pending_plan_approval -> coding transition"
    );

    AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
    let _ = std::fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn plan_rejection_restarts_analysis_and_returns_to_plan_gate() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("plan_rejection");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
    std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
        .expect("failed to seed repository file");

    let repository = repository_repo::create_repository(
        &pool,
        "workflow-plan-rejection-repo",
        "Workflow Plan Rejection Repo",
        &repo_dir.to_string_lossy(),
        "",
        "main",
    )
    .await
    .expect("failed to create repository");

    let product =
        create_test_product(&pool, "workflow-plan-rejection-product", "Workflow Product").await;
    let product_area = create_test_product_area(
        &pool,
        "workflow-plan-rejection-product_area",
        &product.id,
        "Delivery ProductArea",
    )
    .await;

    let work_item = work_item_repo::create_work_item(
        &pool,
        test_work_item_input(
            "workflow-plan-rejection-work-item",
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

    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_PLAN_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto plan approval");
    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_TEST_REVIEW_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto test review");

    AgentService::set_test_model_outputs_for_any_workflow(vec![
        "requirement analysis complete".to_string(),
        "planning complete".to_string(),
        "requirement analysis retry complete".to_string(),
        "planning retry complete".to_string(),
    ]);

    let workflow_run = workflow_service
        .start_work_item_workflow(&work_item.id)
        .await
        .expect("failed to start workflow");

    let at_plan_gate = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to refresh workflow after start");
    assert_eq!(at_plan_gate.current_stage, "pending_plan_approval");

    workflow_service
        .handle_user_action(
            &workflow_run.id,
            UserAction::Reject,
            Some("needs more detail".to_string()),
        )
        .await
        .expect("failed to reject plan and restart workflow");

    let after_rejection = workflow_service
        .get_workflow_run(&workflow_run.id)
        .await
        .expect("failed to refresh workflow after rejection");
    assert_eq!(after_rejection.current_stage, "pending_plan_approval");
    assert_eq!(after_rejection.status, "running");

    let history = workflow_service
        .get_workflow_history(&workflow_run.id)
        .await
        .expect("failed to load workflow history");
    assert!(
        history.iter().any(|entry| {
            entry.from_stage == "pending_plan_approval"
                && entry.to_stage == "requirement_analysis"
                && entry.notes.contains("needs more detail")
        }),
        "missing rejection transition back to requirement analysis"
    );
    assert!(
        history
            .iter()
            .filter(|entry| {
                entry.from_stage == "planning" && entry.to_stage == "pending_plan_approval"
            })
            .count()
            >= 2,
        "expected planning to reach the plan gate again after rejection"
    );

    AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
    let _ = std::fs::remove_dir_all(temp_root);
}

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

#[tokio::test]
async fn start_workflow_closes_orphaned_coordinator_review_and_creates_fresh_run() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("orphaned_coordinator_recovery");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
    std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
        .expect("failed to seed repository file");

    let repository = repository_repo::create_repository(
        &pool,
        "workflow-recovery-repo",
        "Workflow Recovery Repo",
        &repo_dir.to_string_lossy(),
        "",
        "main",
    )
    .await
    .expect("failed to create repository");

    let product = create_test_product(&pool, "workflow-recovery-product", "Workflow Product").await;
    let product_area = create_test_product_area(
        &pool,
        "workflow-recovery-product_area",
        &product.id,
        "Delivery ProductArea",
    )
    .await;

    let work_item = work_item_repo::create_work_item(
        &pool,
        test_work_item_input(
            "workflow-recovery-work-item",
            &product.id,
            Some(&product_area.id),
            "Recover workflow start",
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

    sqlx::query(
        "INSERT INTO workflow_runs (
                id, work_item_id, workflow_version, status, current_stage, retry_count, max_retries,
                started_at, updated_at
             ) VALUES (
                'workflow-orphaned-review', ?, 'v1', 'running', 'coordinator_review', 0, 3,
                datetime('now', '-30 minutes'), datetime('now', '-30 minutes')
             )",
    )
    .bind(&work_item.id)
    .execute(&pool)
    .await
    .expect("failed to seed orphaned workflow run");

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

    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_PLAN_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto plan approval");
    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_TEST_REVIEW_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto test review");

    AgentService::set_test_model_outputs_for_any_workflow(vec![
        "requirement analysis complete".to_string(),
        "planning complete".to_string(),
    ]);

    let fresh_run = workflow_service
        .start_work_item_workflow(&work_item.id)
        .await
        .expect("failed to start workflow after orphan recovery");

    assert_ne!(fresh_run.id, "workflow-orphaned-review");

    let orphaned = workflow_service
        .get_workflow_run("workflow-orphaned-review")
        .await
        .expect("failed to load orphaned workflow");
    assert_eq!(orphaned.status, "failed");
    assert_eq!(
        orphaned.error_message.as_deref(),
        Some("Auto-closed orphaned coordinator review run")
    );
    assert!(orphaned.ended_at.is_some());

    let refreshed_fresh = workflow_service
        .get_workflow_run(&fresh_run.id)
        .await
        .expect("failed to load fresh workflow run");
    assert_eq!(refreshed_fresh.current_stage, "pending_plan_approval");
    assert_eq!(refreshed_fresh.status, "running");

    AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
    let _ = std::fs::remove_dir_all(temp_root);
}

#[tokio::test]
#[ignore = "requires live model provider configuration and can take significant time"]
async fn live_calculator_iterative_workflow_smoke() {
    let _test_guard = acquire_workflow_test_lock().await;
    let temp_root = make_temp_dir("live_calculator");
    println!("LIVE_TEST_ROOT={}", temp_root.display());
    let db_path = temp_root.join("aruvi-live.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let repo_dir = temp_root.join("calculator-test-repo");
    let remote_bare_dir = temp_root.join("calculator-origin.git");
    create_empty_calculator_test_repo(&repo_dir, &remote_bare_dir)
        .expect("failed to create empty calculator test repository");
    let repository = repository_repo::create_repository(
        &pool,
        "calculator-repo",
        "Calculator React Test",
        &repo_dir.to_string_lossy(),
        &remote_bare_dir.to_string_lossy(),
        "main",
    )
    .await
    .expect("failed to register calculator repository");

    configure_live_model_bindings(&pool)
        .await
        .expect("failed to bind live model for agents");

    let product = product_repo::create_product(
        &pool,
        product_repo::CreateProductInput {
            id: "calculator-product",
            name: "Calculator",
            description: "Iterative calculator delivery with full workflow enforcement.",
            vision: "Deliver calculator outcomes in small, validated increments.",
            goals: "[]",
            tags: "[\"react\",\"calculator\",\"agentic\"]",
            lifecycle: Some("active"),
            health: Some("healthy"),
            owner_label: Some("Builder"),
            investment_status: Some("invest"),
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("failed to create Calculator product");

    let product_area = product_repo::create_product_area(
        &pool,
        product_repo::CreateProductAreaInput {
            id: "calculator-product_area",
            product_id: &product.id,
            name: "Calculator Engine",
            description: "Core capability delivery product_area for calculator behavior.",
            purpose: "Implement and validate calculator functionality end-to-end.",
            node_kind: None,
            explanation: "",
            examples: "",
            implementation_notes: "",
            test_guidance: "",
        },
    )
    .await
    .expect("failed to create calculator product_area");

    let bootstrap_work_item = work_item_repo::create_work_item(
        &pool,
        work_item_repo::CreateWorkItemInput {
            id: "work-item-bootstrap-initialize-repo",
            product_id: &product.id,
            product_area_id: Some(&product_area.id),
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title: "Initialize repository and test folder",
            problem_statement:
                "Initialize an empty calculator repository baseline before capability outcomes start shipping.",
            description:
                "Create the baseline repository structure (including an empty tests folder), commit the setup, and keep follow-up outcomes focused on incremental functional changes.",
            acceptance_criteria:
                "Repository baseline is committed, tests folder exists, and subsequent outcomes can commit changes without re-initializing the project.",
            constraints: "Do not implement calculator features in this bootstrap outcome.",
            work_item_type: "story",
            priority: "high",
            complexity: "low",
        },
    )
    .await
    .expect("failed to create bootstrap work item");
    sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
        .bind(&repository.id)
        .bind(&bootstrap_work_item.id)
        .execute(&pool)
        .await
        .expect("failed to assign active repository to bootstrap work item");

    let capability_specs: [(&str, &[&str]); 5] = [
        (
            "Simple Math",
            &["Addition", "Subtraction", "Multiplication", "Division"],
        ),
        ("Scientific", &["Sin", "Cos", "Tan"]),
        ("Exponents", &["Square", "Cube", "Power of X"]),
        ("Roots", &["Square", "Cube"]),
        ("Programming", &["ASCII", "HEX"]),
    ];

    let mut ordered_work_item_ids: Vec<String> = vec![bootstrap_work_item.id];
    for (capability_name, outcomes) in capability_specs {
        let capability_slug = capability_name.to_ascii_lowercase().replace(' ', "-");
        let capability_id = format!("capability-{capability_slug}");
        let capability_description =
            format!("{capability_name} capability for calculator outcomes");
        let capability_acceptance = format!("{capability_name} outcomes: {}", outcomes.join(", "));
        let capability = product_repo::create_capability(
            &pool,
            product_repo::CreateCapabilityInput {
                id: &capability_id,
                product_area_id: &product_area.id,
                parent_capability_id: None,
                name: capability_name,
                description: &capability_description,
                acceptance_criteria: &capability_acceptance,
                priority: "medium",
                risk: "low",
                technical_notes: "Build in iterative outcomes with full test gates.",
                node_kind: None,
                explanation: "",
                examples: "",
                implementation_notes: "",
                test_guidance: "",
            },
        )
        .await
        .expect("failed to create capability");

        for outcome in outcomes {
            let outcome_slug = outcome.to_ascii_lowercase().replace(' ', "-");
            let outcome_capability_id = format!("capability-{capability_slug}-{outcome_slug}");
            let outcome_description = format!("{outcome} outcome for {capability_name}");
            let outcome_acceptance =
                format!("Calculator supports {outcome} for {capability_name}.");
            let outcome_capability = product_repo::create_capability(
                &pool,
                product_repo::CreateCapabilityInput {
                    id: &outcome_capability_id,
                    product_area_id: &product_area.id,
                    parent_capability_id: Some(&capability.id),
                    name: outcome,
                    description: &outcome_description,
                    acceptance_criteria: &outcome_acceptance,
                    priority: "medium",
                    risk: "low",
                    technical_notes: "Deliver as a focused outcome with full workflow validation.",
                    node_kind: None,
                    explanation: "",
                    examples: "",
                    implementation_notes: "",
                    test_guidance: "",
                },
            )
            .await
            .expect("failed to create outcome capability");

            let work_item_id = format!("work-item-{}-{}", capability_slug, outcome_slug);
            let title = format!("{capability_name}: {outcome}");
            let problem_statement = format!("Implement {outcome} behavior for {capability_name}.");
            let description = format!(
                "Deliver the {outcome} outcome under {capability_name} in the React calculator with iterative commits and review gates."
            );
            let work_item = work_item_repo::create_work_item(
                &pool,
                work_item_repo::CreateWorkItemInput {
                    id: &work_item_id,
                    product_id: &product.id,
                    product_area_id: Some(&product_area.id),
                    capability_id: Some(&outcome_capability.id),
                    source_node_id: None,
                    source_node_type: None,
                    parent_work_item_id: None,
                    title: &title,
                    problem_statement: &problem_statement,
                    description: &description,
                    acceptance_criteria:
                        "Component behavior, unit tests, integration tests, and UI tests pass.",
                    constraints: "Stay inside React codebase and calculator scope.",
                    work_item_type: "story",
                    priority: "medium",
                    complexity: "medium",
                },
            )
            .await
            .expect("failed to create work item");

            sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
                .bind(&repository.id)
                .bind(&work_item.id)
                .execute(&pool)
                .await
                .expect("failed to assign active repository");

            ordered_work_item_ids.push(work_item.id);
        }
    }

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

    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_PLAN_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto plan approval for live smoke test");
    settings_repo::set_setting(
        &pool,
        workflow_approval_gate::AUTO_APPROVE_TEST_REVIEW_KEY,
        "false",
    )
    .await
    .expect("failed to disable auto test review for live smoke test");

    let max_iterations = std::env::var("ARUVI_LIVE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let approval_timeout = Duration::from_secs(
        std::env::var("ARUVI_LIVE_STAGE_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1800),
    );
    let complete_to_done = std::env::var("ARUVI_LIVE_COMPLETE_TO_DONE")
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(true);
    let keep_temp = std::env::var("ARUVI_LIVE_KEEP_TEMP")
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    for work_item_id in ordered_work_item_ids.into_iter().take(max_iterations) {
        work_item_repo::update_work_item(
            &pool,
            work_item_repo::UpdateWorkItemPatch {
                id: &work_item_id,
                status: Some("approved"),
                title: None,
                description: None,
                problem_statement: None,
                acceptance_criteria: None,
                constraints: None,
            },
        )
        .await
        .expect("failed to approve work item for workflow start");

        let workflow_run = workflow_service
            .start_work_item_workflow(&work_item_id)
            .await
            .expect("failed to start workflow for live iteration");

        wait_for_stage(
            &workflow_service,
            &workflow_run.id,
            "pending_plan_approval",
            approval_timeout,
        )
        .await
        .expect("workflow never reached pending_plan_approval");

        workflow_service
            .handle_user_action(
                &workflow_run.id,
                UserAction::Approve,
                Some("Auto-approved plan for live iterative test".to_string()),
            )
            .await
            .expect("failed to approve plan in live iteration");

        wait_for_stage(
            &workflow_service,
            &workflow_run.id,
            "pending_test_review",
            approval_timeout,
        )
        .await
        .expect("workflow never reached pending_test_review after plan approval");

        let artifacts = artifact_repo::list_work_item_artifacts(&pool, &work_item_id)
            .await
            .expect("failed to list artifacts for live iteration");
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.artifact_type == "coding_tool_trace"),
            "expected coding_tool_trace artifact for work item {}",
            work_item_id
        );

        if complete_to_done {
            workflow_service
                .handle_user_action(
                    &workflow_run.id,
                    UserAction::Approve,
                    Some("Auto-approved test review for live iterative test".to_string()),
                )
                .await
                .expect("failed to approve test review in live iteration");
            wait_for_stage(
                &workflow_service,
                &workflow_run.id,
                "done",
                approval_timeout,
            )
            .await
            .expect("workflow never reached done after test review approval");
        }
    }

    if keep_temp {
        println!("LIVE_TEST_ROOT_PRESERVED={}", temp_root.display());
    } else {
        let _ = std::fs::remove_dir_all(temp_root);
    }
}

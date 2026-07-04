use super::*;

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

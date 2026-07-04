use super::*;

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

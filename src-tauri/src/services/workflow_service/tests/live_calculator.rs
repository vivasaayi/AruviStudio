use super::super::WorkflowService;
use super::helpers::*;
use crate::domain::workflow::UserAction;
use crate::persistence::{
    artifact_repo, db as db_service, product_repo, repository_repo, settings_repo, work_item_repo,
};
use crate::services::{
    agent_service::AgentService, model_service::ModelService, workflow_approval_gate,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

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

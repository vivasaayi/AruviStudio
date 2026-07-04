use super::*;
use crate::commands::{product_commands, test_helpers::make_test_app, work_item_commands};
use crate::domain::work_item::WorkItem;
use crate::persistence::workflow_repo;
use tauri::test::MockRuntime;
use tauri::{Manager, State};

async fn create_approved_work_item(
    state: State<'_, AppState>,
    product_name: &str,
    work_item_title: &str,
) -> WorkItem {
    let product = product_commands::create_product(
        state.clone(),
        product_commands::CreateProductCommand {
            name: product_name.to_string(),
            description: "".to_string(),
            vision: "".to_string(),
            goals: "[]".to_string(),
            tags: "[]".to_string(),
            lifecycle: None,
            health: None,
            owner_label: None,
            investment_status: None,
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("product should be created");
    let product_area = product_commands::hierarchy::create_product_area(
        state.clone(),
        product_commands::CreateProductAreaCommand {
            product_id: product.id.clone(),
            name: "Area".to_string(),
            description: "".to_string(),
            purpose: "".to_string(),
            node_kind: Some("product_area".to_string()),
            explanation: None,
            examples: None,
            implementation_notes: None,
            test_guidance: None,
        },
    )
    .await
    .expect("product_area should be created");
    let work_item = work_item_commands::create_work_item(
        state.clone(),
        work_item_commands::CreateWorkItemCommand {
            product_id: Some(product.id),
            product_area_id: Some(product_area.id),
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title: work_item_title.to_string(),
            problem_statement: "".to_string(),
            description: "".to_string(),
            acceptance_criteria: "".to_string(),
            constraints: "".to_string(),
            work_item_type: "story".to_string(),
            priority: "medium".to_string(),
            complexity: "medium".to_string(),
        },
    )
    .await
    .expect("work item should be created");

    work_item_commands::update_work_item(
        state,
        work_item_commands::UpdateWorkItemCommand {
            id: work_item.id.clone(),
            title: None,
            description: None,
            status: Some("approved".to_string()),
            problem_statement: None,
            acceptance_criteria: None,
            constraints: None,
        },
    )
    .await
    .expect("work item should be approved")
}

#[tokio::test]
async fn workflow_commands_expose_seeded_runs_and_validate_required_ids() {
    let app: tauri::App<MockRuntime> = make_test_app("workflow_commands_seeded").await;
    let state = app.state::<AppState>();

    let work_item =
        create_approved_work_item(state.clone(), "Workflow Seed Product", "Workflow Seed Item")
            .await;
    let seeded = workflow_repo::create_workflow_run(&state.db, "workflow-seeded", &work_item.id)
        .await
        .expect("workflow should be seeded");
    workflow_repo::update_workflow_stage(&state.db, &seeded.id, "pending_plan_approval")
        .await
        .expect("workflow stage should update");

    let workflow_run = get_workflow_run(state.clone(), Some(seeded.id.clone()))
        .await
        .expect("workflow run should load");
    let latest = get_latest_workflow_run_for_work_item(state.clone(), Some(work_item.id.clone()))
        .await
        .expect("latest workflow should load")
        .expect("latest workflow should exist");
    let missing = start_work_item_workflow(state, None)
        .await
        .expect_err("missing work item id should fail");

    assert_eq!(workflow_run.id, seeded.id);
    assert_eq!(workflow_run.current_stage, "pending_plan_approval");
    assert_eq!(latest.id, seeded.id);
    assert!(matches!(missing, AppError::Validation(message) if message == "missing work item id"));
}

#[tokio::test]
async fn handle_invalid_action_and_mark_failed_from_command_layer() {
    let app: tauri::App<MockRuntime> = make_test_app("workflow_commands_fail").await;
    let state = app.state::<AppState>();
    let work_item =
        create_approved_work_item(state.clone(), "Workflow Fail Product", "Workflow Fail Item")
            .await;
    let workflow_run_id =
        workflow_repo::create_workflow_run(&state.db, "workflow-fail-seeded", &work_item.id)
            .await
            .expect("workflow should be seeded")
            .id;
    workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "pending_plan_approval")
        .await
        .expect("workflow stage should update");

    let invalid = handle_workflow_user_action(
        state.clone(),
        Some(workflow_run_id.clone()),
        "unsupported".to_string(),
        None,
    )
    .await
    .expect_err("invalid action should fail");
    assert!(
        matches!(invalid, AppError::Validation(message) if message.contains("Unsupported workflow action"))
    );

    mark_workflow_run_failed(
        state.clone(),
        Some(workflow_run_id.clone()),
        Some("operator fail".to_string()),
    )
    .await
    .expect("workflow should be marked failed");

    let updated = get_workflow_run(state, Some(workflow_run_id.clone()))
        .await
        .expect("workflow run should load");
    assert_eq!(updated.current_stage, "failed");
    assert_eq!(updated.status, "failed");
    assert_eq!(updated.error_message.as_deref(), Some("operator fail"));
}

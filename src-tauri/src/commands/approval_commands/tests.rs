use super::*;
use crate::commands::{product_commands, test_helpers::make_test_app, work_item_commands};
use crate::domain::approval::{ApprovalStatus, ApprovalType};
use crate::domain::work_item::WorkItemStatus;
use crate::persistence::settings_repo;
use crate::state::AppState;
use tauri::test::MockRuntime;
use tauri::{Manager, State};

async fn create_work_item(state: State<'_, AppState>, title: &str) -> String {
    let product = product_commands::create_product(
        state.clone(),
        product_commands::CreateProductCommand {
            name: "Approval Product".to_string(),
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
    let product_area = product_commands::create_product_area(
        state.clone(),
        product_commands::CreateProductAreaCommand {
            product_id: product.id.clone(),
            name: "Approval ProductArea".to_string(),
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
        state,
        work_item_commands::CreateWorkItemCommand {
            product_id: Some(product.id),
            product_area_id: Some(product_area.id),
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title: title.to_string(),
            problem_statement: "Problem".to_string(),
            description: "Description".to_string(),
            acceptance_criteria: "Acceptance".to_string(),
            constraints: "".to_string(),
            work_item_type: "story".to_string(),
            priority: "medium".to_string(),
            complexity: "medium".to_string(),
        },
    )
    .await
    .expect("work item should be created");

    work_item.id
}

#[tokio::test]
async fn approve_work_item_creates_approval_and_updates_status() {
    let app: tauri::App<MockRuntime> = make_test_app("approval_commands_approve").await;
    let state = app.state::<AppState>();
    settings_repo::set_setting(&state.db, AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY, "false")
        .await
        .expect("setting should be stored");

    let work_item_id = create_work_item(state.clone(), "Approval Item").await;
    let approval = approve_work_item(
        state.clone(),
        Some(work_item_id.clone()),
        Some("looks good".to_string()),
    )
    .await
    .expect("approval should succeed");

    let updated = work_item_commands::get_work_item(state.clone(), work_item_id.clone())
        .await
        .expect("work item should load");
    let approvals = get_work_item_approvals(state, Some(work_item_id))
        .await
        .expect("approvals should load");

    assert!(matches!(approval.approval_type, ApprovalType::TaskApproval));
    assert!(matches!(approval.status, ApprovalStatus::Approved));
    assert_eq!(updated.status, WorkItemStatus::Approved);
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].notes, "looks good");
}

#[tokio::test]
async fn rejection_and_secondary_approvals_are_listed_for_work_item() {
    let app: tauri::App<MockRuntime> = make_test_app("approval_commands_reject").await;
    let state = app.state::<AppState>();
    let work_item_id = create_work_item(state.clone(), "Review Item").await;

    work_item_commands::update_work_item(
        state.clone(),
        work_item_commands::UpdateWorkItemCommand {
            id: work_item_id.clone(),
            title: None,
            description: None,
            status: Some("approved".to_string()),
            problem_statement: None,
            acceptance_criteria: None,
            constraints: None,
        },
    )
    .await
    .expect("work item should move to approved");

    reject_work_item(
        state.clone(),
        Some(work_item_id.clone()),
        "needs changes".to_string(),
    )
    .await
    .expect("rejection should succeed");
    approve_work_item_plan(
        state.clone(),
        Some(work_item_id.clone()),
        Some("plan approved".to_string()),
    )
    .await
    .expect("plan approval should succeed");
    approve_work_item_test_review(
        state.clone(),
        Some(work_item_id.clone()),
        Some("tests reviewed".to_string()),
    )
    .await
    .expect("test review should succeed");

    let updated = work_item_commands::get_work_item(state.clone(), work_item_id.clone())
        .await
        .expect("work item should load");
    let approvals = get_work_item_approvals(state, Some(work_item_id))
        .await
        .expect("approvals should load");

    assert_eq!(updated.status, WorkItemStatus::Draft);
    assert_eq!(approvals.len(), 3);
    assert!(approvals.iter().any(|approval| matches!(
        approval.approval_type,
        ApprovalType::TaskApproval
    ) && matches!(
        approval.status,
        ApprovalStatus::Rejected
    ) && approval.notes == "needs changes"));
    assert!(approvals.iter().any(|approval| matches!(
        approval.approval_type,
        ApprovalType::PlanApproval
    ) && matches!(
        approval.status,
        ApprovalStatus::Approved
    )));
    assert!(approvals.iter().any(|approval| matches!(
        approval.approval_type,
        ApprovalType::TestReview
    ) && matches!(
        approval.status,
        ApprovalStatus::Approved
    )));
}

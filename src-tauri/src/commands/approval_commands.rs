use crate::domain::approval::Approval;
use crate::error::AppError;
use crate::persistence::{approval_repo, settings_repo, work_item_repo};
use crate::state::AppState;
use tauri::State;
use tracing::{error, info};

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

fn resolve_work_item_id(
    work_item_id: Option<String>,
    work_item_id_legacy: Option<String>,
) -> Result<String, AppError> {
    work_item_id
        .or(work_item_id_legacy)
        .ok_or_else(|| AppError::Validation("missing work item id".to_string()))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn approve_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    notes: Option<String>,
) -> Result<Approval, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let id = uuid::Uuid::new_v4().to_string();
    info!(work_item_id = %work_item_id, "approve_work_item requested");
    let approval = approval_repo::create_approval(
        &state.db,
        &id,
        &work_item_id,
        None,
        "task_approval",
        "approved",
        &notes.unwrap_or_default(),
    )
    .await?;
    if let Err(error) = work_item_repo::update_work_item(
        &state.db,
        &work_item_id,
        None,
        None,
        Some("approved"),
        None,
        None,
        None,
    )
    .await
    {
        error!(work_item_id = %work_item_id, error = %error, "approve_work_item failed to set work item status");
        return Err(error);
    }
    let auto_start =
        settings_repo::get_bool_setting(&state.db, AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY, true)
            .await?;
    if auto_start {
        let workflow_service = state.workflow_service.clone();
        let work_item_id_for_spawn = work_item_id.clone();
        tokio::spawn(async move {
            let workflow_service = workflow_service.lock().await;
            if let Err(error) = workflow_service
                .start_work_item_workflow(&work_item_id_for_spawn)
                .await
            {
                error!(
                    work_item_id = %work_item_id_for_spawn,
                    error = %error,
                    "auto-start after work item approval failed"
                );
            }
        });
    }
    info!(work_item_id = %work_item_id, "approve_work_item completed");
    Ok(approval)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn reject_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    notes: String,
) -> Result<Approval, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let id = uuid::Uuid::new_v4().to_string();
    info!(work_item_id = %work_item_id, "reject_work_item requested");
    let approval = approval_repo::create_approval(
        &state.db,
        &id,
        &work_item_id,
        None,
        "task_approval",
        "rejected",
        &notes,
    )
    .await?;
    if let Err(error) = work_item_repo::update_work_item(
        &state.db,
        &work_item_id,
        None,
        None,
        Some("draft"),
        None,
        None,
        None,
    )
    .await
    {
        error!(work_item_id = %work_item_id, error = %error, "reject_work_item failed to reset work item status");
        return Err(error);
    }
    info!(work_item_id = %work_item_id, "reject_work_item completed");
    Ok(approval)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn approve_work_item_plan(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    notes: Option<String>,
) -> Result<Approval, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let id = uuid::Uuid::new_v4().to_string();
    approval_repo::create_approval(
        &state.db,
        &id,
        &work_item_id,
        None,
        "plan_approval",
        "approved",
        &notes.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn reject_work_item_plan(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    notes: String,
) -> Result<Approval, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let id = uuid::Uuid::new_v4().to_string();
    approval_repo::create_approval(
        &state.db,
        &id,
        &work_item_id,
        None,
        "plan_approval",
        "rejected",
        &notes,
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn approve_work_item_test_review(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
    notes: Option<String>,
) -> Result<Approval, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let id = uuid::Uuid::new_v4().to_string();
    approval_repo::create_approval(
        &state.db,
        &id,
        &work_item_id,
        None,
        "test_review",
        "approved",
        &notes.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_work_item_approvals(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
) -> Result<Vec<Approval>, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    approval_repo::list_approvals(&state.db, &work_item_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{product_commands, test_helpers::make_test_app, work_item_commands};
    use crate::domain::approval::{ApprovalStatus, ApprovalType};
    use crate::domain::work_item::WorkItemStatus;
    use crate::persistence::settings_repo;
    use crate::state::AppState;
    use tauri::Manager;
    use tauri::test::MockRuntime;

    async fn create_work_item(state: State<'_, AppState>, title: &str) -> String {
        let product = product_commands::create_product(
            state.clone(),
            "Approval Product".to_string(),
            "".to_string(),
            "".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("product should be created");
        let module = product_commands::create_module(
            state.clone(),
            product.id.clone(),
            "Approval Module".to_string(),
            "".to_string(),
            "".to_string(),
            Some("area".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("module should be created");
        let work_item = work_item_commands::create_work_item(
            state,
            Some(product.id),
            None,
            Some(module.id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            title.to_string(),
            "Problem".to_string(),
            None,
            "Description".to_string(),
            "Acceptance".to_string(),
            None,
            "".to_string(),
            "story".to_string(),
            None,
            "medium".to_string(),
            "medium".to_string(),
        )
        .await
        .expect("work item should be created");

        work_item.id
    }

    #[tokio::test]
    async fn approve_work_item_creates_approval_and_updates_status() {
        let app: tauri::App<MockRuntime> = make_test_app("approval_commands_approve").await;
        let state = app.state::<AppState>();
        settings_repo::set_setting(
            &state.db,
            AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY,
            "false",
        )
        .await
        .expect("setting should be stored");

        let work_item_id = create_work_item(state.clone(), "Approval Item").await;
        let approval = approve_work_item(
            state.clone(),
            None,
            Some(work_item_id.clone()),
            Some("looks good".to_string()),
        )
        .await
        .expect("approval should succeed");

        let updated = work_item_commands::get_work_item(state.clone(), work_item_id.clone())
            .await
            .expect("work item should load");
        let approvals = get_work_item_approvals(state, Some(work_item_id), None)
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
            work_item_id.clone(),
            None,
            None,
            Some("approved".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("work item should move to approved");

        reject_work_item(
            state.clone(),
            Some(work_item_id.clone()),
            None,
            "needs changes".to_string(),
        )
        .await
        .expect("rejection should succeed");
        approve_work_item_plan(
            state.clone(),
            Some(work_item_id.clone()),
            None,
            Some("plan approved".to_string()),
        )
        .await
        .expect("plan approval should succeed");
        approve_work_item_test_review(
            state.clone(),
            None,
            Some(work_item_id.clone()),
            Some("tests reviewed".to_string()),
        )
        .await
        .expect("test review should succeed");

        let updated = work_item_commands::get_work_item(state.clone(), work_item_id.clone())
            .await
            .expect("work item should load");
        let approvals = get_work_item_approvals(state, None, Some(work_item_id))
            .await
            .expect("approvals should load");

        assert_eq!(updated.status, WorkItemStatus::Draft);
        assert_eq!(approvals.len(), 3);
        assert!(
            approvals
                .iter()
                .any(|approval| matches!(approval.approval_type, ApprovalType::TaskApproval)
                    && matches!(approval.status, ApprovalStatus::Rejected)
                    && approval.notes == "needs changes")
        );
        assert!(
            approvals
                .iter()
                .any(|approval| matches!(approval.approval_type, ApprovalType::PlanApproval)
                    && matches!(approval.status, ApprovalStatus::Approved))
        );
        assert!(
            approvals
                .iter()
                .any(|approval| matches!(approval.approval_type, ApprovalType::TestReview)
                    && matches!(approval.status, ApprovalStatus::Approved))
        );
    }
}

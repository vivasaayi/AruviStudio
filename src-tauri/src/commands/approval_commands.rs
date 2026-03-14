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

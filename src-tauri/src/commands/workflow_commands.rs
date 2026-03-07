use crate::domain::agent::AgentRun;
use crate::domain::workflow::{UserAction, WorkflowRun, WorkflowStageHistory};
use crate::error::AppError;
use crate::persistence::{agent_repo, workflow_repo};
use crate::state::AppState;
use tauri::State;

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
pub async fn start_work_item_workflow(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
) -> Result<String, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    let workflow_service = state.workflow_service.lock().await;
    let workflow_run = workflow_service
        .start_work_item_workflow(&work_item_id)
        .await?;
    Ok(workflow_run.id)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_workflow_run(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
) -> Result<WorkflowRun, AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service.get_workflow_run(&workflow_run_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_latest_workflow_run_for_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
    workItemId: Option<String>,
) -> Result<Option<WorkflowRun>, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id, workItemId)?;
    workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_workflow_history(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
) -> Result<Vec<WorkflowStageHistory>, AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service
        .get_workflow_history(&workflow_run_id)
        .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn handle_workflow_user_action(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
    action: String,
    notes: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let parsed_action = match action.as_str() {
        "approve" => UserAction::Approve,
        "reject" => UserAction::Reject,
        "pause" => UserAction::Pause,
        "resume" => UserAction::Resume,
        "cancel" => UserAction::Cancel,
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported workflow action: {}",
                action
            )))
        }
    };
    let workflow_service = state.workflow_service.lock().await;
    workflow_service
        .handle_user_action(&workflow_run_id, parsed_action, notes)
        .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn advance_workflow(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service.advance_workflow(&workflow_run_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_agent_runs_for_workflow(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
) -> Result<Vec<AgentRun>, AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    agent_repo::list_agent_runs_for_workflow(&state.db, &workflow_run_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn mark_workflow_run_failed(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
    reason: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
    if run.current_stage != "failed" {
        workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "failed").await?;
        let transition_id = uuid::Uuid::new_v4().to_string();
        workflow_repo::record_stage_transition(
            &state.db,
            &transition_id,
            &workflow_run_id,
            &run.current_stage,
            "failed",
            "user_override",
            reason
                .as_deref()
                .unwrap_or("Marked failed by operator from UI"),
        )
        .await?;
    }
    workflow_repo::update_workflow_lifecycle(
        &state.db,
        &workflow_run_id,
        "failed",
        Some(
            reason
                .as_deref()
                .unwrap_or("Marked failed by operator from UI"),
        ),
        true,
    )
    .await?;
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn restart_workflow_run(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    workflowRunId: Option<String>,
) -> Result<String, AppError> {
    let workflow_run_id = workflow_run_id
        .or(workflowRunId)
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
    let workflow_service = state.workflow_service.lock().await;
    let next = workflow_service
        .start_work_item_workflow(&run.work_item_id)
        .await?;
    Ok(next.id)
}

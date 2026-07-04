use crate::error::AppError;
use crate::services::planner_service::{
    add_planner_draft_child, analyze_repository_for_planner, clear_planner_pending,
    confirm_planner_plan, create_planner_session, delete_planner_draft_node,
    rename_planner_draft_node, submit_planner_turn, submit_planner_voice_turn,
    update_planner_session, PlannerSessionInfo, PlannerTurnResponse,
};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_planner_session_command(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    create_planner_session(
        state.planner_service.clone(),
        &state.db,
        provider_id,
        model_name,
    )
    .await
}

#[tauri::command]
pub async fn update_planner_session_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    update_planner_session(
        state.planner_service.clone(),
        &state.db,
        session_id,
        provider_id,
        model_name,
    )
    .await
}

#[tauri::command]
pub async fn clear_planner_pending_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    clear_planner_pending(state.planner_service.clone(), &state.db, session_id).await
}

#[tauri::command]
pub async fn submit_planner_turn_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    user_input: Option<String>,
    selected_draft_node_id: Option<String>,
    product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let user_input =
        user_input.ok_or_else(|| AppError::Validation("missing planner user input".to_string()))?;
    submit_planner_turn(
        state.planner_service.clone(),
        &state,
        session_id,
        user_input,
        selected_draft_node_id,
        product_id,
    )
    .await
}

#[tauri::command]
pub async fn submit_planner_voice_turn_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    transcript: Option<String>,
    user_input: Option<String>,
    selected_draft_node_id: Option<String>,
    product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let transcript = transcript
        .or(user_input)
        .ok_or_else(|| AppError::Validation("missing planner voice transcript".to_string()))?;
    submit_planner_voice_turn(
        state.planner_service.clone(),
        &state,
        session_id,
        transcript,
        selected_draft_node_id,
        product_id,
    )
    .await
}

#[tauri::command]
pub async fn confirm_planner_plan_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    confirm_planner_plan(state.planner_service.clone(), &state, session_id).await
}

#[tauri::command]
pub async fn rename_planner_draft_node_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    node_id: Option<String>,
    name: String,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let node_id =
        node_id.ok_or_else(|| AppError::Validation("missing draft node id".to_string()))?;
    rename_planner_draft_node(
        state.planner_service.clone(),
        &state.db,
        session_id,
        node_id,
        name,
    )
    .await
}

#[tauri::command]
pub async fn add_planner_draft_child_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    parent_node_id: Option<String>,
    child_type: Option<String>,
    name: String,
    summary: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let parent_node_id = parent_node_id
        .ok_or_else(|| AppError::Validation("missing parent draft node id".to_string()))?;
    let child_type =
        child_type.ok_or_else(|| AppError::Validation("missing draft child type".to_string()))?;
    add_planner_draft_child(
        state.planner_service.clone(),
        &state.db,
        session_id,
        parent_node_id,
        child_type,
        name,
        summary,
    )
    .await
}

#[tauri::command]
pub async fn delete_planner_draft_node_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    node_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let node_id =
        node_id.ok_or_else(|| AppError::Validation("missing draft node id".to_string()))?;
    delete_planner_draft_node(
        state.planner_service.clone(),
        &state.db,
        session_id,
        node_id,
    )
    .await
}

#[tauri::command]
pub async fn analyze_repository_for_planner_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    repository_id: Option<String>,
    selected_draft_node_id: Option<String>,
    product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id =
        session_id.ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let repository_id =
        repository_id.ok_or_else(|| AppError::Validation("missing repository id".to_string()))?;
    analyze_repository_for_planner(
        state.planner_service.clone(),
        &state.db,
        &state.artifact_base_path,
        session_id,
        repository_id,
        selected_draft_node_id,
        product_id,
    )
    .await
}

#[cfg(test)]
mod tests;

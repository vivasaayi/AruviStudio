use crate::error::AppError;
use crate::services::planner_service::{
    add_planner_draft_child, clear_planner_pending, confirm_planner_plan,
    create_planner_session, delete_planner_draft_node, rename_planner_draft_node,
    submit_planner_turn, update_planner_session, PlannerSessionInfo, PlannerTurnResponse,
};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
#[allow(non_snake_case)]
pub async fn create_planner_session_command(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    providerId: Option<String>,
    model_name: Option<String>,
    modelName: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    create_planner_session(
        state.planner_service.clone(),
        &state.db,
        provider_id.or(providerId),
        model_name.or(modelName),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_planner_session_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    provider_id: Option<String>,
    providerId: Option<String>,
    model_name: Option<String>,
    modelName: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    update_planner_session(
        state.planner_service.clone(),
        &state.db,
        session_id,
        provider_id.or(providerId),
        model_name.or(modelName),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn clear_planner_pending_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    clear_planner_pending(state.planner_service.clone(), &state.db, session_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn submit_planner_turn_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    user_input: Option<String>,
    userInput: Option<String>,
    selected_draft_node_id: Option<String>,
    selectedDraftNodeId: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let user_input = user_input
        .or(userInput)
        .ok_or_else(|| AppError::Validation("missing planner user input".to_string()))?;
    submit_planner_turn(
        state.planner_service.clone(),
        &state,
        session_id,
        user_input,
        selected_draft_node_id.or(selectedDraftNodeId),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn confirm_planner_plan_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    confirm_planner_plan(state.planner_service.clone(), &state, session_id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn rename_planner_draft_node_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    node_id: Option<String>,
    nodeId: Option<String>,
    name: String,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let node_id = node_id
        .or(nodeId)
        .ok_or_else(|| AppError::Validation("missing draft node id".to_string()))?;
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
#[allow(non_snake_case)]
pub async fn add_planner_draft_child_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    parent_node_id: Option<String>,
    parentNodeId: Option<String>,
    child_type: Option<String>,
    childType: Option<String>,
    name: String,
    summary: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let parent_node_id = parent_node_id
        .or(parentNodeId)
        .ok_or_else(|| AppError::Validation("missing parent draft node id".to_string()))?;
    let child_type = child_type
        .or(childType)
        .ok_or_else(|| AppError::Validation("missing draft child type".to_string()))?;
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
#[allow(non_snake_case)]
pub async fn delete_planner_draft_node_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    node_id: Option<String>,
    nodeId: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let node_id = node_id
        .or(nodeId)
        .ok_or_else(|| AppError::Validation("missing draft node id".to_string()))?;
    delete_planner_draft_node(
        state.planner_service.clone(),
        &state.db,
        session_id,
        node_id,
    )
    .await
}

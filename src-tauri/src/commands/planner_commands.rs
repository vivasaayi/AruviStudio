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
    product_id: Option<String>,
    productId: Option<String>,
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
        product_id.or(productId),
    )
    .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn submit_planner_voice_turn_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    transcript: Option<String>,
    user_input: Option<String>,
    userInput: Option<String>,
    selected_draft_node_id: Option<String>,
    selectedDraftNodeId: Option<String>,
    product_id: Option<String>,
    productId: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let transcript = transcript
        .or(user_input)
        .or(userInput)
        .ok_or_else(|| AppError::Validation("missing planner voice transcript".to_string()))?;
    submit_planner_voice_turn(
        state.planner_service.clone(),
        &state,
        session_id,
        transcript,
        selected_draft_node_id.or(selectedDraftNodeId),
        product_id.or(productId),
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

#[tauri::command]
#[allow(non_snake_case)]
pub async fn analyze_repository_for_planner_command(
    state: State<'_, AppState>,
    session_id: Option<String>,
    sessionId: Option<String>,
    repository_id: Option<String>,
    repositoryId: Option<String>,
    selected_draft_node_id: Option<String>,
    selectedDraftNodeId: Option<String>,
    product_id: Option<String>,
    productId: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let session_id = session_id
        .or(sessionId)
        .ok_or_else(|| AppError::Validation("missing planner session id".to_string()))?;
    let repository_id = repository_id
        .or(repositoryId)
        .ok_or_else(|| AppError::Validation("missing repository id".to_string()))?;
    analyze_repository_for_planner(
        state.planner_service.clone(),
        &state.db,
        &state.artifact_base_path,
        session_id,
        repository_id,
        selected_draft_node_id.or(selectedDraftNodeId),
        product_id.or(productId),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_helpers::make_test_app;
    use tauri::test::MockRuntime;
    use tauri::Manager;

    #[tokio::test]
    async fn planner_commands_validate_required_session_ids() {
        let app: tauri::App<MockRuntime> = make_test_app("planner_commands_validation").await;
        let state = app.state::<AppState>();

        let update =
            update_planner_session_command(state.clone(), None, None, None, None, None, None)
                .await
                .expect_err("update should require session id");
        let clear = clear_planner_pending_command(state.clone(), None, None)
            .await
            .expect_err("clear should require session id");
        let confirm = confirm_planner_plan_command(state.clone(), None, None)
            .await
            .expect_err("confirm should require session id");
        let rename = rename_planner_draft_node_command(
            state.clone(),
            None,
            None,
            Some("node-1".to_string()),
            None,
            "Renamed".to_string(),
        )
        .await
        .expect_err("rename should require session id");
        let delete =
            delete_planner_draft_node_command(state, None, None, Some("node-1".to_string()), None)
                .await
                .expect_err("delete should require session id");

        assert!(matches!(
            update,
            AppError::Validation(message) if message == "missing planner session id"
        ));
        assert!(matches!(
            clear,
            AppError::Validation(message) if message == "missing planner session id"
        ));
        assert!(matches!(
            confirm,
            AppError::Validation(message) if message == "missing planner session id"
        ));
        assert!(matches!(
            rename,
            AppError::Validation(message) if message == "missing planner session id"
        ));
        assert!(matches!(
            delete,
            AppError::Validation(message) if message == "missing planner session id"
        ));
    }

    #[tokio::test]
    async fn planner_commands_validate_other_required_fields_and_aliases() {
        let app: tauri::App<MockRuntime> = make_test_app("planner_commands_required_fields").await;
        let state = app.state::<AppState>();

        let submit = submit_planner_turn_command(
            state.clone(),
            Some("session-1".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("submit should require user input");
        let voice = submit_planner_voice_turn_command(
            state.clone(),
            None,
            Some("session-1".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("voice submit should require transcript");
        let add_child = add_planner_draft_child_command(
            state.clone(),
            Some("session-1".to_string()),
            None,
            None,
            None,
            Some("product_area".to_string()),
            None,
            "Child".to_string(),
            None,
        )
        .await
        .expect_err("add child should require parent node id");
        let analyze = analyze_repository_for_planner_command(
            state.clone(),
            None,
            Some("session-1".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("analyze should require repository id");
        let rename = rename_planner_draft_node_command(
            state,
            None,
            Some("session-1".to_string()),
            None,
            None,
            "Renamed".to_string(),
        )
        .await
        .expect_err("rename should require node id");

        assert!(matches!(
            submit,
            AppError::Validation(message) if message == "missing planner user input"
        ));
        assert!(matches!(
            voice,
            AppError::Validation(message) if message == "missing planner voice transcript"
        ));
        assert!(matches!(
            add_child,
            AppError::Validation(message) if message == "missing parent draft node id"
        ));
        assert!(matches!(
            analyze,
            AppError::Validation(message) if message == "missing repository id"
        ));
        assert!(matches!(
            rename,
            AppError::Validation(message) if message == "missing draft node id"
        ));
    }
}

use super::*;
use crate::commands::test_helpers::make_test_app;
use tauri::test::MockRuntime;
use tauri::Manager;

#[tokio::test]
async fn planner_commands_validate_required_session_ids() {
    let app: tauri::App<MockRuntime> = make_test_app("planner_commands_validation").await;
    let state = app.state::<AppState>();

    let update = update_planner_session_command(state.clone(), None, None, None)
        .await
        .expect_err("update should require session id");
    let clear = clear_planner_pending_command(state.clone(), None)
        .await
        .expect_err("clear should require session id");
    let confirm = confirm_planner_plan_command(state.clone(), None)
        .await
        .expect_err("confirm should require session id");
    let rename = rename_planner_draft_node_command(
        state.clone(),
        None,
        Some("node-1".to_string()),
        "Renamed".to_string(),
    )
    .await
    .expect_err("rename should require session id");
    let delete = delete_planner_draft_node_command(state, None, Some("node-1".to_string()))
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
async fn planner_commands_validate_other_required_fields() {
    let app: tauri::App<MockRuntime> = make_test_app("planner_commands_required_fields").await;
    let state = app.state::<AppState>();

    let submit = submit_planner_turn_command(
        state.clone(),
        Some("session-1".to_string()),
        None,
        None,
        None,
    )
    .await
    .expect_err("submit should require user input");
    let voice = submit_planner_voice_turn_command(
        state.clone(),
        Some("session-1".to_string()),
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
        Some("product_area".to_string()),
        "Child".to_string(),
        None,
    )
    .await
    .expect_err("add child should require parent node id");
    let analyze = analyze_repository_for_planner_command(
        state.clone(),
        Some("session-1".to_string()),
        None,
        None,
        None,
    )
    .await
    .expect_err("analyze should require repository id");
    let rename = rename_planner_draft_node_command(
        state,
        Some("session-1".to_string()),
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

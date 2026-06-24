use super::*;
use crate::commands::test_helpers::make_test_app;
use tauri::test::MockRuntime;
use tauri::Manager;

#[tokio::test]
async fn settings_commands_round_trip_settings_and_db_metadata() {
    let app: tauri::App<MockRuntime> = make_test_app("settings_commands_round_trip").await;
    let state = app.state::<AppState>();

    set_setting(
        state.clone(),
        "coverage.test.key".to_string(),
        "enabled".to_string(),
    )
    .await
    .expect("setting should be stored");
    let stored = get_setting(state.clone(), "coverage.test.key".to_string())
        .await
        .expect("setting should load");
    let active_db_path = get_active_database_path(state.clone())
        .await
        .expect("db path should resolve");
    let health = get_database_health(state)
        .await
        .expect("db health should resolve");

    assert_eq!(stored.as_deref(), Some("enabled"));
    assert!(active_db_path.ends_with(".db"));
    assert!(health.applied_migrations > 0);
    assert_eq!(
        health.latest_version,
        health.migrations.last().map(|item| item.version)
    );
}

#[tokio::test]
async fn database_override_commands_validate_and_persist_absolute_paths() {
    let app: tauri::App<MockRuntime> = make_test_app("settings_commands_override").await;
    let state = app.state::<AppState>();

    let empty = set_database_path_override(state.clone(), "   ".to_string())
        .await
        .expect_err("empty path should fail");
    let relative = set_database_path_override(state.clone(), "relative/path.db".to_string())
        .await
        .expect_err("relative path should fail");

    let override_path = state.app_data_dir.join("custom.db");
    set_database_path_override(state.clone(), override_path.to_string_lossy().to_string())
        .await
        .expect("absolute path should persist");
    let stored = get_database_path_override(state.clone())
        .await
        .expect("override should load");

    clear_database_path_override(state.clone())
        .await
        .expect("override should clear");
    let cleared = get_database_path_override(state.clone())
        .await
        .expect("cleared override should load");
    clear_database_path_override(state)
        .await
        .expect("clearing missing override should be harmless");

    assert!(matches!(
        empty,
        AppError::Validation(message) if message == "Database path cannot be empty"
    ));
    assert!(matches!(
        relative,
        AppError::Validation(message) if message == "Database path must be an absolute path"
    ));
    assert_eq!(
        stored.as_deref(),
        Some(override_path.to_string_lossy().as_ref())
    );
    assert_eq!(cleared, None);
}

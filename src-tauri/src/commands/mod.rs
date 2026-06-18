pub mod agent_commands;
pub mod approval_commands;
pub mod artifact_commands;
pub mod channel_commands;
pub mod external_cli_commands;
pub mod finding_commands;
pub mod model_commands;
pub mod observability_commands;
pub mod planner_commands;
pub mod product_commands;
pub mod repository_commands;
pub mod settings_commands;
pub mod speech_commands;
pub mod strategy_commands;
pub mod work_item_commands;
pub mod workflow_commands;

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::persistence::db as db_service;
    use crate::state::AppState;
    use std::path::PathBuf;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    fn make_temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aruvi_command_tests_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    pub(crate) async fn make_test_app(name: &str) -> tauri::App<MockRuntime> {
        let temp_root = make_temp_dir(name);
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");
        let state = AppState::new(pool, temp_root)
            .await
            .expect("failed to create app state");

        mock_builder()
            .manage(state)
            .build(mock_context(noop_assets()))
            .expect("failed to build tauri test app")
    }
}

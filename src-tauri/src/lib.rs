mod commands;
mod domain;
mod error;
mod execution;
mod observability;
mod persistence;
mod providers;
mod secrets;
mod services;
mod state;
mod workflows;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    observability::logger::init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            let state = rt.block_on(async {
                let proj_dirs = directories::ProjectDirs::from("com", "aruvi", "studio")
                    .expect("Failed to get project directories");
                let data_dir = proj_dirs.data_dir();
                std::fs::create_dir_all(data_dir).expect("Failed to create data directory");

                let db_override_path = data_dir.join("db_override_path.txt");
                let db_path = std::env::var("ARUVI_DB_PATH")
                    .ok()
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::fs::read_to_string(&db_override_path)
                            .ok()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty())
                            .map(std::path::PathBuf::from)
                    })
                    .unwrap_or_else(|| data_dir.join("aruvi_studio.db"));

                if let Some(parent) = db_path.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("Failed to create parent directory for database path");
                }
                let db_url = format!("sqlite:{}", db_path.display());

                let pool = persistence::db::create_pool(&db_url)
                    .await
                    .expect("Failed to create database pool");

                AppState::new(pool, data_dir.to_path_buf())
                    .await
                    .expect("Failed to create app state")
            });
            let webhook_state = state.clone();
            app.manage(state);
            tauri::async_runtime::spawn(async move {
                services::webhook_service::start_webhook_server(webhook_state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Product commands
            commands::product_commands::create_product,
            commands::product_commands::get_product,
            commands::product_commands::list_products,
            commands::product_commands::update_product,
            commands::product_commands::archive_product,
            commands::product_commands::seed_example_products,
            commands::product_commands::create_module,
            commands::product_commands::list_modules,
            commands::product_commands::update_module,
            commands::product_commands::delete_module,
            commands::product_commands::reorder_modules,
            commands::product_commands::create_capability,
            commands::product_commands::list_capabilities,
            commands::product_commands::update_capability,
            commands::product_commands::delete_capability,
            commands::product_commands::reorder_capabilities,
            commands::product_commands::get_product_tree,
            // Work item commands
            commands::work_item_commands::create_work_item,
            commands::work_item_commands::get_work_item,
            commands::work_item_commands::list_work_items,
            commands::work_item_commands::summarize_work_items_by_product,
            commands::work_item_commands::update_work_item,
            commands::work_item_commands::delete_work_item,
            commands::work_item_commands::get_sub_work_items,
            commands::work_item_commands::reorder_work_items,
            // Repository commands
            commands::repository_commands::register_repository,
            commands::repository_commands::list_repositories,
            commands::repository_commands::delete_repository,
            commands::repository_commands::attach_repository,
            commands::repository_commands::resolve_repository_for_work_item,
            commands::repository_commands::resolve_repository_for_scope,
            commands::repository_commands::create_local_workspace,
            commands::repository_commands::browse_for_repository_path,
            commands::repository_commands::reveal_in_finder,
            commands::repository_commands::list_repository_tree,
            commands::repository_commands::read_repository_file,
            commands::repository_commands::write_repository_file,
            commands::repository_commands::get_repository_file_sha256,
            commands::repository_commands::apply_repository_patch,
            // Approval commands
            commands::approval_commands::approve_work_item,
            commands::approval_commands::reject_work_item,
            commands::approval_commands::approve_work_item_plan,
            commands::approval_commands::reject_work_item_plan,
            commands::approval_commands::approve_work_item_test_review,
            commands::approval_commands::get_work_item_approvals,
            // Workflow commands
            commands::workflow_commands::start_work_item_workflow,
            commands::workflow_commands::get_workflow_run,
            commands::workflow_commands::get_latest_workflow_run_for_work_item,
            commands::workflow_commands::get_workflow_history,
            commands::workflow_commands::handle_workflow_user_action,
            commands::workflow_commands::advance_workflow,
            commands::workflow_commands::list_agent_runs_for_workflow,
            commands::workflow_commands::mark_workflow_run_failed,
            commands::workflow_commands::restart_workflow_run,
            // Agent commands
            commands::agent_commands::list_agent_definitions,
            commands::agent_commands::list_agent_model_bindings,
            commands::agent_commands::set_primary_agent_model_binding,
            commands::agent_commands::create_agent_definition,
            commands::agent_commands::update_agent_definition,
            commands::agent_commands::delete_agent_definition,
            commands::agent_commands::list_agent_teams,
            commands::agent_commands::create_agent_team,
            commands::agent_commands::update_agent_team,
            commands::agent_commands::delete_agent_team,
            commands::agent_commands::list_team_memberships,
            commands::agent_commands::add_team_member,
            commands::agent_commands::remove_team_member,
            commands::agent_commands::list_team_assignments,
            commands::agent_commands::assign_team_scope,
            commands::agent_commands::remove_team_assignment,
            commands::agent_commands::list_skills,
            commands::agent_commands::create_skill,
            commands::agent_commands::update_skill,
            commands::agent_commands::delete_skill,
            commands::agent_commands::list_agent_skill_links,
            commands::agent_commands::link_skill_to_agent,
            commands::agent_commands::unlink_skill_from_agent,
            commands::agent_commands::list_team_skill_links,
            commands::agent_commands::link_skill_to_team,
            commands::agent_commands::unlink_skill_from_team,
            commands::agent_commands::list_workflow_stage_policies,
            commands::agent_commands::upsert_workflow_stage_policy,
            commands::agent_commands::delete_workflow_stage_policy,
            // Model commands
            commands::model_commands::create_provider,
            commands::model_commands::list_providers,
            commands::model_commands::update_provider,
            commands::model_commands::delete_provider,
            commands::model_commands::create_model_definition,
            commands::model_commands::list_model_definitions,
            commands::model_commands::update_model_definition,
            commands::model_commands::delete_model_definition,
            commands::model_commands::test_provider_connectivity,
            commands::model_commands::browse_for_local_model_file,
            commands::model_commands::register_local_runtime_model_command,
            commands::model_commands::install_managed_local_model_command,
            commands::model_commands::run_model_chat_completion,
            commands::model_commands::start_model_chat_stream,
            // Planner commands
            commands::planner_commands::create_planner_session_command,
            commands::planner_commands::update_planner_session_command,
            commands::planner_commands::clear_planner_pending_command,
            commands::planner_commands::submit_planner_turn_command,
            commands::planner_commands::submit_planner_voice_turn_command,
            commands::planner_commands::confirm_planner_plan_command,
            commands::planner_commands::rename_planner_draft_node_command,
            commands::planner_commands::add_planner_draft_child_command,
            commands::planner_commands::delete_planner_draft_node_command,
            commands::planner_commands::analyze_repository_for_planner_command,
            // Channel commands
            commands::channel_commands::send_twilio_whatsapp_message,
            commands::channel_commands::start_twilio_voice_call,
            commands::channel_commands::route_planner_contact_command,
            // Artifact commands
            commands::artifact_commands::list_work_item_artifacts,
            commands::artifact_commands::read_artifact_content,
            // Finding commands
            commands::finding_commands::list_work_item_findings,
            // Settings commands
            commands::settings_commands::get_setting,
            commands::settings_commands::set_setting,
            commands::settings_commands::get_mobile_bridge_status,
            commands::settings_commands::get_database_health,
            commands::settings_commands::get_active_database_path,
            commands::settings_commands::get_database_path_override,
            commands::settings_commands::set_database_path_override,
            commands::settings_commands::clear_database_path_override,
            // Speech commands
            commands::speech_commands::transcribe_audio_command,
            commands::speech_commands::speak_text_natively_command,
            // Observability commands
            commands::observability_commands::get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

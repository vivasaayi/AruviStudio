mod app_paths;
mod bootstrap;
mod commands;
mod domain;
mod error;
mod execution;
mod mcp;
mod observability;
mod persistence;
mod planning_doctrine;
mod providers;
mod secrets;
mod services;
mod state;
mod workflows;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    observability::logger::init_logging();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_identifier = app.config().identifier.clone();
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    eprintln!("failed to create Tokio runtime: {err}");
                    std::process::exit(1);
                }
            };
            let state = match rt.block_on(async {
                bootstrap::initialize_app_state(Some(app_identifier.as_str())).await
            }) {
                Ok(state) => state,
                Err(err) => {
                    eprintln!("failed to create app state: {err}");
                    std::process::exit(1);
                }
            };
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
            commands::product_commands::reset_product_plan,
            commands::product_commands::seed_example_products,
            commands::product_commands::hierarchy::create_product_area,
            commands::product_commands::hierarchy::list_product_areas,
            commands::product_commands::hierarchy::update_product_area,
            commands::product_commands::hierarchy::delete_product_area,
            commands::product_commands::hierarchy::reorder_product_areas,
            commands::product_commands::hierarchy::create_capability,
            commands::product_commands::hierarchy::list_capabilities,
            commands::product_commands::hierarchy::list_product_capabilities,
            commands::product_commands::hierarchy::get_capability,
            commands::product_commands::hierarchy::update_capability,
            commands::product_commands::hierarchy::delete_capability,
            commands::product_commands::hierarchy::reorder_capabilities,
            commands::product_commands::hierarchy::apply_semantic_template,
            commands::product_commands::hierarchy::convert_capability_kind,
            commands::product_commands::hierarchy::get_product_tree,
            commands::product_commands::hierarchy::summarize_product_tree,
            commands::product_commands::bulk_import::get_bulk_import_schema,
            commands::product_commands::bulk_import::submit_bulk_import,
            commands::product_commands::bulk_import::get_bulk_import_status,
            commands::product_commands::bulk_import::list_bulk_import_jobs,
            commands::product_commands::list_product_references,
            commands::product_commands::create_product_reference,
            commands::product_commands::delete_product_reference,
            // Strategy and portfolio commands
            commands::strategy_commands::list_strategy_nodes,
            commands::strategy_commands::create_strategy_node,
            commands::strategy_commands::update_strategy_node,
            commands::strategy_commands::delete_strategy_node,
            commands::strategy_commands::list_product_strategy_links,
            commands::strategy_commands::link_product_to_strategy,
            commands::strategy_commands::unlink_product_from_strategy,
            commands::strategy_commands::list_product_dependencies,
            commands::strategy_commands::create_product_dependency,
            commands::strategy_commands::delete_product_dependency,
            // Work item commands
            commands::work_item_commands::create_work_item,
            commands::work_item_commands::get_work_item,
            commands::work_item_commands::list_work_items,
            commands::work_item_commands::list_work_items_page,
            commands::work_item_commands::summarize_work_items_by_product,
            commands::work_item_commands::summarize_work_items_by_scope,
            commands::work_item_commands::update_work_item,
            commands::work_item_commands::assign_work_item_workspace,
            commands::work_item_commands::delete_work_item,
            commands::work_item_commands::get_sub_work_items,
            commands::work_item_commands::reorder_work_items,
            // Repository commands
            commands::repository_commands::register_repository,
            commands::repository_commands::update_repository,
            commands::repository_commands::list_repositories,
            commands::repository_commands::delete_repository,
            commands::repository_commands::attach_repository,
            commands::repository_commands::resolve_repository_for_work_item,
            commands::repository_commands::resolve_repository_for_scope,
            commands::repository_commands::workspace::create_local_workspace,
            commands::repository_commands::browse_for_repository_path,
            commands::repository_commands::reveal_in_finder,
            commands::repository_commands::exports::export_product_overview_html,
            commands::repository_commands::exports::export_product_overview_epub,
            commands::repository_commands::exports::export_product_overview_pdf,
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
            commands::workflow_commands::list_agent_model_calls_for_workflow,
            commands::workflow_commands::mark_workflow_run_failed,
            commands::workflow_commands::restart_workflow_run,
            // External CLI commands
            commands::external_cli_commands::invoke_external_cli_for_work_item,
            commands::external_cli_commands::list_external_cli_run_events,
            commands::external_cli_commands::list_external_cli_runs_for_work_item,
            // Agent commands
            commands::agent_commands::list_agent_definitions,
            commands::agent_commands::list_agent_model_bindings,
            commands::agent_commands::set_primary_agent_model_binding,
            commands::agent_commands::create_agent_definition,
            commands::agent_commands::update_agent_definition,
            commands::agent_commands::delete_agent_definition,
            commands::agent_commands::team::list_agent_teams,
            commands::agent_commands::team::create_agent_team,
            commands::agent_commands::team::update_agent_team,
            commands::agent_commands::team::delete_agent_team,
            commands::agent_commands::team::list_team_memberships,
            commands::agent_commands::team::add_team_member,
            commands::agent_commands::team::remove_team_member,
            commands::agent_commands::team::list_team_assignments,
            commands::agent_commands::team::assign_team_scope,
            commands::agent_commands::team::remove_team_assignment,
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
            commands::model_commands::local_runtime::browse_for_local_model_file,
            commands::model_commands::local_runtime::register_local_runtime_model_command,
            commands::model_commands::local_runtime::install_managed_local_model_command,
            commands::model_commands::run_model_chat_completion,
            commands::model_commands::list_model_calls,
            commands::model_commands::get_model_call,
            commands::model_commands::read_model_call_snapshot,
            commands::model_commands::stream::start_model_chat_stream,
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
            commands::settings_commands::get_mcp_bridge_status,
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
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("error while running tauri application: {err}");
    }
}

pub fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    observability::logger::init_logging();
    mcp::run_stdio_server()
}

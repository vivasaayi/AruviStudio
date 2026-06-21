use serde::Serialize;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

pub(super) fn legacy_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        action_tool(
            "aruvi_catalog",
            "Manage products, product areas, capabilities, features, and attached delivery stories/tasks.",
            &[
                "create_product",
                "get_product",
                "list_products",
                "seed_example_products",
                "update_product",
                "archive_product",
                "create_product_area",
                "list_product_areas",
                "update_product_area",
                "delete_product_area",
                "reorder_product_areas",
                "create_capability",
                "list_capabilities",
                "update_capability",
                "delete_capability",
                "reorder_capabilities",
                "apply_capability_template",
                "convert_capability_kind",
                "get_product_tree",
                "list_references",
                "create_reference",
                "delete_reference",
                "get_bulk_import_schema",
                "submit_bulk_import",
                "get_bulk_import_status",
                "list_bulk_import_jobs",
            ],
        ),
        action_tool(
            "aruvi_work_items",
            "Manage delivery stories and tasks for Builder execution. Stories attach directly to product features.",
            &[
                "create_story",
                "create_task",
                "create_work_item",
                "get_work_item",
                "list_work_items",
                "summarize_work_items_by_product",
                "update_work_item",
                "delete_work_item",
                "get_sub_work_items",
                "reorder_work_items",
            ],
        ),
        action_tool(
            "aruvi_repositories",
            "Register repositories, attach them to product or product-area scope, create workspaces, and edit files safely.",
            &[
                "register_repository",
                "list_repositories",
                "delete_repository",
                "attach_repository",
                "resolve_repository_for_work_item",
                "resolve_repository_for_scope",
                "create_local_workspace",
                "list_repository_tree",
                "read_repository_file",
                "write_repository_file",
                "get_repository_file_sha256",
                "apply_repository_patch",
                "get_git_status",
                "get_git_diff",
                "list_git_changed_files",
                "get_git_current_branch",
            ],
        ),
        action_tool(
            "aruvi_planner",
            "Drive planner sessions, draft trees, repository analysis, and plan confirmation.",
            &[
                "create_planner_session",
                "update_planner_session",
                "clear_planner_pending",
                "submit_planner_turn",
                "submit_planner_voice_turn",
                "confirm_planner_plan",
                "rename_planner_draft_node",
                "add_planner_draft_child",
                "delete_planner_draft_node",
                "analyze_repository_for_planner",
            ],
        ),
        action_tool(
            "aruvi_workflows",
            "Control workflow execution and inspect workflow/agent run state.",
            &[
                "start_work_item_workflow",
                "get_workflow_run",
                "get_latest_workflow_run_for_work_item",
                "get_workflow_history",
                "handle_workflow_user_action",
                "advance_workflow",
                "list_agent_runs_for_workflow",
                "mark_workflow_run_failed",
                "restart_workflow_run",
            ],
        ),
        action_tool(
            "aruvi_checkpoints",
            "Handle approvals, artifacts, findings, and logs for checkpointing and review.",
            &[
                "approve_work_item",
                "reject_work_item",
                "approve_work_item_plan",
                "reject_work_item_plan",
                "approve_work_item_test_review",
                "get_work_item_approvals",
                "list_work_item_artifacts",
                "read_artifact_content",
                "list_work_item_findings",
                "get_logs",
            ],
        ),
        action_tool(
            "aruvi_agents",
            "Manage agents, teams, skills, model bindings, and workflow stage routing policies.",
            &[
                "list_agent_definitions",
                "list_agent_model_bindings",
                "set_primary_agent_model_binding",
                "create_agent_definition",
                "update_agent_definition",
                "delete_agent_definition",
                "list_agent_teams",
                "create_agent_team",
                "update_agent_team",
                "delete_agent_team",
                "list_team_memberships",
                "add_team_member",
                "remove_team_member",
                "list_team_assignments",
                "assign_team_scope",
                "remove_team_assignment",
                "list_skills",
                "create_skill",
                "update_skill",
                "delete_skill",
                "list_agent_skill_links",
                "link_skill_to_agent",
                "unlink_skill_from_agent",
                "list_team_skill_links",
                "link_skill_to_team",
                "unlink_skill_from_team",
                "list_workflow_stage_policies",
                "upsert_workflow_stage_policy",
                "delete_workflow_stage_policy",
            ],
        ),
        action_tool(
            "aruvi_agent_work",
            "Coordinate large multi-agent roadmap runs through MCP: runs, feature rows, atomic claims, leases, conflict locks, events, batches, and commit checkpoints.",
            &[
                "upsert_run",
                "get_run",
                "list_runs",
                "get_run_summary",
                "upsert_item",
                "list_items",
                "claim_next_item",
                "heartbeat_item",
                "update_item_status",
                "requeue_item",
                "requeue_expired_items",
                "release_item_locks",
                "list_active_locks",
                "list_conflict_zones",
                "inspect_conflict_zone",
                "reserve_conflict_zone",
                "release_conflict_zone",
                "complete_batch",
                "upsert_dependency",
                "delete_dependency",
                "list_dependencies",
                "list_ready_items",
                "append_evidence",
                "list_evidence",
                "get_run_health",
                "list_agent_activity",
                "append_event",
                "list_events",
                "link_commit",
                "import_legacy_checkpoint",
                "materialize_catalog",
                "link_catalog_work_items",
                "get_feature_context",
                "export_feature_context",
            ],
        ),
        action_tool(
            "aruvi_models",
            "Manage providers and models, test connectivity, register local runtimes, and run chat completion.",
            &[
                "create_provider",
                "list_providers",
                "update_provider",
                "delete_provider",
                "create_model_definition",
                "list_model_definitions",
                "update_model_definition",
                "delete_model_definition",
                "test_provider_connectivity",
                "register_local_runtime_model",
                "install_managed_local_model",
                "run_model_chat_completion",
            ],
        ),
        action_tool(
            "aruvi_settings",
            "Inspect and update operational settings, mobile and MCP bridge status, and database configuration.",
            &[
                "get_setting",
                "set_setting",
                "get_mobile_bridge_status",
                "get_mcp_bridge_status",
                "get_database_health",
                "get_active_database_path",
                "get_database_path_override",
                "set_database_path_override",
                "clear_database_path_override",
            ],
        ),
        action_tool(
            "aruvi_channels",
            "Use Twilio-backed outbound channels and planner contact routing.",
            &[
                "send_twilio_whatsapp_message",
                "start_twilio_voice_call",
                "route_planner_contact",
            ],
        ),
        action_tool(
            "aruvi_speech",
            "Transcribe audio and trigger native speech output for voice-driven planning flows.",
            &["transcribe_audio", "speak_text_natively"],
        ),
    ]
}

pub(super) fn action_tool(name: &str, description: &str, actions: &[&str]) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        title: None,
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": actions,
                    "description": "The operation to perform with this tool."
                },
                "arguments": {
                    "type": "object",
                    "description": "Action-specific arguments. Use snake_case or camelCase keys."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        }),
        output_schema: None,
    }
}

pub(super) fn first_class_tool(
    name: &str,
    title: &str,
    description: &str,
    input_schema: Value,
) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        title: Some(title.to_string()),
        description: description.to_string(),
        input_schema,
        output_schema: None,
    }
}

pub(super) fn empty_object_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

pub(super) fn object_schema(properties: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let mut property_map = Map::new();
    for (name, schema) in properties {
        property_map.insert(name.to_string(), schema);
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(property_map));
    schema.insert("additionalProperties".to_string(), json!(false));
    if !required.is_empty() {
        schema.insert("required".to_string(), json!(required));
    }

    Value::Object(schema)
}

pub(super) fn string_property(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description
    })
}

pub(super) fn string_array_property(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": {
            "type": "string"
        }
    })
}

pub(super) fn boolean_property(description: &str) -> Value {
    json!({
        "type": "boolean",
        "description": description
    })
}

pub(super) fn integer_property(description: &str) -> Value {
    json!({
        "type": "integer",
        "description": description
    })
}

pub(super) fn json_object_property(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "additionalProperties": true
    })
}

pub(super) fn enum_property(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values
    })
}

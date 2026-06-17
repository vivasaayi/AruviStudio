use crate::commands::model_commands::upsert_local_runtime_registration;
use crate::commands::repository_commands::create_local_workspace_for_scope;
use crate::commands::settings_commands::{DatabaseHealth, MigrationStatus};
use crate::domain::model::{ModelDefinition, ProviderType};
use crate::domain::product::{Capability, Module};
use crate::domain::work_item::WorkItem;
use crate::domain::workflow::UserAction;
use crate::error::AppError;
use crate::persistence::{
    agent_repo, agent_work_repo, approval_repo, artifact_repo, finding_repo, model_call_repo,
    model_repo, observability_repo, product_repo, repository_repo, settings_repo, work_item_repo,
    workflow_repo,
};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::secrets;
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::services::channel_service::{self, PlannerContactRequest};
use crate::services::planner_service::{
    add_planner_draft_child, analyze_repository_for_planner, clear_planner_pending,
    confirm_planner_plan, create_planner_session, delete_planner_draft_node,
    rename_planner_draft_node, submit_planner_turn, submit_planner_voice_turn,
    update_planner_session,
};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::services::speech_service::{
    looks_like_transcription_model, speak_text_natively, transcribe_audio_with_provider,
    TextToSpeechRequest,
};
use crate::services::webhook_service;
use crate::state::AppState;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sqlx::Row;
use std::path::Path;
use std::time::Instant;
use tracing::error;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";
const MCP_MODULE_SELECT_COLUMNS: &str = "id, product_id, CASE lower(replace(node_kind, '-', '_')) WHEN 'area' THEN 'area' WHEN 'product_area' THEN 'area' WHEN 'module' THEN 'area' WHEN 'strategic_area' THEN 'area' WHEN 'domain' THEN 'area' WHEN 'subdomain' THEN 'area' WHEN 'capability' THEN 'area' WHEN 'feature_set' THEN 'area' WHEN 'feature_group' THEN 'area' ELSE 'area' END AS node_kind, name, description, purpose, explanation, examples, implementation_notes, test_guidance, sort_order, created_at, updated_at";
const MCP_CAPABILITY_SELECT_COLUMNS: &str = "id, module_id, parent_capability_id, level, CASE lower(replace(node_kind, '-', '_')) WHEN 'capability' THEN 'capability' WHEN 'area' THEN 'capability' WHEN 'product_area' THEN 'capability' WHEN 'module' THEN 'capability' WHEN 'strategic_area' THEN 'capability' WHEN 'domain' THEN 'capability' WHEN 'subdomain' THEN 'capability' WHEN 'feature_set' THEN 'capability' WHEN 'feature_group' THEN 'capability' WHEN 'system' THEN 'capability' WHEN 'feature' THEN 'feature' WHEN 'rollout' THEN 'feature' WHEN 'capability_slice' THEN 'feature' ELSE CASE WHEN parent_capability_id IS NULL OR level <= 0 THEN 'capability' ELSE 'feature' END END AS node_kind, sort_order, name, description, acceptance_criteria, explanation, examples, priority, risk, status, technical_notes, implementation_notes, test_guidance, created_at, updated_at";

fn char_count_i64(content: &str) -> i64 {
    i64::try_from(content.chars().count()).unwrap_or(i64::MAX)
}

fn message_char_count(messages: &[ChatMessage]) -> i64 {
    messages
        .iter()
        .map(|message| char_count_i64(&message.content))
        .sum()
}

fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

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

pub fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = legacy_tool_definitions();
    definitions.extend(first_class_tool_definitions());
    definitions
}

fn legacy_tool_definitions() -> Vec<ToolDefinition> {
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

fn first_class_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "catalog.products.list",
            "List Products",
            "List visible products in the Aruvi catalog.",
            empty_object_schema(),
        ),
        first_class_tool(
            "catalog.products.get",
            "Get Product",
            "Get a single product by id.",
            object_schema(vec![("id", string_property("The product id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.products.create",
            "Create Product",
            "Create a new product.",
            object_schema(
                vec![
                    ("name", string_property("The product name.")),
                    ("description", string_property("Short product description.")),
                    ("vision", string_property("Longer product vision statement.")),
                    ("goals", string_array_property("Ordered list of product goals.")),
                    ("tags", string_array_property("Optional product tags.")),
                ],
                &["name"],
            ),
        ),
        first_class_tool(
            "catalog.products.update",
            "Update Product",
            "Update mutable fields on an existing product.",
            object_schema(
                vec![
                    ("id", string_property("The product id.")),
                    ("name", string_property("Updated product name.")),
                    ("description", string_property("Updated product description.")),
                    ("vision", string_property("Updated product vision.")),
                    ("goals", string_array_property("Updated list of product goals.")),
                    ("tags", string_array_property("Updated product tags.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.products.archive",
            "Archive Product",
            "Archive a product.",
            object_schema(vec![("id", string_property("The product id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.products.get_tree",
            "Get Product Tree",
            "Get the full semantic product tree for a product.",
            object_schema(
                vec![("productId", string_property("The product id."))],
                &["productId"],
            ),
        ),
        first_class_tool(
            "catalog.references.list",
            "List Catalog References",
            "List scoped notes, external docs, evidence, architecture references, standards, and design packets attached to product book scopes.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property(
                            "Optional scope type filter.",
                            &[
                                "strategy_node",
                                "product",
                                "product_area",
                                "capability",
                                "feature",
                                "delivery_item",
                            ],
                        ),
                    ),
                    ("scopeId", string_property("Optional scope id filter.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "catalog.references.create",
            "Create Catalog Reference",
            "Attach a scoped reference to a product, product area, capability, feature, or delivery item.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property(
                            "Reference scope type.",
                            &[
                                "strategy_node",
                                "product",
                                "product_area",
                                "capability",
                                "feature",
                                "delivery_item",
                            ],
                        ),
                    ),
                    ("scopeId", string_property("Reference scope id.")),
                    ("title", string_property("Reference title.")),
                    (
                        "referenceKind",
                        enum_property(
                            "Reference kind.",
                            &[
                                "note",
                                "external_doc",
                                "architecture",
                                "customer_evidence",
                                "regulatory",
                                "design_packet",
                                "standard",
                                "other",
                            ],
                        ),
                    ),
                    ("uri", string_property("Optional URI or file path.")),
                    ("content", string_property("Optional reference summary or pasted note.")),
                ],
                &["scopeType", "scopeId", "title"],
            ),
        ),
        first_class_tool(
            "catalog.references.delete",
            "Delete Catalog Reference",
            "Delete a scoped catalog reference.",
            object_schema(vec![("id", string_property("The reference id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.bulk_import.schema",
            "Get Bulk Import Schema",
            "Return the expected JSON and CSV structures for bulk importing products, product areas, capabilities, features, work items, and tasks.",
            empty_object_schema(),
        ),
        first_class_tool(
            "catalog.bulk_import.submit",
            "Submit Bulk Import",
            "Start an asynchronous JSON or CSV bulk import job from a local file path.",
            object_schema(
                vec![
                    ("filePath", string_property("JSON or CSV file path to import.")),
                    (
                        "format",
                        enum_property("Optional file format; inferred from extension when omitted.", &["json", "csv"]),
                    ),
                    (
                        "productId",
                        string_property("Optional existing product id used when the file does not define a product."),
                    ),
                ],
                &["filePath"],
            ),
        ),
        first_class_tool(
            "catalog.bulk_import.get_status",
            "Get Bulk Import Status",
            "Get durable status, progress counts, and recent errors for a bulk import job.",
            object_schema(vec![("jobId", string_property("Bulk import job id."))], &["jobId"]),
        ),
        first_class_tool(
            "catalog.bulk_import.list_jobs",
            "List Bulk Import Jobs",
            "List recent bulk import jobs.",
            object_schema(
                vec![("limit", integer_property("Maximum jobs to return, capped at 100."))],
                &[],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.list",
            "List Product Areas",
            "List product areas for a product. Product areas are the top-level product-management boundaries in the canonical Product Area > Capability > Feature hierarchy.",
            object_schema(
                vec![("productId", string_property("The product id."))],
                &["productId"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.create",
            "Create Product Area",
            "Create a top-level product area. Product areas must use nodeKind=area; see aruvi://catalog/node-kind-constraints.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    ("name", string_property("The product area name.")),
                    ("description", string_property("Short product area description.")),
                    ("purpose", string_property("Product area purpose or summary.")),
                    ("explanation", string_property("Long-form area explanation.")),
                    ("examples", string_property("Worked examples or concrete scenarios.")),
                    (
                        "implementationNotes",
                        string_property("Implementation-oriented notes for the product area."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test guidance or validation notes for the product area."),
                    ),
                    (
                        "nodeKind",
                        enum_property(
                            "Storage node kind for the product area.",
                            &["area"],
                        ),
                    ),
                ],
                &["productId", "name"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.update",
            "Update Product Area",
            "Update an existing top-level product area.",
            object_schema(
                vec![
                    ("id", string_property("The product area id.")),
                    ("name", string_property("Updated product area name.")),
                    ("description", string_property("Updated description.")),
                    ("purpose", string_property("Updated purpose.")),
                    ("explanation", string_property("Updated area explanation.")),
                    ("examples", string_property("Updated worked examples.")),
                    (
                        "implementationNotes",
                        string_property("Updated implementation-oriented notes."),
                    ),
                    (
                        "testGuidance",
                        string_property("Updated test guidance."),
                    ),
                    (
                        "nodeKind",
                        enum_property(
                            "Updated storage node kind for the product area.",
                            &["area"],
                        ),
                    ),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.delete",
            "Delete Product Area",
            "Delete a product area.",
            object_schema(vec![("id", string_property("The product area id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.product_areas.reorder",
            "Reorder Product Areas",
            "Reorder product areas within a product.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    (
                        "orderedIds",
                        string_array_property("Product area ids in the desired order."),
                    ),
                ],
                &["productId", "orderedIds"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.list",
            "List Capabilities",
            "List capabilities and features for a product area.",
            object_schema(
                vec![("productAreaId", string_property("The product area id."))],
                &["productAreaId"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.create",
            "Create Capability",
            "Create a capability or feature inside the product management hierarchy. Feature is the product-management leaf; stories and tasks live in work items.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    ("parentCapabilityId", string_property("Optional parent capability id.")),
                    ("name", string_property("The child node name.")),
                    ("description", string_property("Short node description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the node."),
                    ),
                    ("explanation", string_property("Long-form explanation for the node.")),
                    ("examples", string_property("Worked examples for the node.")),
                    (
                        "priority",
                        enum_property(
                            "Priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "risk",
                        enum_property("Risk level.", &["high", "medium", "low"]),
                    ),
                    (
                        "technicalNotes",
                        string_property("Technical notes for the node."),
                    ),
                    (
                        "implementationNotes",
                        string_property("Implementation plan or engineering notes."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test strategy or verification notes."),
                    ),
                    (
                        "nodeKind",
                        enum_property(
                            "Semantic node kind.",
                            &["capability", "feature"],
                        ),
                    ),
                ],
                &["productAreaId", "name"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.update",
            "Update Capability",
            "Update a capability or feature.",
            object_schema(
                vec![
                    ("id", string_property("The capability id.")),
                    ("name", string_property("Updated node name.")),
                    ("description", string_property("Updated description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Updated acceptance criteria."),
                    ),
                    ("explanation", string_property("Updated long-form explanation.")),
                    ("examples", string_property("Updated worked examples.")),
                    (
                        "priority",
                        enum_property(
                            "Updated priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "risk",
                        enum_property("Updated risk level.", &["high", "medium", "low"]),
                    ),
                    (
                        "technicalNotes",
                        string_property("Updated technical notes."),
                    ),
                    (
                        "implementationNotes",
                        string_property("Updated implementation plan or engineering notes."),
                    ),
                    (
                        "testGuidance",
                        string_property("Updated test strategy or verification notes."),
                    ),
                    (
                        "nodeKind",
                        enum_property(
                            "Updated semantic node kind.",
                            &["capability", "feature"],
                        ),
                    ),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.delete",
            "Delete Capability",
            "Delete a product design child node.",
            object_schema(vec![("id", string_property("The capability id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.capabilities.reorder",
            "Reorder Capabilities",
            "Reorder capabilities or features under a product design scope.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    ("parentCapabilityId", string_property("Optional parent capability id.")),
                    (
                        "orderedIds",
                        string_array_property("Child capability ids in the desired order."),
                    ),
                ],
                &["productAreaId", "orderedIds"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.apply_template",
            "Apply Capability Template",
            "Create a supported book-shaped subtree under a product area or capability. Use this for topics such as operator chapters with definition, examples, implementation, and tests.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    ("parentCapabilityId", string_property("Optional parent capability id.")),
                    (
                        "templateKind",
                        enum_property(
                            "Template kind to apply.",
                            &["operator_chapter", "technical_topic_book"],
                        ),
                    ),
                    ("name", string_property("Topic name for the generated subtree.")),
                    ("description", string_property("Optional chapter description.")),
                    ("explanation", string_property("Long-form explanation content.")),
                    ("examples", string_property("Worked examples content.")),
                    (
                        "implementationNotes",
                        string_property("Implementation guidance for the generated subtree."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test guidance for the generated subtree."),
                    ),
                    (
                        "priority",
                        enum_property(
                            "Priority level applied to generated nodes and work items.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "risk",
                        enum_property("Risk level.", &["high", "medium", "low"]),
                    ),
                ],
                &["productAreaId", "templateKind", "name"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.convert_kind",
            "Convert Capability Kind",
            "Safely convert a product design node between capability and feature. Use childStrategy=reparent_to_parent when converting a structural node into a feature while preserving children.",
            object_schema(
                vec![
                    ("id", string_property("The capability id.")),
                    (
                        "nodeKind",
                        enum_property(
                            "Target semantic node kind.",
                            &["capability", "feature"],
                        ),
                    ),
                    (
                        "childStrategy",
                        enum_property(
                            "How to handle existing structural children during conversion.",
                            &["reject", "reparent_to_parent"],
                        ),
                    ),
                ],
                &["id", "nodeKind"],
            ),
        ),
        first_class_tool(
            "work_items.list",
            "List Work Items",
            "List delivery stories and tasks filtered by product, product area, feature, source scope, or status.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("sourceNodeId", string_property("Optional source node id.")),
                    (
                        "sourceNodeType",
                        enum_property(
                            "Optional source node type.",
                            &["product_area", "capability", "feature"],
                        ),
                    ),
                    ("status", string_property("Optional work item status filter.")),
                    (
                        "limit",
                        integer_property("Maximum rows to return, capped at 2000."),
                    ),
                    ("offset", integer_property("Pagination offset.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "work_items.get",
            "Get Work Item",
            "Get a work item by id.",
            object_schema(vec![("id", string_property("The work item id."))], &["id"]),
        ),
        first_class_tool(
            "work_items.create",
            "Create Work Item",
            "Create a delivery story or task attached to a product and optional source scope. Prefer work_items.stories.create for feature-attached stories and work_items.tasks.create for tasks under stories.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("sourceNodeId", string_property("Optional source node id.")),
                    (
                        "sourceNodeType",
                        enum_property(
                            "Optional source node type.",
                            &["product_area", "capability", "feature"],
                        ),
                    ),
                    ("parentWorkItemId", string_property("Optional parent work item id.")),
                    ("title", string_property("The work item title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short work item description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the work item."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "workItemType",
                        enum_property(
                            "Delivery work item type. story/task are accepted MCP aliases and persist on the legacy delivery type.",
                            &[
                                "story",
                                "task",
                                "setup",
                                "bug",
                                "refactor",
                                "test",
                                "review",
                                "security_fix",
                                "performance_improvement",
                            ],
                        ),
                    ),
                    (
                        "priority",
                        enum_property(
                            "Priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["productId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.stories.create",
            "Create Story",
            "Create a delivery story attached directly to a product feature.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    ("featureId", string_property("The feature id that owns this story.")),
                    ("title", string_property("The story title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short story description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the story."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "priority",
                        enum_property(
                            "Priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["productId", "featureId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.tasks.create",
            "Create Task",
            "Create an implementation, test, review, documentation, or release task under a delivery story.",
            object_schema(
                vec![
                    ("storyId", string_property("The parent story work item id.")),
                    ("productId", string_property("Optional product id. If omitted, it is inherited from the story.")),
                    ("title", string_property("The task title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short task description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the task."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "priority",
                        enum_property(
                            "Priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["storyId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.update",
            "Update Work Item",
            "Update mutable fields on a work item.",
            object_schema(
                vec![
                    ("id", string_property("The work item id.")),
                    ("title", string_property("Updated title.")),
                    ("description", string_property("Updated description.")),
                    ("status", string_property("Updated status.")),
                    ("problemStatement", string_property("Updated problem statement.")),
                    (
                        "acceptanceCriteria",
                        string_property("Updated acceptance criteria."),
                    ),
                    ("constraints", string_property("Updated constraints.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "work_items.delete",
            "Delete Work Item",
            "Delete a work item.",
            object_schema(vec![("id", string_property("The work item id."))], &["id"]),
        ),
        first_class_tool(
            "work_items.list_children",
            "List Child Work Items",
            "List direct child work items for a parent work item.",
            object_schema(
                vec![("workItemId", string_property("The parent work item id."))],
                &["workItemId"],
            ),
        ),
        first_class_tool(
            "work_items.reorder",
            "Reorder Work Items",
            "Reorder work items by supplying the desired ordered ids.",
            object_schema(
                vec![(
                    "orderedIds",
                    string_array_property("Work item ids in the desired order."),
                )],
                &["orderedIds"],
            ),
        ),
        first_class_tool(
            "work_items.summarize_by_product",
            "Summarize Work Items By Product",
            "Summarize work item counts grouped by product.",
            empty_object_schema(),
        ),
        first_class_tool(
            "agent_work.runs.upsert",
            "Upsert Agent Work Run",
            "Create or update a durable multi-agent roadmap run ledger.",
            object_schema(
                vec![
                    ("id", string_property("Run id, for example run-02-sim.")),
                    ("productId", string_property("Optional Aruvi product id.")),
                    ("repositoryId", string_property("Optional repository id.")),
                    ("roadmapHash", string_property("Hash of roadmap inputs.")),
                    (
                        "status",
                        enum_property(
                            "Run status.",
                            &["active", "paused", "completed", "blocked", "cancelled"],
                        ),
                    ),
                    ("lastCommitSha", string_property("Latest implementation commit SHA.")),
                    ("currentBatchId", string_property("Current batch id.")),
                    ("nextAction", string_property("Exact next action for resume.")),
                    ("metadata", json_object_property("Optional run metadata.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "agent_work.runs.get",
            "Get Agent Work Run",
            "Get a multi-agent run ledger.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.runs.list",
            "List Agent Work Runs",
            "List multi-agent run ledgers.",
            object_schema(
                vec![
                    ("status", string_property("Optional run status filter.")),
                    ("limit", integer_property("Maximum rows to return.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "agent_work.runs.summary",
            "Get Agent Work Run Summary",
            "Get run status counts, active lock count, and recent events.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("eventLimit", integer_property("Recent event limit.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.upsert",
            "Upsert Agent Work Item",
            "Create or update a feature row in a multi-agent run ledger.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("module", string_property("Roadmap module or domain id.")),
                    ("serviceOrDomain", string_property("Optional service/domain label.")),
                    ("priority", string_property("Priority, for example P0/P1/P2.")),
                    ("releasePhase", string_property("Release phase, for example M1.")),
                    ("title", string_property("Feature title.")),
                    ("description", string_property("Feature description.")),
                    (
                        "status",
                        enum_property(
                            "Feature status.",
                            &[
                                "pending",
                                "claimed",
                                "in_progress",
                                "implemented",
                                "tests_passed",
                                "committed",
                                "blocked",
                                "skipped",
                                "cancelled",
                            ],
                        ),
                    ),
                    ("batchId", string_property("Optional batch id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    (
                        "conflictZones",
                        string_array_property(
                            "Serialized resources that cannot be edited concurrently.",
                        ),
                    ),
                    ("metadata", json_object_property("Optional feature metadata.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.list",
            "List Agent Work Items",
            "List feature rows with server-side filters and pagination.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("status", string_property("Optional feature status filter.")),
                    ("agent", string_property("Optional agent filter.")),
                    ("limit", integer_property("Maximum rows.")),
                    ("offset", integer_property("Offset.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.claim_next",
            "Claim Next Agent Work Item",
            "Atomically claim the next pending feature row, create/update its batch, and acquire conflict-zone locks with a lease.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("agent", string_property("Claiming agent id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("selectionRule", string_property("Why this row was selected.")),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "agent"],
            ),
        ),
        first_class_tool(
            "agent_work.items.heartbeat",
            "Heartbeat Agent Work Item",
            "Extend a claimed feature row lease and its conflict locks.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("claimToken", string_property("Claim token returned by claim_next.")),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "featureId", "claimToken"],
            ),
        ),
        first_class_tool(
            "agent_work.items.update_status",
            "Update Agent Work Item Status",
            "Update feature row status and append a checkpoint event.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("status", string_property("New feature status.")),
                    ("agent", string_property("Agent id.")),
                    ("batchId", string_property("Batch id.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "featureId", "status"],
            ),
        ),
        first_class_tool(
            "agent_work.items.release_locks",
            "Release Agent Work Item Locks",
            "Release active conflict-zone locks for a feature row.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.requeue",
            "Requeue Agent Work Item",
            "Move a claimed, in-progress, blocked, or stale feature row back to pending and release its locks.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("agent", string_property("Agent or coordinator id.")),
                    ("details", string_property("Reason for requeue.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.requeue_expired",
            "Requeue Expired Agent Work Items",
            "Release expired leases and move stale claimed/in-progress rows back to pending.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("agent", string_property("Agent or coordinator id.")),
                    ("details", string_property("Reason recorded on requeue events.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.list_ready",
            "List Ready Agent Work Items",
            "List pending feature rows whose dependencies are already satisfied.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("limit", integer_property("Maximum rows.")),
                    ("offset", integer_property("Offset.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.locks.list_active",
            "List Active Agent Work Locks",
            "List active conflict-zone locks for a run.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.conflict_zones.list",
            "List Agent Work Conflict Zones",
            "List active conflict-zone reservations grouped by zone.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.conflict_zones.inspect",
            "Inspect Agent Work Conflict Zone",
            "Inspect active locks for one conflict zone.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                ],
                &["runId", "zoneKey"],
            ),
        ),
        first_class_tool(
            "agent_work.conflict_zones.reserve",
            "Reserve Agent Work Conflict Zone",
            "Reserve a run-level conflict zone such as a shared build target, migration lane, or repo path.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                    ("agent", string_property("Agent id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("claimToken", string_property("Optional claim token.")),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "zoneKey", "agent"],
            ),
        ),
        first_class_tool(
            "agent_work.conflict_zones.release",
            "Release Agent Work Conflict Zone",
            "Release an active conflict-zone reservation.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                ],
                &["runId", "zoneKey"],
            ),
        ),
        first_class_tool(
            "agent_work.batches.complete",
            "Complete Agent Work Batch",
            "Mark a batch implemented, tests_passed, committed, blocked, skipped, or cancelled.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Batch id.")),
                    ("status", string_property("Batch status.")),
                    ("agent", string_property("Agent id.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "batchId", "status"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.upsert",
            "Upsert Agent Work Dependency",
            "Record that one feature row depends on another feature row before it can be claimed.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Dependent feature id.")),
                    ("dependsOnFeatureId", string_property("Prerequisite feature id.")),
                    ("dependencyKind", string_property("Dependency kind.")),
                    ("metadata", json_object_property("Optional dependency metadata.")),
                ],
                &["runId", "featureId", "dependsOnFeatureId"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.delete",
            "Delete Agent Work Dependency",
            "Remove a dependency edge between two feature rows.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Dependent feature id.")),
                    ("dependsOnFeatureId", string_property("Prerequisite feature id.")),
                ],
                &["runId", "featureId", "dependsOnFeatureId"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.list",
            "List Agent Work Dependencies",
            "List dependency edges for a run or one feature row.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Optional dependent feature id.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.evidence.append",
            "Append Agent Work Evidence",
            "Append structured evidence for commands, tests, changed files, artifacts, or validation.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("evidenceType", string_property("Evidence type, such as test, diff, validation, review.")),
                    ("command", string_property("Command that produced the evidence.")),
                    ("exitCode", integer_property("Command exit code.")),
                    ("status", string_property("Evidence status.")),
                    ("summary", string_property("Short evidence summary.")),
                    ("details", string_property("Long evidence details.")),
                    ("changedFiles", string_array_property("Changed files.")),
                    ("artifactRefs", string_array_property("Artifact ids or URIs.")),
                    ("metadata", json_object_property("Optional evidence metadata.")),
                ],
                &["runId", "evidenceType"],
            ),
        ),
        first_class_tool(
            "agent_work.evidence.list",
            "List Agent Work Evidence",
            "List structured evidence for a run, feature, batch, or agent.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("limit", integer_property("Maximum rows.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.runs.health",
            "Get Agent Work Run Health",
            "Get summary counts, expired claims, ready items, active agents, active conflict zones, and latest evidence.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.agents.activity",
            "List Agent Work Agent Activity",
            "List agent activity counts and latest heartbeat/event times for a run.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.events.append",
            "Append Agent Work Event",
            "Append a durable coordination/checkpoint event.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("eventType", string_property("Event type.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature row id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("command", string_property("Command or action.")),
                    ("status", string_property("Event status.")),
                    ("details", string_property("Event details.")),
                    ("metadata", json_object_property("Optional event metadata.")),
                ],
                &["runId", "eventType"],
            ),
        ),
        first_class_tool(
            "agent_work.events.list",
            "List Agent Work Events",
            "List checkpoint events with cursor and feature filters.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("afterId", integer_property("Only events after this id.")),
                    ("featureId", string_property("Optional feature row id.")),
                    ("limit", integer_property("Maximum rows.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.commits.link",
            "Link Agent Work Commit",
            "Link a git commit to feature rows and a batch, mark them committed, release locks, and update the run checkpoint.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Batch id.")),
                    (
                        "featureIds",
                        string_array_property("Feature row ids included in the commit."),
                    ),
                    ("commitSha", string_property("Commit SHA.")),
                    ("agent", string_property("Agent id.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "batchId", "featureIds", "commitSha"],
            ),
        ),
        first_class_tool(
            "agent_work.import_legacy_checkpoint",
            "Import Legacy Agent Checkpoint",
            "Import an AGBot-style .codex/.claude checkpoint.sqlite ledger into the Aruvi MCP agent-work tables.",
            object_schema(
                vec![
                    ("checkpointPath", string_property("Absolute path to checkpoint.sqlite.")),
                    ("runId", string_property("Optional target run id.")),
                    ("sourceLabel", string_property("Optional source label.")),
                ],
                &["checkpointPath"],
            ),
        ),
        first_class_tool(
            "agent_work.materialize_catalog",
            "Materialize Agent Work Catalog",
            "Bulk-create or update catalog product areas, capabilities, features, visible work items, and ledger work-item links from an agent-work run.",
            object_schema(
                vec![
                    ("runId", string_property("Agent-work run id.")),
                    (
                        "productId",
                        string_property(
                            "Optional product id. Required when the run is not attached to a product.",
                        ),
                    ),
                    (
                        "createWorkItems",
                        boolean_property(
                            "Whether to create visible delivery work items and link ledger rows. Defaults to true.",
                        ),
                    ),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.link_catalog_work_items",
            "Link Catalog Work Items",
            "Link existing catalog work items to agent-work rows by matching stable feature ids.",
            object_schema(
                vec![
                    ("runId", string_property("Agent-work run id.")),
                    (
                        "productId",
                        string_property(
                            "Optional product id. Required when the run is not attached to a product.",
                        ),
                    ),
                    (
                        "syncStatuses",
                        boolean_property(
                            "Whether to sync agent-work status onto matched work items and catalog features. Defaults to false.",
                        ),
                    ),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.context.get_feature",
            "Get Feature Implementation Context",
            "Get 360-degree product, feature, story, parent, sibling, reference, dependency, evidence, and agent-work context for implementation.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("featureId", string_property("Optional product feature/capability id.")),
                    ("workItemId", string_property("Optional story/task id.")),
                    ("runId", string_property("Optional agent-work run id.")),
                    ("includeProductTree", boolean_property("Include full product tree.")),
                    ("siblingLimit", integer_property("Maximum siblings to include.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "agent_work.context.export_feature",
            "Export Feature Implementation Context",
            "Write feature implementation context to a JSON or Markdown file for agent handoff.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("featureId", string_property("Optional product feature/capability id.")),
                    ("workItemId", string_property("Optional story/task id.")),
                    ("runId", string_property("Optional agent-work run id.")),
                    ("includeProductTree", boolean_property("Include full product tree.")),
                    ("siblingLimit", integer_property("Maximum siblings to include.")),
                    ("outputPath", string_property("File path to write.")),
                    ("format", enum_property("Export format.", &["json", "markdown"])),
                ],
                &["outputPath"],
            ),
        ),
        first_class_tool(
            "repositories.list",
            "List Repositories",
            "List registered repositories.",
            empty_object_schema(),
        ),
        first_class_tool(
            "repositories.register",
            "Register Repository",
            "Register a repository with Aruvi.",
            object_schema(
                vec![
                    ("name", string_property("Repository display name.")),
                    ("localPath", string_property("Absolute local repository path.")),
                    ("remoteUrl", string_property("Optional remote url.")),
                    ("defaultBranch", string_property("Default branch name.")),
                ],
                &["name", "localPath"],
            ),
        ),
        first_class_tool(
            "repositories.delete",
            "Delete Repository",
            "Delete a registered repository.",
            object_schema(vec![("id", string_property("The repository id."))], &["id"]),
        ),
        first_class_tool(
            "repositories.attachments.create",
            "Attach Repository",
            "Attach a repository to a product or product area scope.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property("Attachment scope type.", &["product", "product_area"]),
                    ),
                    ("scopeId", string_property("Scope id to attach to.")),
                    ("repositoryId", string_property("The repository id.")),
                    ("isDefault", boolean_property("Whether the attachment is the default.")),
                ],
                &["scopeType", "scopeId", "repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_work_item",
            "Resolve Repository For Work Item",
            "Resolve the repository associated with a work item.",
            object_schema(
                vec![("workItemId", string_property("The work item id."))],
                &["workItemId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_scope",
            "Resolve Repository For Scope",
            "Resolve the repository associated with a product or product area scope.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.workspaces.create_for_scope",
            "Create Local Workspace",
            "Create a local workspace for a product, product area, or work item scope.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("workItemId", string_property("Optional work item id.")),
                    ("preferredPath", string_property("Optional preferred workspace path.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.trees.list",
            "List Repository Tree",
            "List the file tree for a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("includeHidden", boolean_property("Whether to include hidden files.")),
                    ("maxDepth", integer_property("Optional maximum traversal depth.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.files.read",
            "Read Repository File",
            "Read a file from a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.write",
            "Write Repository File",
            "Write a file in a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                    ("content", string_property("New file content.")),
                ],
                &["repositoryId", "relativePath", "content"],
            ),
        ),
        first_class_tool(
            "repositories.files.get_sha256",
            "Get Repository File SHA256",
            "Get the SHA256 of a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.apply_patch",
            "Apply Repository Patch",
            "Apply a patch to a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                    ("patch", string_property("Unified patch text to apply.")),
                    (
                        "baseSha256",
                        string_property("Optional expected base SHA256 for optimistic locking."),
                    ),
                ],
                &["repositoryId", "relativePath", "patch"],
            ),
        ),
        first_class_tool(
            "repositories.git.status",
            "Get Repository Git Status",
            "Get branch, head SHA, dirty flag, and changed file status for a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("includeIgnored", boolean_property("Whether to include ignored files.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.diff",
            "Get Repository Git Diff",
            "Get a patch diff for working tree changes in a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("maxBytes", integer_property("Maximum diff bytes returned.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.changed_files",
            "List Repository Git Changed Files",
            "List changed files in a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.current_branch",
            "Get Repository Git Current Branch",
            "Get the current branch for a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
    ]
}

pub async fn dispatch_tool(
    state: &AppState,
    tool_name: &str,
    payload: Value,
) -> Result<Value, AppError> {
    if is_legacy_tool_name(tool_name) {
        return dispatch_namespace_tool(state, tool_name, payload).await;
    }

    if let Some((namespace_tool, adapted_payload)) = translate_first_class_tool(tool_name, payload)?
    {
        return dispatch_namespace_tool(state, namespace_tool, adapted_payload).await;
    }

    Err(AppError::Validation(format!(
        "Unknown MCP tool: {tool_name}"
    )))
}

async fn dispatch_namespace_tool(
    state: &AppState,
    tool_name: &str,
    payload: Value,
) -> Result<Value, AppError> {
    match tool_name {
        "aruvi_catalog" => handle_catalog(state, payload).await,
        "aruvi_work_items" => handle_work_items(state, payload).await,
        "aruvi_repositories" => handle_repositories(state, payload).await,
        "aruvi_planner" => handle_planner(state, payload).await,
        "aruvi_workflows" => handle_workflows(state, payload).await,
        "aruvi_checkpoints" => handle_checkpoints(state, payload).await,
        "aruvi_agents" => handle_agents(state, payload).await,
        "aruvi_agent_work" => handle_agent_work(state, payload).await,
        "aruvi_models" => handle_models(state, payload).await,
        "aruvi_settings" => handle_settings(state, payload).await,
        "aruvi_channels" => handle_channels(state, payload).await,
        "aruvi_speech" => handle_speech(state, payload).await,
        _ => Err(AppError::Validation(format!(
            "Unknown MCP namespace tool: {tool_name}"
        ))),
    }
}

fn is_legacy_tool_name(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "aruvi_catalog"
            | "aruvi_work_items"
            | "aruvi_repositories"
            | "aruvi_planner"
            | "aruvi_workflows"
            | "aruvi_checkpoints"
            | "aruvi_agents"
            | "aruvi_agent_work"
            | "aruvi_models"
            | "aruvi_settings"
            | "aruvi_channels"
            | "aruvi_speech"
    )
}

fn translate_first_class_tool(
    tool_name: &str,
    payload: Value,
) -> Result<Option<(&'static str, Value)>, AppError> {
    let (namespace_tool, action) = match tool_name {
        "catalog.products.list" => ("aruvi_catalog", "list_products"),
        "catalog.products.get" => ("aruvi_catalog", "get_product"),
        "catalog.products.create" => ("aruvi_catalog", "create_product"),
        "catalog.products.update" => ("aruvi_catalog", "update_product"),
        "catalog.products.archive" => ("aruvi_catalog", "archive_product"),
        "catalog.products.get_tree" => ("aruvi_catalog", "get_product_tree"),
        "catalog.references.list" => ("aruvi_catalog", "list_references"),
        "catalog.references.create" => ("aruvi_catalog", "create_reference"),
        "catalog.references.delete" => ("aruvi_catalog", "delete_reference"),
        "catalog.bulk_import.schema" => ("aruvi_catalog", "get_bulk_import_schema"),
        "catalog.bulk_import.submit" => ("aruvi_catalog", "submit_bulk_import"),
        "catalog.bulk_import.get_status" => ("aruvi_catalog", "get_bulk_import_status"),
        "catalog.bulk_import.list_jobs" => ("aruvi_catalog", "list_bulk_import_jobs"),
        "catalog.product_areas.list" => ("aruvi_catalog", "list_product_areas"),
        "catalog.product_areas.create" => ("aruvi_catalog", "create_product_area"),
        "catalog.product_areas.update" => ("aruvi_catalog", "update_product_area"),
        "catalog.product_areas.delete" => ("aruvi_catalog", "delete_product_area"),
        "catalog.product_areas.reorder" => ("aruvi_catalog", "reorder_product_areas"),
        "catalog.modules.list" => ("aruvi_catalog", "list_modules"),
        "catalog.modules.create" => ("aruvi_catalog", "create_module"),
        "catalog.modules.update" => ("aruvi_catalog", "update_module"),
        "catalog.modules.delete" => ("aruvi_catalog", "delete_module"),
        "catalog.modules.reorder" => ("aruvi_catalog", "reorder_modules"),
        "catalog.capabilities.list" => ("aruvi_catalog", "list_capabilities"),
        "catalog.capabilities.create" => ("aruvi_catalog", "create_capability"),
        "catalog.capabilities.update" => ("aruvi_catalog", "update_capability"),
        "catalog.capabilities.delete" => ("aruvi_catalog", "delete_capability"),
        "catalog.capabilities.reorder" => ("aruvi_catalog", "reorder_capabilities"),
        "catalog.capabilities.apply_template" => ("aruvi_catalog", "apply_capability_template"),
        "catalog.capabilities.convert_kind" => ("aruvi_catalog", "convert_capability_kind"),
        "work_items.list" => ("aruvi_work_items", "list_work_items"),
        "work_items.get" => ("aruvi_work_items", "get_work_item"),
        "work_items.create" => ("aruvi_work_items", "create_work_item"),
        "work_items.stories.create" => ("aruvi_work_items", "create_story"),
        "work_items.tasks.create" => ("aruvi_work_items", "create_task"),
        "work_items.update" => ("aruvi_work_items", "update_work_item"),
        "work_items.delete" => ("aruvi_work_items", "delete_work_item"),
        "work_items.list_children" => ("aruvi_work_items", "get_sub_work_items"),
        "work_items.reorder" => ("aruvi_work_items", "reorder_work_items"),
        "work_items.summarize_by_product" => {
            ("aruvi_work_items", "summarize_work_items_by_product")
        }
        "agent_work.runs.upsert" => ("aruvi_agent_work", "upsert_run"),
        "agent_work.runs.get" => ("aruvi_agent_work", "get_run"),
        "agent_work.runs.list" => ("aruvi_agent_work", "list_runs"),
        "agent_work.runs.summary" => ("aruvi_agent_work", "get_run_summary"),
        "agent_work.items.upsert" => ("aruvi_agent_work", "upsert_item"),
        "agent_work.items.list" => ("aruvi_agent_work", "list_items"),
        "agent_work.items.claim_next" => ("aruvi_agent_work", "claim_next_item"),
        "agent_work.items.heartbeat" => ("aruvi_agent_work", "heartbeat_item"),
        "agent_work.items.update_status" => ("aruvi_agent_work", "update_item_status"),
        "agent_work.items.release_locks" => ("aruvi_agent_work", "release_item_locks"),
        "agent_work.items.requeue" => ("aruvi_agent_work", "requeue_item"),
        "agent_work.items.requeue_expired" => ("aruvi_agent_work", "requeue_expired_items"),
        "agent_work.items.list_ready" => ("aruvi_agent_work", "list_ready_items"),
        "agent_work.locks.list_active" => ("aruvi_agent_work", "list_active_locks"),
        "agent_work.conflict_zones.list" => ("aruvi_agent_work", "list_conflict_zones"),
        "agent_work.conflict_zones.inspect" => ("aruvi_agent_work", "inspect_conflict_zone"),
        "agent_work.conflict_zones.reserve" => ("aruvi_agent_work", "reserve_conflict_zone"),
        "agent_work.conflict_zones.release" => ("aruvi_agent_work", "release_conflict_zone"),
        "agent_work.batches.complete" => ("aruvi_agent_work", "complete_batch"),
        "agent_work.dependencies.upsert" => ("aruvi_agent_work", "upsert_dependency"),
        "agent_work.dependencies.delete" => ("aruvi_agent_work", "delete_dependency"),
        "agent_work.dependencies.list" => ("aruvi_agent_work", "list_dependencies"),
        "agent_work.evidence.append" => ("aruvi_agent_work", "append_evidence"),
        "agent_work.evidence.list" => ("aruvi_agent_work", "list_evidence"),
        "agent_work.runs.health" => ("aruvi_agent_work", "get_run_health"),
        "agent_work.agents.activity" => ("aruvi_agent_work", "list_agent_activity"),
        "agent_work.events.append" => ("aruvi_agent_work", "append_event"),
        "agent_work.events.list" => ("aruvi_agent_work", "list_events"),
        "agent_work.commits.link" => ("aruvi_agent_work", "link_commit"),
        "agent_work.import_legacy_checkpoint" => ("aruvi_agent_work", "import_legacy_checkpoint"),
        "agent_work.materialize_catalog" => ("aruvi_agent_work", "materialize_catalog"),
        "agent_work.link_catalog_work_items" => ("aruvi_agent_work", "link_catalog_work_items"),
        "agent_work.context.get_feature" => ("aruvi_agent_work", "get_feature_context"),
        "agent_work.context.export_feature" => ("aruvi_agent_work", "export_feature_context"),
        "repositories.list" => ("aruvi_repositories", "list_repositories"),
        "repositories.register" => ("aruvi_repositories", "register_repository"),
        "repositories.delete" => ("aruvi_repositories", "delete_repository"),
        "repositories.attachments.create" => ("aruvi_repositories", "attach_repository"),
        "repositories.resolution.for_work_item" => {
            ("aruvi_repositories", "resolve_repository_for_work_item")
        }
        "repositories.resolution.for_scope" => {
            ("aruvi_repositories", "resolve_repository_for_scope")
        }
        "repositories.workspaces.create_for_scope" => {
            ("aruvi_repositories", "create_local_workspace")
        }
        "repositories.trees.list" => ("aruvi_repositories", "list_repository_tree"),
        "repositories.files.read" => ("aruvi_repositories", "read_repository_file"),
        "repositories.files.write" => ("aruvi_repositories", "write_repository_file"),
        "repositories.files.get_sha256" => ("aruvi_repositories", "get_repository_file_sha256"),
        "repositories.files.apply_patch" => ("aruvi_repositories", "apply_repository_patch"),
        "repositories.git.status" => ("aruvi_repositories", "get_git_status"),
        "repositories.git.diff" => ("aruvi_repositories", "get_git_diff"),
        "repositories.git.changed_files" => ("aruvi_repositories", "list_git_changed_files"),
        "repositories.git.current_branch" => ("aruvi_repositories", "get_git_current_branch"),
        _ => return Ok(None),
    };

    let arguments = match payload {
        Value::Object(map) => Value::Object(map),
        Value::Null => Value::Object(Map::new()),
        _ => {
            return Err(AppError::Validation(format!(
                "{tool_name} arguments must be a JSON object"
            )))
        }
    };

    Ok(Some((
        namespace_tool,
        json!({
            "action": action,
            "arguments": arguments
        }),
    )))
}

fn action_tool(name: &str, description: &str, actions: &[&str]) -> ToolDefinition {
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

fn first_class_tool(
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

fn empty_object_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

fn object_schema(properties: Vec<(&str, Value)>, required: &[&str]) -> Value {
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

fn string_property(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description
    })
}

fn string_array_property(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": {
            "type": "string"
        }
    })
}

fn boolean_property(description: &str) -> Value {
    json!({
        "type": "boolean",
        "description": description
    })
}

fn integer_property(description: &str) -> Value {
    json!({
        "type": "integer",
        "description": description
    })
}

fn json_object_property(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "additionalProperties": true
    })
}

fn enum_property(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values
    })
}

struct ToolAction {
    action: String,
    arguments: Value,
}

impl ToolAction {
    fn parse(payload: Value) -> Result<Self, AppError> {
        let object = payload.as_object().ok_or_else(|| {
            AppError::Validation("tool payload must be a JSON object".to_string())
        })?;
        let root = ActionArgs { object };
        let action = root.required_string(&["action"], "action")?;
        let arguments = match object.get("arguments") {
            Some(Value::Object(map)) => Value::Object(map.clone()),
            Some(Value::Null) | None => Value::Object(Map::new()),
            Some(_) => {
                return Err(AppError::Validation(
                    "tool payload arguments must be a JSON object".to_string(),
                ))
            }
        };

        Ok(Self { action, arguments })
    }

    fn args(&self) -> ActionArgs<'_> {
        ActionArgs {
            object: self
                .arguments
                .as_object()
                .expect("arguments must be object"),
        }
    }
}

struct ActionArgs<'a> {
    object: &'a Map<String, Value>,
}

impl<'a> ActionArgs<'a> {
    fn get(&self, keys: &[&str]) -> Option<&Value> {
        for key in keys {
            if let Some(value) = self.object.get(*key) {
                return Some(value);
            }
        }
        None
    }

    fn required_string(&self, keys: &[&str], label: &str) -> Result<String, AppError> {
        self.optional_string(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    fn optional_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.to_string())),
            Some(Value::Number(value)) => Ok(Some(value.to_string())),
            Some(Value::Bool(value)) => Ok(Some(value.to_string())),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a string",
                keys[0]
            ))),
        }
    }

    fn string_or_default(&self, keys: &[&str], default: &str) -> Result<String, AppError> {
        Ok(self
            .optional_string(keys)?
            .unwrap_or_else(|| default.to_string()))
    }

    fn optional_bool(&self, keys: &[&str]) -> Result<Option<bool>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(value)) => Ok(Some(*value)),
            Some(Value::String(value)) => match value.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                _ => Err(AppError::Validation(format!(
                    "{} must be a boolean",
                    keys[0]
                ))),
            },
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a boolean",
                keys[0]
            ))),
        }
    }

    fn bool_or_default(&self, keys: &[&str], default: bool) -> Result<bool, AppError> {
        Ok(self.optional_bool(keys)?.unwrap_or(default))
    }

    fn optional_i64(&self, keys: &[&str]) -> Result<Option<i64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_i64()
                .ok_or_else(|| AppError::Validation(format!("{} must be an integer", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<i64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be an integer", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an integer",
                keys[0]
            ))),
        }
    }

    fn optional_i32(&self, keys: &[&str]) -> Result<Option<i32>, AppError> {
        self.optional_i64(keys)?
            .map(|value| {
                i32::try_from(value).map_err(|_| {
                    AppError::Validation(format!("{} is out of range for i32", keys[0]))
                })
            })
            .transpose()
    }

    fn optional_f64(&self, keys: &[&str]) -> Result<Option<f64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_f64()
                .ok_or_else(|| AppError::Validation(format!("{} must be a number", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<f64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be a number", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a number",
                keys[0]
            ))),
        }
    }

    fn required_string_list(&self, keys: &[&str], label: &str) -> Result<Vec<String>, AppError> {
        self.optional_string_list(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    fn optional_string_list(&self, keys: &[&str]) -> Result<Option<Vec<String>>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(|value| value.to_string())
                        .ok_or_else(|| {
                            AppError::Validation(format!("{} must contain only strings", keys[0]))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
            Some(Value::String(value)) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(Some(Vec::new()));
                }
                if trimmed.starts_with('[') {
                    let parsed = serde_json::from_str::<Vec<String>>(trimmed)?;
                    Ok(Some(parsed))
                } else {
                    Ok(Some(
                        trimmed
                            .split(',')
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(ToString::to_string)
                            .collect(),
                    ))
                }
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an array of strings",
                keys[0]
            ))),
        }
    }

    fn optional_json_array_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        self.optional_string_list(keys)?
            .map(|value| serde_json::to_string(&value).map_err(AppError::from))
            .transpose()
    }

    fn optional_json_object_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(value)) => serde_json::to_string(value)
                .map(Some)
                .map_err(AppError::from),
            Some(Value::String(value)) => {
                let parsed = serde_json::from_str::<Value>(value)?;
                if !parsed.is_object() {
                    return Err(AppError::Validation(format!(
                        "{} must be a JSON object",
                        keys[0]
                    )));
                }
                serde_json::to_string(&parsed)
                    .map(Some)
                    .map_err(AppError::from)
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a JSON object",
                keys[0]
            ))),
        }
    }

    fn optional_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<Option<T>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => serde_json::from_value::<T>(value.clone())
                .map(Some)
                .map_err(|error| AppError::Validation(format!("invalid {label}: {error}"))),
        }
    }

    fn required_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<T, AppError> {
        self.optional_deserialize(keys, label)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }
}

fn action_result<T: Serialize>(action: &str, result: T) -> Result<Value, AppError> {
    Ok(json!({
        "action": action,
        "result": serde_json::to_value(result)?
    }))
}

fn action_ok(action: &str) -> Value {
    json!({
        "action": action,
        "result": { "ok": true }
    })
}

fn normalize_repository_scope_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "product" => Ok("product".to_string()),
        "product_area" | "product-area" | "area" | "module" => Ok("module".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported repository scope type '{other}'. Use product or product_area."
        ))),
    }
}

fn git_status_labels(status: git2::Status) -> Vec<String> {
    let mut labels = Vec::new();
    if status.is_index_new() {
        labels.push("index_new");
    }
    if status.is_index_modified() {
        labels.push("index_modified");
    }
    if status.is_index_deleted() {
        labels.push("index_deleted");
    }
    if status.is_index_renamed() {
        labels.push("index_renamed");
    }
    if status.is_wt_new() {
        labels.push("worktree_new");
    }
    if status.is_wt_modified() {
        labels.push("worktree_modified");
    }
    if status.is_wt_deleted() {
        labels.push("worktree_deleted");
    }
    if status.is_wt_renamed() {
        labels.push("worktree_renamed");
    }
    if status.is_ignored() {
        labels.push("ignored");
    }
    labels.into_iter().map(ToString::to_string).collect()
}

fn repository_git_status(local_path: &str, include_ignored: bool) -> Result<Value, AppError> {
    let repo = git2::Repository::open(local_path)?;
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|head| head.shorthand())
        .map(ToString::to_string);
    let head_sha = head
        .as_ref()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string());
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    if include_ignored {
        options.include_ignored(true);
    }
    let statuses = repo.statuses(Some(&mut options))?;
    let changed_files = statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            Some(json!({
                "path": path,
                "status": git_status_labels(entry.status())
            }))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "branch": branch,
        "headSha": head_sha,
        "dirty": !changed_files.is_empty(),
        "changedFiles": changed_files
    }))
}

fn repository_git_diff(local_path: &str, max_bytes: i64) -> Result<Value, AppError> {
    let repo = git2::Repository::open(local_path)?;
    let mut options = git2::DiffOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let diff = repo.diff_index_to_workdir(None, Some(&mut options))?;
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' | '-' | ' ' => diff_text.push(line.origin()),
            _ => {}
        }
        diff_text.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;
    let max_bytes = max_bytes.clamp(1_024, 2_000_000) as usize;
    let truncated = diff_text.len() > max_bytes;
    if truncated {
        diff_text.truncate(max_bytes);
    }
    Ok(json!({
        "diff": diff_text,
        "truncated": truncated
    }))
}

async fn get_module_by_id(pool: &sqlx::SqlitePool, module_id: &str) -> Result<Module, AppError> {
    sqlx::query_as::<_, Module>(&format!(
        "SELECT {MCP_MODULE_SELECT_COLUMNS} FROM modules WHERE id=?"
    ))
    .bind(module_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Module {module_id} not found")))
}

async fn list_capability_children(
    pool: &sqlx::SqlitePool,
    module_id: &str,
    parent_capability_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Capability>, AppError> {
    let limit = limit.clamp(1, 500);
    if let Some(parent_id) = parent_capability_id {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE module_id=? AND parent_capability_id=?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(module_id)
        .bind(parent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE module_id=? AND parent_capability_id IS NULL
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(module_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

async fn list_capability_siblings(
    pool: &sqlx::SqlitePool,
    capability: &Capability,
    limit: i64,
) -> Result<Vec<Capability>, AppError> {
    let limit = limit.clamp(1, 100);
    if let Some(parent_id) = capability.parent_capability_id.as_deref() {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE module_id=? AND parent_capability_id=? AND id<>?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(&capability.module_id)
        .bind(parent_id)
        .bind(&capability.id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE module_id=? AND parent_capability_id IS NULL AND id<>?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(&capability.module_id)
        .bind(&capability.id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

async fn capability_ancestors(
    pool: &sqlx::SqlitePool,
    capability: &Capability,
) -> Result<Vec<Capability>, AppError> {
    let mut ancestors = Vec::new();
    let mut parent_id = capability.parent_capability_id.clone();
    while let Some(id) = parent_id {
        let parent = product_repo::get_capability(pool, &id).await?;
        parent_id = parent.parent_capability_id.clone();
        ancestors.push(parent);
    }
    ancestors.reverse();
    Ok(ancestors)
}

async fn top_level_work_items_for_feature(
    pool: &sqlx::SqlitePool,
    product_id: Option<&str>,
    module_id: Option<&str>,
    feature_id: Option<&str>,
) -> Result<Vec<WorkItem>, AppError> {
    let Some(feature_id) = feature_id else {
        return Ok(Vec::new());
    };
    let mut items = work_item_repo::list_work_items(
        pool,
        product_id,
        module_id,
        Some(feature_id),
        Some(feature_id),
        Some("capability"),
        None,
    )
    .await?;
    items.retain(|item| item.parent_work_item_id.is_none());
    Ok(items)
}

async fn work_item_parent_chain(
    pool: &sqlx::SqlitePool,
    work_item: &WorkItem,
) -> Result<Vec<WorkItem>, AppError> {
    let mut parents = Vec::new();
    let mut parent_id = work_item.parent_work_item_id.clone();
    while let Some(id) = parent_id {
        let parent = work_item_repo::get_work_item(pool, &id).await?;
        parent_id = parent.parent_work_item_id.clone();
        parents.push(parent);
    }
    parents.reverse();
    Ok(parents)
}

async fn work_item_siblings(
    pool: &sqlx::SqlitePool,
    work_item: &WorkItem,
    limit: i64,
) -> Result<Vec<WorkItem>, AppError> {
    let limit = limit.clamp(1, 100) as usize;
    let mut siblings = if let Some(parent_id) = work_item.parent_work_item_id.as_deref() {
        work_item_repo::get_sub_work_items(pool, parent_id).await?
    } else {
        work_item_repo::list_work_items(
            pool,
            work_item.product_id.as_deref(),
            work_item.module_id.as_deref(),
            work_item.capability_id.as_deref(),
            work_item.source_node_id.as_deref(),
            work_item
                .source_node_type
                .as_ref()
                .map(|source_type| source_type.to_string())
                .as_deref(),
            None,
        )
        .await?
        .into_iter()
        .filter(|item| item.parent_work_item_id.is_none())
        .collect()
    };
    siblings.retain(|item| item.id != work_item.id);
    siblings.truncate(limit);
    Ok(siblings)
}

async fn build_feature_context(
    state: &AppState,
    product_id: Option<String>,
    feature_id: Option<String>,
    work_item_id: Option<String>,
    run_id: Option<String>,
    include_product_tree: bool,
    sibling_limit: i64,
) -> Result<Value, AppError> {
    if product_id.is_none() && feature_id.is_none() && work_item_id.is_none() {
        return Err(AppError::Validation(
            "Provide productId, featureId, or workItemId for feature context.".to_string(),
        ));
    }

    let selected_work_item = if let Some(work_item_id) = work_item_id.as_deref() {
        Some(work_item_repo::get_work_item(&state.db, work_item_id).await?)
    } else {
        None
    };

    let resolved_feature_id = feature_id
        .or_else(|| {
            selected_work_item
                .as_ref()
                .and_then(|item| item.capability_id.clone())
        })
        .or_else(|| {
            selected_work_item.as_ref().and_then(|item| {
                item.source_node_type.as_ref().and_then(|source_type| {
                    (source_type.to_string() == "capability")
                        .then(|| item.source_node_id.clone())
                        .flatten()
                })
            })
        });

    let feature = if let Some(feature_id) = resolved_feature_id.as_deref() {
        Some(product_repo::get_capability(&state.db, feature_id).await?)
    } else {
        None
    };

    let product_area = if let Some(feature) = feature.as_ref() {
        Some(get_module_by_id(&state.db, &feature.module_id).await?)
    } else if let Some(module_id) = selected_work_item
        .as_ref()
        .and_then(|item| item.module_id.as_deref())
    {
        Some(get_module_by_id(&state.db, module_id).await?)
    } else {
        None
    };

    let resolved_product_id = product_id
        .or_else(|| {
            selected_work_item
                .as_ref()
                .and_then(|item| item.product_id.clone())
        })
        .or_else(|| product_area.as_ref().map(|area| area.product_id.clone()));
    let product = if let Some(product_id) = resolved_product_id.as_deref() {
        Some(product_repo::get_product(&state.db, product_id).await?)
    } else {
        None
    };

    let ancestors = if let Some(feature) = feature.as_ref() {
        capability_ancestors(&state.db, feature).await?
    } else {
        Vec::new()
    };
    let feature_children = if let Some(feature) = feature.as_ref() {
        list_capability_children(
            &state.db,
            &feature.module_id,
            Some(&feature.id),
            sibling_limit,
        )
        .await?
    } else {
        Vec::new()
    };
    let feature_siblings = if let Some(feature) = feature.as_ref() {
        list_capability_siblings(&state.db, feature, sibling_limit).await?
    } else {
        Vec::new()
    };

    let stories = top_level_work_items_for_feature(
        &state.db,
        product.as_ref().map(|product| product.id.as_str()),
        product_area.as_ref().map(|area| area.id.as_str()),
        feature.as_ref().map(|feature| feature.id.as_str()),
    )
    .await?;
    let mut story_contexts = Vec::new();
    for story in stories {
        let children = work_item_repo::get_sub_work_items(&state.db, &story.id).await?;
        story_contexts.push(json!({
            "story": story,
            "children": children
        }));
    }

    let work_item_parents = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_parent_chain(&state.db, work_item).await?
    } else {
        Vec::new()
    };
    let work_item_siblings = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_siblings(&state.db, work_item, sibling_limit).await?
    } else {
        Vec::new()
    };
    let selected_work_item_children = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_repo::get_sub_work_items(&state.db, &work_item.id).await?
    } else {
        Vec::new()
    };

    let product_references = if let Some(product) = product.as_ref() {
        product_repo::list_product_references(&state.db, Some("product"), Some(&product.id)).await?
    } else {
        Vec::new()
    };
    let area_references = if let Some(area) = product_area.as_ref() {
        product_repo::list_product_references(&state.db, Some("product_area"), Some(&area.id))
            .await?
    } else {
        Vec::new()
    };
    let mut feature_references = Vec::new();
    if let Some(feature) = feature.as_ref() {
        feature_references.extend(
            product_repo::list_product_references(&state.db, Some("feature"), Some(&feature.id))
                .await?,
        );
        feature_references.extend(
            product_repo::list_product_references(&state.db, Some("capability"), Some(&feature.id))
                .await?,
        );
    }
    let work_item_references = if let Some(work_item) = selected_work_item.as_ref() {
        product_repo::list_product_references(&state.db, Some("delivery_item"), Some(&work_item.id))
            .await?
    } else {
        Vec::new()
    };

    let agent_work = if let (Some(run_id), Some(feature)) = (run_id.as_deref(), feature.as_ref()) {
        let item = match agent_work_repo::get_item(&state.db, run_id, &feature.id).await {
            Ok(item) => Some(item),
            Err(AppError::NotFound(_)) => None,
            Err(error) => return Err(error),
        };
        json!({
            "item": item,
            "dependencies": agent_work_repo::list_dependencies(&state.db, run_id, Some(&feature.id)).await?,
            "evidence": agent_work_repo::list_evidence(&state.db, run_id, Some(&feature.id), None, None, 50).await?
        })
    } else {
        Value::Null
    };

    let product_tree = if include_product_tree {
        if let Some(product) = product.as_ref() {
            Some(product_repo::get_product_tree(&state.db, &product.id).await?)
        } else {
            None
        }
    } else {
        None
    };

    Ok(json!({
        "product": product,
        "productArea": product_area,
        "feature": feature,
        "featureAncestors": ancestors,
        "featureChildren": feature_children,
        "featureSiblings": feature_siblings,
        "stories": story_contexts,
        "selectedWorkItem": selected_work_item,
        "selectedWorkItemParents": work_item_parents,
        "selectedWorkItemSiblings": work_item_siblings,
        "selectedWorkItemChildren": selected_work_item_children,
        "references": {
            "product": product_references,
            "productArea": area_references,
            "feature": feature_references,
            "workItem": work_item_references
        },
        "agentWork": agent_work,
        "productTree": product_tree
    }))
}

fn feature_context_markdown(context: &Value) -> Result<String, AppError> {
    let title = context
        .pointer("/feature/name")
        .and_then(Value::as_str)
        .or_else(|| {
            context
                .pointer("/selectedWorkItem/title")
                .and_then(Value::as_str)
        })
        .or_else(|| context.pointer("/product/name").and_then(Value::as_str))
        .unwrap_or("Feature Context");
    Ok(format!(
        "# {title}\n\nGenerated Aruvi implementation context.\n\n```json\n{}\n```\n",
        serde_json::to_string_pretty(context)?
    ))
}

async fn export_feature_context_to_file(
    context: &Value,
    output_path: &str,
    format: &str,
) -> Result<Value, AppError> {
    let content = match format {
        "markdown" | "md" => feature_context_markdown(context)?,
        "json" | "" => serde_json::to_string_pretty(context)?,
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported context export format '{other}'. Use json or markdown."
            )))
        }
    };
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::write(path, content.as_bytes()).await?;
    Ok(json!({
        "outputPath": output_path,
        "bytesWritten": content.len(),
        "format": format
    }))
}

async fn handle_catalog(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_product" => {
            let name = args.required_string(&["name"], "name")?;
            let description = args.string_or_default(&["description"], "")?;
            let vision = args.string_or_default(&["vision"], "")?;
            let goals = args.optional_string_list(&["goals"])?.unwrap_or_default();
            let tags = args.optional_string_list(&["tags"])?.unwrap_or_default();
            let lifecycle = args.optional_string(&["lifecycle"])?;
            let health = args.optional_string(&["health"])?;
            let owner_label = args.optional_string(&["ownerLabel", "owner_label"])?;
            let investment_status =
                args.optional_string(&["investmentStatus", "investment_status"])?;
            let roadmap = args.optional_string(&["roadmap"])?;
            let evidence = args.optional_string(&["evidence"])?;
            let product = product_repo::create_product(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &name,
                &description,
                &vision,
                &serde_json::to_string(&goals)?,
                &serde_json::to_string(&tags)?,
                lifecycle.as_deref(),
                health.as_deref(),
                owner_label.as_deref(),
                investment_status.as_deref(),
                roadmap.as_deref(),
                evidence.as_deref(),
            )
            .await?;
            action_result("create_product", product)
        }
        "get_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "get_product",
                product_repo::get_product(&state.db, &id).await?,
            )
        }
        "list_products" => {
            let hide_examples =
                settings_repo::get_bool_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true).await?;
            let mut products = product_repo::list_products(&state.db).await?;
            if hide_examples {
                products.retain(|product| !product.is_example_product());
            }
            action_result("list_products", products)
        }
        "seed_example_products" => {
            product_service::initialize_example_catalog(&state.db).await?;
            Ok(action_ok("seed_example_products"))
        }
        "update_product" => {
            let id = args.required_string(&["id"], "id")?;
            let goals = args.optional_json_array_string(&["goals"])?;
            let tags = args.optional_json_array_string(&["tags"])?;
            let product = product_repo::update_product(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["vision"])?.as_deref(),
                goals.as_deref(),
                tags.as_deref(),
                args.optional_string(&["lifecycle"])?.as_deref(),
                args.optional_string(&["health"])?.as_deref(),
                args.optional_string(&["ownerLabel", "owner_label"])?
                    .as_deref(),
                args.optional_string(&["investmentStatus", "investment_status"])?
                    .as_deref(),
                args.optional_string(&["roadmap"])?.as_deref(),
                args.optional_string(&["evidence"])?.as_deref(),
            )
            .await?;
            action_result("update_product", product)
        }
        "archive_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "archive_product",
                product_repo::archive_product(&state.db, &id).await?,
            )
        }
        action @ ("create_module" | "create_product_area") => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let name = args.required_string(&["name"], "name")?;
            let product_area = product_repo::create_module(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product_id,
                &name,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["purpose"], "")?,
                args.optional_string(&["node_kind", "nodeKind"])?.as_deref(),
                &args.string_or_default(&["explanation"], "")?,
                &args.string_or_default(&["examples"], "")?,
                &args.string_or_default(&["implementation_notes", "implementationNotes"], "")?,
                &args.string_or_default(&["test_guidance", "testGuidance"], "")?,
            )
            .await?;
            action_result(action, product_area)
        }
        action @ ("list_modules" | "list_product_areas") => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            action_result(
                action,
                product_repo::list_modules(&state.db, &product_id).await?,
            )
        }
        action @ ("update_module" | "update_product_area") => {
            let id = args.required_string(&["id"], "id")?;
            let product_area = product_repo::update_module(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["purpose"])?.as_deref(),
                args.optional_string(&["node_kind", "nodeKind"])?.as_deref(),
                args.optional_string(&["explanation"])?.as_deref(),
                args.optional_string(&["examples"])?.as_deref(),
                args.optional_string(&["implementation_notes", "implementationNotes"])?
                    .as_deref(),
                args.optional_string(&["test_guidance", "testGuidance"])?
                    .as_deref(),
            )
            .await?;
            action_result(action, product_area)
        }
        action @ ("delete_module" | "delete_product_area") => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_module(&state.db, &id).await?;
            Ok(action_ok(action))
        }
        action @ ("reorder_modules" | "reorder_product_areas") => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_modules(&state.db, &product_id, &ordered_ids).await?;
            Ok(action_ok(action))
        }
        "create_capability" => {
            let product_area_id = args.required_string(
                &["product_area_id", "productAreaId", "module_id", "moduleId"],
                "product_area_id",
            )?;
            let name = args.required_string(&["name"], "name")?;
            let capability = product_repo::create_capability(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product_area_id,
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?
                    .as_deref(),
                &name,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?,
                &args.string_or_default(&["priority"], "medium")?,
                &args.string_or_default(&["risk"], "medium")?,
                &args.string_or_default(&["technical_notes", "technicalNotes"], "")?,
                args.optional_string(&["node_kind", "nodeKind"])?.as_deref(),
                &args.string_or_default(&["explanation"], "")?,
                &args.string_or_default(&["examples"], "")?,
                &args.string_or_default(&["implementation_notes", "implementationNotes"], "")?,
                &args.string_or_default(&["test_guidance", "testGuidance"], "")?,
            )
            .await?;
            action_result("create_capability", capability)
        }
        "list_capabilities" => {
            let product_area_id = args.required_string(
                &["product_area_id", "productAreaId", "module_id", "moduleId"],
                "product_area_id",
            )?;
            action_result(
                "list_capabilities",
                product_repo::list_capabilities(&state.db, &product_area_id).await?,
            )
        }
        "update_capability" => {
            let id = args.required_string(&["id"], "id")?;
            let capability = product_repo::update_capability(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?
                    .as_deref(),
                args.optional_string(&["priority"])?.as_deref(),
                args.optional_string(&["risk"])?.as_deref(),
                args.optional_string(&["technical_notes", "technicalNotes"])?
                    .as_deref(),
                args.optional_string(&["node_kind", "nodeKind"])?.as_deref(),
                args.optional_string(&["explanation"])?.as_deref(),
                args.optional_string(&["examples"])?.as_deref(),
                args.optional_string(&["implementation_notes", "implementationNotes"])?
                    .as_deref(),
                args.optional_string(&["test_guidance", "testGuidance"])?
                    .as_deref(),
            )
            .await?;
            action_result("update_capability", capability)
        }
        "delete_capability" => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_capability(&state.db, &id).await?;
            Ok(action_ok("delete_capability"))
        }
        "reorder_capabilities" => {
            let product_area_id = args.required_string(
                &["product_area_id", "productAreaId", "module_id", "moduleId"],
                "product_area_id",
            )?;
            let parent_capability_id =
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_capabilities(
                &state.db,
                &product_area_id,
                parent_capability_id.as_deref(),
                &ordered_ids,
            )
            .await?;
            Ok(action_ok("reorder_capabilities"))
        }
        "apply_capability_template" => {
            let result = product_service::apply_semantic_template(
                &state.db,
                &args.required_string(
                    &["product_area_id", "productAreaId", "module_id", "moduleId"],
                    "product_area_id",
                )?,
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?
                    .as_deref(),
                &args.required_string(&["template_kind", "templateKind"], "template_kind")?,
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["description"], "")?,
                args.optional_string(&["priority"])?.as_deref(),
                args.optional_string(&["risk"])?.as_deref(),
                &args.string_or_default(&["explanation"], "")?,
                &args.string_or_default(&["examples"], "")?,
                &args.string_or_default(&["implementation_notes", "implementationNotes"], "")?,
                &args.string_or_default(&["test_guidance", "testGuidance"], "")?,
            )
            .await?;
            action_result("apply_capability_template", result)
        }
        "convert_capability_kind" => {
            let result = product_service::convert_capability_kind(
                &state.db,
                &args.required_string(&["id"], "id")?,
                &args.required_string(&["node_kind", "nodeKind"], "node_kind")?,
                args.optional_string(&["child_strategy", "childStrategy"])?
                    .as_deref(),
            )
            .await?;
            action_result("convert_capability_kind", result)
        }
        "get_product_tree" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let tree = product_repo::get_product_tree(&state.db, &product_id).await?;
            let mut tree_value = serde_json::to_value(tree)?;
            if let Some(product_areas) = tree_value.get("modules").cloned() {
                tree_value["productAreas"] = product_areas;
            }
            action_result("get_product_tree", tree_value)
        }
        action @ ("list_references" | "list_product_references") => {
            let scope_type = args.optional_string(&["scope_type", "scopeType"])?;
            let scope_id = args.optional_string(&["scope_id", "scopeId"])?;
            action_result(
                action,
                product_repo::list_product_references(
                    &state.db,
                    scope_type.as_deref(),
                    scope_id.as_deref(),
                )
                .await?,
            )
        }
        action @ ("create_reference" | "create_product_reference") => {
            let reference = product_repo::create_product_reference(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["scope_type", "scopeType"], "scope_type")?,
                &args.required_string(&["scope_id", "scopeId"], "scope_id")?,
                &args.required_string(&["title"], "title")?,
                &args.string_or_default(&["reference_kind", "referenceKind"], "note")?,
                &args.string_or_default(&["uri"], "")?,
                &args.string_or_default(&["content"], "")?,
            )
            .await?;
            action_result(action, reference)
        }
        action @ ("delete_reference" | "delete_product_reference") => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_product_reference(&state.db, &id).await?;
            Ok(action_ok(action))
        }
        "get_bulk_import_schema" => action_result(
            "get_bulk_import_schema",
            bulk_import_service::bulk_import_schema(),
        ),
        "submit_bulk_import" => {
            let file_path = args.required_string(&["file_path", "filePath"], "file_path")?;
            let job = bulk_import_service::submit_bulk_import(
                (*state).clone(),
                BulkImportRequest {
                    file_path,
                    format: args.optional_string(&["format"])?,
                    product_id: args.optional_string(&["product_id", "productId"])?,
                },
            )
            .await?;
            action_result("submit_bulk_import", job)
        }
        "get_bulk_import_status" => {
            let job_id = args.required_string(&["job_id", "jobId"], "job_id")?;
            action_result(
                "get_bulk_import_status",
                bulk_import_service::get_bulk_import_status(&state.db, &job_id).await?,
            )
        }
        "list_bulk_import_jobs" => action_result(
            "list_bulk_import_jobs",
            bulk_import_service::list_bulk_import_jobs(&state.db, args.optional_i64(&["limit"])?)
                .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_catalog action: {other}"
        ))),
    }
}

async fn handle_work_items(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        action @ ("create_work_item" | "create_story" | "create_task") => {
            let parent_work_item_id = if action == "create_task" {
                Some(args.required_string(
                    &[
                        "story_id",
                        "storyId",
                        "parent_work_item_id",
                        "parentWorkItemId",
                    ],
                    "story_id",
                )?)
            } else {
                args.optional_string(&["parent_work_item_id", "parentWorkItemId"])?
            };
            let product_id = match args.optional_string(&["product_id", "productId"])? {
                Some(product_id) => product_id,
                None if action == "create_task" => {
                    let story_id = parent_work_item_id
                        .as_deref()
                        .ok_or_else(|| AppError::Validation("missing story_id".to_string()))?;
                    work_item_repo::get_work_item(&state.db, story_id)
                        .await?
                        .product_id
                        .ok_or_else(|| {
                            AppError::Validation(
                                "Parent story does not have an associated product.".to_string(),
                            )
                        })?
                }
                None => return Err(AppError::Validation("missing product_id".to_string())),
            };
            let title = args.required_string(&["title"], "title")?;
            let product_area_id = args.optional_string(&[
                "product_area_id",
                "productAreaId",
                "module_id",
                "moduleId",
            ])?;
            let feature_id = args.optional_string(&[
                "feature_id",
                "featureId",
                "capability_id",
                "capabilityId",
            ])?;
            let source_node_id = args.optional_string(&["source_node_id", "sourceNodeId"])?;
            let source_node_type = args.optional_string(&["source_node_type", "sourceNodeType"])?;
            let work_item_type = if action == "create_story" {
                "story".to_string()
            } else if action == "create_task" {
                "task".to_string()
            } else {
                args.string_or_default(&["work_item_type", "workItemType"], "story")?
            };
            let work_item = work_item_repo::create_work_item(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product_id,
                product_area_id.as_deref(),
                feature_id.as_deref(),
                source_node_id.as_deref(),
                source_node_type.as_deref(),
                parent_work_item_id.as_deref(),
                &title,
                &args.string_or_default(&["problem_statement", "problemStatement"], "")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?,
                &args.string_or_default(&["constraints"], "")?,
                &work_item_type,
                &args.string_or_default(&["priority"], "medium")?,
                &args.string_or_default(&["complexity"], "medium")?,
            )
            .await?;
            action_result(action, work_item)
        }
        "get_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "get_work_item",
                work_item_repo::get_work_item(&state.db, &id).await?,
            )
        }
        "list_work_items" => action_result(
            "list_work_items",
            work_item_repo::list_work_items_page(
                &state.db,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.optional_string(&[
                    "product_area_id",
                    "productAreaId",
                    "module_id",
                    "moduleId",
                ])?
                .as_deref(),
                args.optional_string(&[
                    "feature_id",
                    "featureId",
                    "capability_id",
                    "capabilityId",
                ])?
                .as_deref(),
                args.optional_string(&["source_node_id", "sourceNodeId"])?
                    .as_deref(),
                args.optional_string(&["source_node_type", "sourceNodeType"])?
                    .as_deref(),
                args.optional_string(&["status"])?.as_deref(),
                args.optional_i64(&["limit"])?,
                args.optional_i64(&["offset"])?,
            )
            .await?,
        ),
        "summarize_work_items_by_product" => action_result(
            "summarize_work_items_by_product",
            work_item_repo::summarize_work_items_by_product(&state.db).await?,
        ),
        "update_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            let work_item = work_item_repo::update_work_item(
                &state.db,
                &id,
                args.optional_string(&["title"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&["problem_statement", "problemStatement"])?
                    .as_deref(),
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?
                    .as_deref(),
                args.optional_string(&["constraints"])?.as_deref(),
            )
            .await?;
            action_result("update_work_item", work_item)
        }
        "delete_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            work_item_repo::delete_work_item(&state.db, &id).await?;
            Ok(action_ok("delete_work_item"))
        }
        "get_sub_work_items" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "get_sub_work_items",
                work_item_repo::get_sub_work_items(&state.db, &work_item_id).await?,
            )
        }
        "reorder_work_items" => {
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            work_item_repo::reorder_work_items(&state.db, &ordered_ids).await?;
            Ok(action_ok("reorder_work_items"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_work_items action: {other}"
        ))),
    }
}

async fn handle_repositories(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "register_repository" => {
            let repository = repository_repo::create_repository(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.required_string(&["local_path", "localPath"], "local_path")?,
                &args.string_or_default(&["remote_url", "remoteUrl"], "")?,
                &args.string_or_default(&["default_branch", "defaultBranch"], "main")?,
            )
            .await?;
            action_result("register_repository", repository)
        }
        "list_repositories" => action_result(
            "list_repositories",
            repository_repo::list_repositories(&state.db).await?,
        ),
        "delete_repository" => {
            let id = args.required_string(&["id"], "id")?;
            repository_repo::delete_repository(&state.db, &id).await?;
            Ok(action_ok("delete_repository"))
        }
        "attach_repository" => {
            let attachment_id = uuid::Uuid::new_v4().to_string();
            let scope_type = normalize_repository_scope_type(
                &args.required_string(&["scope_type", "scopeType"], "scope_type")?,
            )?;
            repository_repo::attach_repository(
                &state.db,
                &attachment_id,
                &scope_type,
                &args.required_string(&["scope_id", "scopeId"], "scope_id")?,
                &args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                args.bool_or_default(&["is_default", "isDefault"], false)?,
            )
            .await?;
            Ok(json!({
                "action": "attach_repository",
                "result": {
                    "ok": true,
                    "attachment_id": attachment_id
                }
            }))
        }
        "resolve_repository_for_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "resolve_repository_for_work_item",
                repository_repo::resolve_repository_for_work_item(&state.db, &work_item_id).await?,
            )
        }
        "resolve_repository_for_scope" => action_result(
            "resolve_repository_for_scope",
            repository_repo::resolve_repository_for_scope(
                &state.db,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.optional_string(&[
                    "product_area_id",
                    "productAreaId",
                    "module_id",
                    "moduleId",
                ])?
                .as_deref(),
            )
            .await?,
        ),
        "create_local_workspace" => {
            let workspace = create_local_workspace_for_scope(
                state,
                args.optional_string(&["product_id", "productId"])?,
                args.optional_string(&[
                    "product_area_id",
                    "productAreaId",
                    "module_id",
                    "moduleId",
                ])?,
                args.optional_string(&["work_item_id", "workItemId"])?,
                args.optional_string(&["preferred_path", "preferredPath"])?,
            )
            .await?;
            action_result("create_local_workspace", workspace)
        }
        "list_repository_tree" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let max_depth = args.optional_i64(&["max_depth", "maxDepth"])?;
            let tree = crate::services::repo_service::list_repository_tree(
                &repository.local_path,
                args.bool_or_default(&["include_hidden", "includeHidden"], false)?,
                max_depth.map(|value| value.clamp(1, 32) as usize),
            )?;
            action_result("list_repository_tree", tree)
        }
        "read_repository_file" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let content = crate::services::repo_service::read_repository_file(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("read_repository_file", json!({ "content": content }))
        }
        "write_repository_file" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            crate::services::repo_service::write_repository_file(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
                &args.required_string(&["content"], "content")?,
            )?;
            Ok(action_ok("write_repository_file"))
        }
        "get_repository_file_sha256" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let sha = crate::services::repo_service::get_repository_file_sha256(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("get_repository_file_sha256", json!({ "sha256": sha }))
        }
        "apply_repository_patch" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let sha = crate::services::repo_service::apply_repository_patch(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
                &args.required_string(&["patch"], "patch")?,
                args.optional_string(&["base_sha256", "baseSha256"])?
                    .as_deref(),
            )?;
            action_result("apply_repository_patch", json!({ "sha256": sha }))
        }
        "get_git_status" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            action_result(
                "get_git_status",
                repository_git_status(
                    &repository.local_path,
                    args.bool_or_default(&["include_ignored", "includeIgnored"], false)?,
                )?,
            )
        }
        "get_git_diff" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            action_result(
                "get_git_diff",
                repository_git_diff(
                    &repository.local_path,
                    args.optional_i64(&["max_bytes", "maxBytes"])?
                        .unwrap_or(200_000),
                )?,
            )
        }
        "list_git_changed_files" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let status = repository_git_status(&repository.local_path, false)?;
            action_result(
                "list_git_changed_files",
                json!({
                    "changedFiles": status.get("changedFiles").cloned().unwrap_or_else(|| json!([]))
                }),
            )
        }
        "get_git_current_branch" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let status = repository_git_status(&repository.local_path, false)?;
            action_result(
                "get_git_current_branch",
                json!({
                    "branch": status.get("branch").cloned().unwrap_or(Value::Null),
                    "headSha": status.get("headSha").cloned().unwrap_or(Value::Null)
                }),
            )
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_repositories action: {other}"
        ))),
    }
}

async fn handle_planner(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_planner_session" => action_result(
            "create_planner_session",
            create_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "update_planner_session" => action_result(
            "update_planner_session",
            update_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "clear_planner_pending" => action_result(
            "clear_planner_pending",
            clear_planner_pending(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "submit_planner_turn" => action_result(
            "submit_planner_turn",
            submit_planner_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["user_input", "userInput"], "user_input")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        "submit_planner_voice_turn" => action_result(
            "submit_planner_voice_turn",
            submit_planner_voice_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["transcript", "user_input", "userInput"], "transcript")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        "confirm_planner_plan" => action_result(
            "confirm_planner_plan",
            confirm_planner_plan(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "rename_planner_draft_node" => action_result(
            "rename_planner_draft_node",
            rename_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
                args.required_string(&["name"], "name")?,
            )
            .await?,
        ),
        "add_planner_draft_child" => action_result(
            "add_planner_draft_child",
            add_planner_draft_child(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["parent_node_id", "parentNodeId"], "parent_node_id")?,
                args.required_string(&["child_type", "childType"], "child_type")?,
                args.required_string(&["name"], "name")?,
                args.optional_string(&["summary"])?,
            )
            .await?,
        ),
        "delete_planner_draft_node" => action_result(
            "delete_planner_draft_node",
            delete_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
            )
            .await?,
        ),
        "analyze_repository_for_planner" => action_result(
            "analyze_repository_for_planner",
            analyze_repository_for_planner(
                state.planner_service.clone(),
                &state.db,
                &state.artifact_base_path,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
                args.optional_string(&["product_id", "productId"])?,
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_planner action: {other}"
        ))),
    }
}

async fn handle_workflows(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "start_work_item_workflow" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let workflow_service = state.workflow_service.lock().await;
            let run = workflow_service
                .start_work_item_workflow(&work_item_id)
                .await?;
            action_result(
                "start_work_item_workflow",
                json!({ "workflow_run_id": run.id }),
            )
        }
        "get_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_run",
                workflow_service.get_workflow_run(&workflow_run_id).await?,
            )
        }
        "get_latest_workflow_run_for_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "get_latest_workflow_run_for_work_item",
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item_id)
                    .await?,
            )
        }
        "get_workflow_history" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_history",
                workflow_service
                    .get_workflow_history(&workflow_run_id)
                    .await?,
            )
        }
        "handle_workflow_user_action" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let action = match args.required_string(&["action"], "action")?.as_str() {
                "approve" => UserAction::Approve,
                "reject" => UserAction::Reject,
                "pause" => UserAction::Pause,
                "resume" => UserAction::Resume,
                "cancel" => UserAction::Cancel,
                other => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action: {other}"
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&workflow_run_id, action, args.optional_string(&["notes"])?)
                .await?;
            Ok(action_ok("handle_workflow_user_action"))
        }
        "advance_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service.advance_workflow(&workflow_run_id).await?;
            Ok(action_ok("advance_workflow"))
        }
        "list_agent_runs_for_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            action_result(
                "list_agent_runs_for_workflow",
                agent_repo::list_agent_runs_for_workflow(&state.db, &workflow_run_id).await?,
            )
        }
        "mark_workflow_run_failed" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            if run.current_stage != "failed" {
                workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "failed").await?;
                workflow_repo::record_stage_transition(
                    &state.db,
                    &uuid::Uuid::new_v4().to_string(),
                    &workflow_run_id,
                    &run.current_stage,
                    "failed",
                    "user_override",
                    args.optional_string(&["reason"])?
                        .as_deref()
                        .unwrap_or("Marked failed by MCP operator"),
                )
                .await?;
            }
            workflow_repo::update_workflow_lifecycle(
                &state.db,
                &workflow_run_id,
                "failed",
                args.optional_string(&["reason"])?.as_deref(),
                true,
            )
            .await?;
            Ok(action_ok("mark_workflow_run_failed"))
        }
        "restart_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            let workflow_service = state.workflow_service.lock().await;
            let next = workflow_service
                .start_work_item_workflow(&run.work_item_id)
                .await?;
            action_result(
                "restart_workflow_run",
                json!({ "workflow_run_id": next.id }),
            )
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_workflows action: {other}"
        ))),
    }
}

async fn handle_checkpoints(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "approve_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let approval = approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item_id,
                None,
                "task_approval",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item_id,
                None,
                None,
                Some("approved"),
                None,
                None,
                None,
            )
            .await?;

            let auto_start = settings_repo::get_bool_setting(
                &state.db,
                AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY,
                true,
            )
            .await?;
            if auto_start {
                let workflow_service = state.workflow_service.lock().await;
                if let Err(err) = workflow_service
                    .start_work_item_workflow(&work_item_id)
                    .await
                {
                    error!(work_item_id = %work_item_id, error = %err, "auto-start after approval failed");
                }
            }

            action_result("approve_work_item", approval)
        }
        "reject_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let approval = approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item_id,
                None,
                "task_approval",
                "rejected",
                &args.required_string(&["notes"], "notes")?,
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item_id,
                None,
                None,
                Some("draft"),
                None,
                None,
                None,
            )
            .await?;
            action_result("reject_work_item", approval)
        }
        "approve_work_item_plan" => action_result(
            "approve_work_item_plan",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "plan_approval",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?,
        ),
        "reject_work_item_plan" => action_result(
            "reject_work_item_plan",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "plan_approval",
                "rejected",
                &args.required_string(&["notes"], "notes")?,
            )
            .await?,
        ),
        "approve_work_item_test_review" => action_result(
            "approve_work_item_test_review",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "test_review",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?,
        ),
        "get_work_item_approvals" => action_result(
            "get_work_item_approvals",
            approval_repo::list_approvals(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "list_work_item_artifacts" => action_result(
            "list_work_item_artifacts",
            artifact_repo::list_work_item_artifacts(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "read_artifact_content" => {
            let artifact_id =
                args.required_string(&["artifact_id", "artifactId"], "artifact_id")?;
            let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
            let content = tokio::fs::read_to_string(&artifact.storage_path).await?;
            action_result(
                "read_artifact_content",
                json!({
                    "artifact": artifact,
                    "content": content
                }),
            )
        }
        "list_work_item_findings" => action_result(
            "list_work_item_findings",
            finding_repo::list_work_item_findings(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "get_logs" => action_result(
            "get_logs",
            observability_repo::get_logs(
                &state.db,
                args.optional_string(&["level"])?.as_deref(),
                args.optional_string(&["target"])?.as_deref(),
                args.optional_string(&["workflow_run_id", "workflowRunId"])?
                    .as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_checkpoints action: {other}"
        ))),
    }
}

async fn handle_agents(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "list_agent_definitions" => action_result(
            "list_agent_definitions",
            agent_repo::list_agent_definitions(&state.db).await?,
        ),
        "list_agent_model_bindings" => action_result(
            "list_agent_model_bindings",
            agent_repo::list_agent_model_bindings(&state.db).await?,
        ),
        "set_primary_agent_model_binding" => {
            let agent_id = args.required_string(&["agent_id", "agentId"], "agent_id")?;
            let model_id = args.required_string(&["model_id", "modelId"], "model_id")?;
            agent_repo::delete_agent_model_bindings_for_agent(&state.db, &agent_id).await?;
            let binding = agent_repo::create_agent_model_binding(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &agent_id,
                &model_id,
                1,
            )
            .await?;
            action_result("set_primary_agent_model_binding", binding)
        }
        "create_agent_definition" => {
            let agent = agent_repo::create_agent_definition(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.required_string(&["role"], "role")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["prompt_template_ref", "promptTemplateRef"], "")?,
                &args
                    .optional_json_array_string(&["allowed_tools", "allowedTools"])?
                    .unwrap_or_else(|| "[]".to_string()),
                &args
                    .optional_json_array_string(&["skill_tags", "skillTags"])?
                    .unwrap_or_else(|| "[]".to_string()),
                &args
                    .optional_json_object_string(&["boundaries"])?
                    .unwrap_or_else(|| "{}".to_string()),
                args.bool_or_default(&["enabled"], true)?,
                &args.string_or_default(&["employment_status", "employmentStatus"], "active")?,
            )
            .await?;
            action_result("create_agent_definition", agent)
        }
        "update_agent_definition" => {
            let id = args.required_string(&["id"], "id")?;
            let agent = agent_repo::update_agent_definition(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["role"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["prompt_template_ref", "promptTemplateRef"])?
                    .as_deref(),
                args.optional_json_array_string(&["allowed_tools", "allowedTools"])?
                    .as_deref(),
                args.optional_json_array_string(&["skill_tags", "skillTags"])?
                    .as_deref(),
                args.optional_json_object_string(&["boundaries"])?
                    .as_deref(),
                args.optional_bool(&["enabled"])?,
                args.optional_string(&["employment_status", "employmentStatus"])?
                    .as_deref(),
            )
            .await?;
            action_result("update_agent_definition", agent)
        }
        "delete_agent_definition" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_agent_definition(&state.db, &id).await?;
            Ok(action_ok("delete_agent_definition"))
        }
        "list_agent_teams" => action_result(
            "list_agent_teams",
            agent_repo::list_agent_teams(&state.db).await?,
        ),
        "create_agent_team" => {
            let team = agent_repo::create_agent_team(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["department"], "")?,
                &args.string_or_default(&["description"], "")?,
                args.bool_or_default(&["enabled"], true)?,
                args.optional_i32(&["max_concurrent_workflows", "maxConcurrentWorkflows"])?
                    .unwrap_or(2),
            )
            .await?;
            action_result("create_agent_team", team)
        }
        "update_agent_team" => {
            let id = args.required_string(&["id"], "id")?;
            let team = agent_repo::update_agent_team(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["department"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
                args.optional_i32(&["max_concurrent_workflows", "maxConcurrentWorkflows"])?,
            )
            .await?;
            action_result("update_agent_team", team)
        }
        "delete_agent_team" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_agent_team(&state.db, &id).await?;
            Ok(action_ok("delete_agent_team"))
        }
        "list_team_memberships" => action_result(
            "list_team_memberships",
            agent_repo::list_team_memberships(&state.db).await?,
        ),
        "add_team_member" => {
            let membership = agent_repo::add_team_member(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["agent_id", "agentId"], "agent_id")?,
                &args.string_or_default(&["title"], "")?,
                args.bool_or_default(&["is_lead", "isLead"], false)?,
            )
            .await?;
            action_result("add_team_member", membership)
        }
        "remove_team_member" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::remove_team_member(&state.db, &id).await?;
            Ok(action_ok("remove_team_member"))
        }
        "list_team_assignments" => action_result(
            "list_team_assignments",
            agent_repo::list_team_assignments(&state.db).await?,
        ),
        "assign_team_scope" => {
            let assignment = agent_repo::assign_team_scope(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["scope_type", "scopeType"], "scope_type")?,
                &args.required_string(&["scope_id", "scopeId"], "scope_id")?,
            )
            .await?;
            action_result("assign_team_scope", assignment)
        }
        "remove_team_assignment" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::remove_team_assignment(&state.db, &id).await?;
            Ok(action_ok("remove_team_assignment"))
        }
        "list_skills" => action_result("list_skills", agent_repo::list_skills(&state.db).await?),
        "create_skill" => {
            let skill = agent_repo::create_skill(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["category"], "")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["instructions"], "")?,
                args.bool_or_default(&["enabled"], true)?,
            )
            .await?;
            action_result("create_skill", skill)
        }
        "update_skill" => {
            let id = args.required_string(&["id"], "id")?;
            let skill = agent_repo::update_skill(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["category"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["instructions"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_skill", skill)
        }
        "delete_skill" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_skill(&state.db, &id).await?;
            Ok(action_ok("delete_skill"))
        }
        "list_agent_skill_links" => action_result(
            "list_agent_skill_links",
            agent_repo::list_agent_skill_links(&state.db).await?,
        ),
        "link_skill_to_agent" => {
            let link = agent_repo::link_skill_to_agent(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["agent_id", "agentId"], "agent_id")?,
                &args.required_string(&["skill_id", "skillId"], "skill_id")?,
                &args.string_or_default(&["proficiency"], "working")?,
            )
            .await?;
            action_result("link_skill_to_agent", link)
        }
        "unlink_skill_from_agent" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::unlink_skill_from_agent(&state.db, &id).await?;
            Ok(action_ok("unlink_skill_from_agent"))
        }
        "list_team_skill_links" => action_result(
            "list_team_skill_links",
            agent_repo::list_team_skill_links(&state.db).await?,
        ),
        "link_skill_to_team" => {
            let link = agent_repo::link_skill_to_team(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["skill_id", "skillId"], "skill_id")?,
            )
            .await?;
            action_result("link_skill_to_team", link)
        }
        "unlink_skill_from_team" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::unlink_skill_from_team(&state.db, &id).await?;
            Ok(action_ok("unlink_skill_from_team"))
        }
        "list_workflow_stage_policies" => action_result(
            "list_workflow_stage_policies",
            agent_repo::list_workflow_stage_policies(&state.db).await?,
        ),
        "upsert_workflow_stage_policy" => {
            let primary_roles = args
                .optional_json_array_string(&["primary_roles", "primaryRoles"])?
                .unwrap_or_else(|| "[]".to_string());
            let fallback_roles = args
                .optional_json_array_string(&["fallback_roles", "fallbackRoles"])?
                .unwrap_or_else(|| "[]".to_string());
            let policy = agent_repo::upsert_workflow_stage_policy(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["stage_name", "stageName"], "stage_name")?,
                &primary_roles,
                &fallback_roles,
                args.bool_or_default(&["coordinator_required", "coordinatorRequired"], false)?,
            )
            .await?;
            action_result("upsert_workflow_stage_policy", policy)
        }
        "delete_workflow_stage_policy" => {
            let stage_name = args.required_string(&["stage_name", "stageName"], "stage_name")?;
            agent_repo::delete_workflow_stage_policy(&state.db, &stage_name).await?;
            Ok(action_ok("delete_workflow_stage_policy"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_agents action: {other}"
        ))),
    }
}

async fn handle_agent_work(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "upsert_run" => {
            let run = agent_work_repo::upsert_run(
                &state.db,
                &args.required_string(&["id", "run_id", "runId"], "id")?,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.optional_string(&["repository_id", "repositoryId"])?
                    .as_deref(),
                &args.string_or_default(&["roadmap_hash", "roadmapHash"], "")?,
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&[
                    "last_commit_sha",
                    "lastCommitSha",
                    "last_commit",
                    "lastCommit",
                ])?
                .as_deref(),
                args.optional_string(&["current_batch_id", "currentBatchId"])?
                    .as_deref(),
                args.optional_string(&["next_action", "nextAction"])?
                    .as_deref(),
                args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
            )
            .await?;
            action_result("upsert_run", run)
        }
        "get_run" => action_result(
            "get_run",
            agent_work_repo::get_run(
                &state.db,
                &args.required_string(&["run_id", "runId", "id"], "run_id")?,
            )
            .await?,
        ),
        "list_runs" => action_result(
            "list_runs",
            agent_work_repo::list_runs(
                &state.db,
                args.optional_string(&["status"])?.as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        ),
        "get_run_summary" => action_result(
            "get_run_summary",
            agent_work_repo::get_run_summary(
                &state.db,
                &args.required_string(&["run_id", "runId", "id"], "run_id")?,
                args.optional_i64(&["event_limit", "eventLimit"])?
                    .unwrap_or(20),
            )
            .await?,
        ),
        "upsert_item" => {
            let feature_id = args.required_string(&["feature_id", "featureId"], "feature_id")?;
            let title = args
                .optional_string(&["title"])?
                .unwrap_or_else(|| feature_id.clone());
            let conflict_zones = args
                .optional_string_list(&["conflict_zones", "conflictZones"])?
                .map(|zones| json!(zones));
            let item = agent_work_repo::upsert_item(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &feature_id,
                args.optional_string(&["work_item_id", "workItemId"])?
                    .as_deref(),
                &args.string_or_default(&["module"], "")?,
                args.optional_string(&["service_or_domain", "serviceOrDomain"])?
                    .as_deref(),
                args.optional_string(&["priority"])?.as_deref(),
                args.optional_string(&["release_phase", "releasePhase"])?
                    .as_deref(),
                &title,
                &args.string_or_default(&["description"], "")?,
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["commit_sha", "commitSha"])?
                    .as_deref(),
                conflict_zones,
                args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
            )
            .await?;
            action_result("upsert_item", item)
        }
        "list_items" => action_result(
            "list_items",
            agent_work_repo::list_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
                args.optional_i64(&["offset"])?.unwrap_or(0),
            )
            .await?,
        ),
        "claim_next_item" => {
            let claim = agent_work_repo::claim_next_item(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["agent"], "agent")?,
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["selection_rule", "selectionRule"])?
                    .as_deref(),
                args.optional_i64(&["lease_seconds", "leaseSeconds"])?,
            )
            .await?;
            let claimed = claim.is_some();
            action_result(
                "claim_next_item",
                json!({
                    "claimed": claimed,
                    "claim": claim
                }),
            )
        }
        "heartbeat_item" => action_result(
            "heartbeat_item",
            agent_work_repo::heartbeat_item(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                &args.required_string(&["claim_token", "claimToken"], "claim_token")?,
                args.optional_i64(&["lease_seconds", "leaseSeconds"])?,
            )
            .await?,
        ),
        "update_item_status" => action_result(
            "update_item_status",
            agent_work_repo::update_item_status(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                &args.required_string(&["status"], "status")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["claim_token", "claimToken"])?
                    .as_deref(),
                args.optional_string(&["commit_sha", "commitSha"])?
                    .as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        ),
        "release_item_locks" => {
            agent_work_repo::release_item_locks(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                args.optional_string(&["claim_token", "claimToken"])?
                    .as_deref(),
            )
            .await?;
            Ok(action_ok("release_item_locks"))
        }
        "requeue_item" => action_result(
            "requeue_item",
            agent_work_repo::requeue_item(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        ),
        "requeue_expired_items" => action_result(
            "requeue_expired_items",
            agent_work_repo::requeue_expired_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        ),
        "list_ready_items" => action_result(
            "list_ready_items",
            agent_work_repo::list_ready_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_i64(&["limit"])?.unwrap_or(100),
                args.optional_i64(&["offset"])?.unwrap_or(0),
            )
            .await?,
        ),
        "list_active_locks" => action_result(
            "list_active_locks",
            agent_work_repo::list_active_locks(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
            )
            .await?,
        ),
        "list_conflict_zones" => action_result(
            "list_conflict_zones",
            agent_work_repo::list_conflict_zones(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
            )
            .await?,
        ),
        "inspect_conflict_zone" => action_result(
            "inspect_conflict_zone",
            agent_work_repo::inspect_conflict_zone(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["zone_key", "zoneKey"], "zone_key")?,
            )
            .await?,
        ),
        "reserve_conflict_zone" => action_result(
            "reserve_conflict_zone",
            agent_work_repo::reserve_conflict_zone(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["zone_key", "zoneKey"], "zone_key")?,
                &args.required_string(&["agent"], "agent")?,
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["feature_id", "featureId"])?
                    .as_deref(),
                args.optional_string(&["claim_token", "claimToken"])?
                    .as_deref(),
                args.optional_i64(&["lease_seconds", "leaseSeconds"])?,
            )
            .await?,
        ),
        "release_conflict_zone" => {
            agent_work_repo::release_conflict_zone(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["zone_key", "zoneKey"], "zone_key")?,
                args.optional_string(&["claim_token", "claimToken"])?
                    .as_deref(),
            )
            .await?;
            Ok(action_ok("release_conflict_zone"))
        }
        "complete_batch" => action_result(
            "complete_batch",
            agent_work_repo::complete_batch(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["batch_id", "batchId"], "batch_id")?,
                &args.required_string(&["status"], "status")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["commit_sha", "commitSha"])?
                    .as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        ),
        "upsert_dependency" => action_result(
            "upsert_dependency",
            agent_work_repo::upsert_dependency(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                &args.required_string(
                    &["depends_on_feature_id", "dependsOnFeatureId"],
                    "depends_on_feature_id",
                )?,
                args.optional_string(&["dependency_kind", "dependencyKind"])?
                    .as_deref(),
                args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
            )
            .await?,
        ),
        "delete_dependency" => {
            agent_work_repo::delete_dependency(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                &args.required_string(
                    &["depends_on_feature_id", "dependsOnFeatureId"],
                    "depends_on_feature_id",
                )?,
            )
            .await?;
            Ok(action_ok("delete_dependency"))
        }
        "list_dependencies" => action_result(
            "list_dependencies",
            agent_work_repo::list_dependencies(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["feature_id", "featureId"])?
                    .as_deref(),
            )
            .await?,
        ),
        "append_evidence" => {
            let changed_files = args
                .optional_string_list(&["changed_files", "changedFiles"])?
                .map(|values| json!(values));
            let artifact_refs = args
                .optional_string_list(&["artifact_refs", "artifactRefs"])?
                .map(|values| json!(values));
            action_result(
                "append_evidence",
                agent_work_repo::append_evidence(
                    &state.db,
                    &args.required_string(&["run_id", "runId"], "run_id")?,
                    args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                    args.optional_string(&["feature_id", "featureId"])?
                        .as_deref(),
                    args.optional_string(&["work_item_id", "workItemId"])?
                        .as_deref(),
                    args.optional_string(&["agent"])?.as_deref(),
                    &args.required_string(&["evidence_type", "evidenceType"], "evidence_type")?,
                    args.optional_string(&["command"])?.as_deref(),
                    args.optional_i64(&["exit_code", "exitCode"])?,
                    args.optional_string(&["status"])?.as_deref(),
                    &args.string_or_default(&["summary"], "")?,
                    &args.string_or_default(&["details"], "")?,
                    changed_files,
                    artifact_refs,
                    args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
                )
                .await?,
            )
        }
        "list_evidence" => action_result(
            "list_evidence",
            agent_work_repo::list_evidence(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["feature_id", "featureId"])?
                    .as_deref(),
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        ),
        "get_run_health" => action_result(
            "get_run_health",
            agent_work_repo::get_run_health(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
            )
            .await?,
        ),
        "list_agent_activity" => action_result(
            "list_agent_activity",
            agent_work_repo::list_agent_activity(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
            )
            .await?,
        ),
        "append_event" => action_result(
            "append_event",
            agent_work_repo::append_event(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["event_type", "eventType"], "event_type")?,
                args.optional_string(&["batch_id", "batchId"])?.as_deref(),
                args.optional_string(&["feature_id", "featureId"])?
                    .as_deref(),
                args.optional_string(&["work_item_id", "workItemId"])?
                    .as_deref(),
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["command"])?.as_deref(),
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&["details"])?.as_deref(),
                args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
            )
            .await?,
        ),
        "list_events" => action_result(
            "list_events",
            agent_work_repo::list_events(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_i64(&["after_id", "afterId"])?,
                args.optional_string(&["feature_id", "featureId"])?
                    .as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        ),
        "import_legacy_checkpoint" => action_result(
            "import_legacy_checkpoint",
            agent_work_repo::import_legacy_checkpoint(
                &state.db,
                &args.required_string(&["checkpoint_path", "checkpointPath"], "checkpoint_path")?,
                args.optional_string(&["run_id", "runId"])?.as_deref(),
                args.optional_string(&["source_label", "sourceLabel"])?
                    .as_deref(),
            )
            .await?,
        ),
        "materialize_catalog" => action_result(
            "materialize_catalog",
            agent_work_repo::materialize_catalog(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.bool_or_default(&["create_work_items", "createWorkItems"], true)?,
            )
            .await?,
        ),
        "link_catalog_work_items" => action_result(
            "link_catalog_work_items",
            agent_work_repo::link_catalog_work_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.bool_or_default(&["sync_statuses", "syncStatuses"], false)?,
            )
            .await?,
        ),
        "get_feature_context" => action_result(
            "get_feature_context",
            build_feature_context(
                state,
                args.optional_string(&["product_id", "productId"])?,
                args.optional_string(&["feature_id", "featureId"])?,
                args.optional_string(&["work_item_id", "workItemId"])?,
                args.optional_string(&["run_id", "runId"])?,
                args.bool_or_default(&["include_product_tree", "includeProductTree"], false)?,
                args.optional_i64(&["sibling_limit", "siblingLimit"])?
                    .unwrap_or(25),
            )
            .await?,
        ),
        "export_feature_context" => {
            let context = build_feature_context(
                state,
                args.optional_string(&["product_id", "productId"])?,
                args.optional_string(&["feature_id", "featureId"])?,
                args.optional_string(&["work_item_id", "workItemId"])?,
                args.optional_string(&["run_id", "runId"])?,
                args.bool_or_default(&["include_product_tree", "includeProductTree"], true)?,
                args.optional_i64(&["sibling_limit", "siblingLimit"])?
                    .unwrap_or(50),
            )
            .await?;
            action_result(
                "export_feature_context",
                export_feature_context_to_file(
                    &context,
                    &args.required_string(&["output_path", "outputPath"], "output_path")?,
                    &args.string_or_default(&["format"], "json")?,
                )
                .await?,
            )
        }
        "link_commit" => action_result(
            "link_commit",
            agent_work_repo::link_commit(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["batch_id", "batchId"], "batch_id")?,
                &args.required_string_list(&["feature_ids", "featureIds"], "feature_ids")?,
                &args.required_string(&["commit_sha", "commitSha"], "commit_sha")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_agent_work action: {other}"
        ))),
    }
}

async fn handle_models(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_provider" => {
            let id = uuid::Uuid::new_v4().to_string();
            let stored_secret_ref = secrets::store_provider_secret(
                &id,
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
                    .as_deref(),
            )?;
            let provider = model_repo::create_provider(
                &state.db,
                &id,
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["provider_type", "providerType"], "openai_compatible")?,
                &args.required_string(&["base_url", "baseUrl"], "base_url")?,
                stored_secret_ref.as_deref(),
            )
            .await?;
            action_result("create_provider", provider)
        }
        "list_providers" => action_result(
            "list_providers",
            model_repo::list_providers(&state.db).await?,
        ),
        "update_provider" => {
            let id = args.required_string(&["id"], "id")?;
            let stored_secret_ref = if let Some(secret_input) =
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
            {
                secrets::store_provider_secret(&id, Some(&secret_input))?
            } else {
                None
            };
            let provider = model_repo::update_provider(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["provider_type", "providerType"])?
                    .as_deref(),
                args.optional_string(&["base_url", "baseUrl"])?.as_deref(),
                stored_secret_ref.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_provider", provider)
        }
        "delete_provider" => {
            let id = args.required_string(&["id"], "id")?;
            model_repo::delete_provider(&state.db, &id).await?;
            Ok(action_ok("delete_provider"))
        }
        "create_model_definition" => {
            let model = model_repo::create_model_definition(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["provider_id", "providerId"], "provider_id")?,
                &args.required_string(&["name"], "name")?,
                args.optional_i64(&["context_window", "contextWindow"])?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
            )
            .await?;
            action_result("create_model_definition", model)
        }
        "list_model_definitions" => action_result(
            "list_model_definitions",
            model_repo::list_model_definitions(&state.db).await?,
        ),
        "update_model_definition" => {
            let id = args.required_string(&["id"], "id")?;
            let model = model_repo::update_model_definition(
                &state.db,
                &id,
                args.optional_string(&["provider_id", "providerId"])?
                    .as_deref(),
                args.optional_string(&["name"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_model_definition", model)
        }
        "delete_model_definition" => {
            let id = args.required_string(&["id"], "id")?;
            model_repo::delete_model_definition(&state.db, &id).await?;
            Ok(action_ok("delete_model_definition"))
        }
        "test_provider_connectivity" => {
            let id = args.required_string(&["id"], "id")?;
            let provider = model_repo::get_provider(&state.db, &id).await?;
            let message = if matches!(provider.provider_type, ProviderType::LocalRuntime) {
                let model_path = crate::services::speech_service::resolve_local_runtime_model_path(
                    &provider.base_url,
                )?;
                format!(
                    "Local speech runtime is configured at {}. Whisper models transcribe audio; they do not perform speech synthesis.",
                    model_path.display()
                )
            } else {
                let api_key = secrets::resolve_provider_secret(&provider)?;
                let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
                match gateway.health_check().await {
                    Ok(true) => "Connection successful".to_string(),
                    Ok(false) => "Connection failed - server responded but not healthy".to_string(),
                    Err(error) => return Err(error),
                }
            };
            action_result("test_provider_connectivity", json!({ "message": message }))
        }
        "register_local_runtime_model" => {
            let registration = upsert_local_runtime_registration(
                state,
                &args.required_string(&["provider_name", "providerName"], "provider_name")?,
                &args.required_string(&["model_name", "modelName"], "model_name")?,
                &args.required_string(&["model_path", "modelPath"], "model_path")?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                false,
            )
            .await?;
            action_result("register_local_runtime_model", registration)
        }
        "install_managed_local_model" => {
            let provider_name =
                args.required_string(&["provider_name", "providerName"], "provider_name")?;
            let model_name = args.required_string(&["model_name", "modelName"], "model_name")?;
            let download_url =
                args.required_string(&["download_url", "downloadUrl"], "download_url")?;
            let file_name = args.required_string(&["file_name", "fileName"], "file_name")?;
            let safe_dir = slugify(&provider_name);
            let models_dir = state.app_data_dir.join("models").join(safe_dir);
            tokio::fs::create_dir_all(&models_dir).await?;
            let destination_path = models_dir.join(file_name.trim());

            let mut downloaded = false;
            if !destination_path.exists() {
                let response = reqwest::get(download_url.trim()).await.map_err(|error| {
                    AppError::Provider(format!("Failed to download model: {error}"))
                })?;
                if !response.status().is_success() {
                    return Err(AppError::Provider(format!(
                        "Failed to download model: HTTP {}",
                        response.status()
                    )));
                }

                let mut file = tokio::fs::File::create(&destination_path).await?;
                let mut stream = response.bytes_stream();
                use futures_util::StreamExt;
                use tokio::io::AsyncWriteExt;

                while let Some(chunk) = stream.next().await {
                    let bytes = chunk.map_err(|error| {
                        AppError::Provider(format!("Failed to read model download stream: {error}"))
                    })?;
                    file.write_all(&bytes).await?;
                }
                file.flush().await?;
                downloaded = true;
            }

            let registration = upsert_local_runtime_registration(
                state,
                &provider_name,
                &model_name,
                destination_path.to_str().ok_or_else(|| {
                    AppError::Validation("Installed model path is not valid UTF-8".to_string())
                })?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                downloaded,
            )
            .await?;
            action_result("install_managed_local_model", registration)
        }
        "run_model_chat_completion" => {
            let provider_id =
                args.required_string(&["provider_id", "providerId"], "provider_id")?;
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let api_key = secrets::resolve_provider_secret(&provider)?;
            let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
            let model = args.required_string(&["model"], "model")?;
            let messages =
                args.required_deserialize::<Vec<ChatMessage>>(&["messages"], "messages")?;
            let temperature = args.optional_f64(&["temperature"])?;
            let max_tokens = args.optional_i64(&["max_tokens", "maxTokens"])?;
            let started = Instant::now();
            let response = match gateway
                .run_completion(CompletionRequest {
                    model: model.clone(),
                    messages: messages.clone(),
                    temperature,
                    max_tokens,
                })
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let error_message = error.to_string();
                    let call_index = model_call_repo::next_model_call_index(
                        &state.db,
                        "mcp_model_completion",
                        None,
                    )
                    .await?;
                    let call_id = uuid::Uuid::new_v4().to_string();
                    let request_messages_json = serde_json::to_string_pretty(&messages)?;
                    let snapshots = model_call_repo::write_model_call_snapshots(
                        &state.artifact_base_path,
                        &call_id,
                        Some(&request_messages_json),
                        None,
                    )
                    .await?;
                    model_call_repo::create_model_call(
                        &state.db,
                        model_call_repo::CreateModelCallParams {
                            id: &call_id,
                            source_kind: "mcp_model_completion",
                            source_id: None,
                            source_label: "MCP Model Completion",
                            workflow_run_id: None,
                            agent_run_id: None,
                            work_item_id: None,
                            product_id: None,
                            session_id: None,
                            agent_id: None,
                            stage: None,
                            provider_id: &provider.id,
                            provider_name: &provider.name,
                            provider_type: provider.provider_type.as_str(),
                            provider_base_url: &provider.base_url,
                            model_id: None,
                            model_name: &model,
                            call_index,
                            request_message_count: i64::try_from(messages.len())
                                .unwrap_or(i64::MAX),
                            prompt_chars: message_char_count(&messages),
                            response_chars: 0,
                            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                            max_tokens,
                            temperature,
                            token_count_input: None,
                            token_count_output: None,
                            duration_ms: Some(elapsed_ms(started)),
                            status: "failed",
                            error_message: Some(&error_message),
                        },
                    )
                    .await?;
                    return Err(error);
                }
            };
            let call_index =
                model_call_repo::next_model_call_index(&state.db, "mcp_model_completion", None)
                    .await?;
            let call_id = uuid::Uuid::new_v4().to_string();
            let request_messages_json = serde_json::to_string_pretty(&messages)?;
            let snapshots = model_call_repo::write_model_call_snapshots(
                &state.artifact_base_path,
                &call_id,
                Some(&request_messages_json),
                Some(&response.content),
            )
            .await?;
            model_call_repo::create_model_call(
                &state.db,
                model_call_repo::CreateModelCallParams {
                    id: &call_id,
                    source_kind: "mcp_model_completion",
                    source_id: None,
                    source_label: "MCP Model Completion",
                    workflow_run_id: None,
                    agent_run_id: None,
                    work_item_id: None,
                    product_id: None,
                    session_id: None,
                    agent_id: None,
                    stage: None,
                    provider_id: &provider.id,
                    provider_name: &provider.name,
                    provider_type: provider.provider_type.as_str(),
                    provider_base_url: &provider.base_url,
                    model_id: None,
                    model_name: &model,
                    call_index,
                    request_message_count: i64::try_from(messages.len()).unwrap_or(i64::MAX),
                    prompt_chars: message_char_count(&messages),
                    response_chars: char_count_i64(&response.content),
                    request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                    response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                    max_tokens,
                    temperature,
                    token_count_input: response.token_count_input,
                    token_count_output: response.token_count_output,
                    duration_ms: Some(elapsed_ms(started)),
                    status: "completed",
                    error_message: None,
                },
            )
            .await?;
            action_result("run_model_chat_completion", response)
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_models action: {other}"
        ))),
    }
}

async fn handle_settings(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "get_setting" => action_result(
            "get_setting",
            json!({
                "key": args.required_string(&["key"], "key")?,
                "value": settings_repo::get_setting(&state.db, &args.required_string(&["key"], "key")?).await?
            }),
        ),
        "set_setting" => {
            settings_repo::set_setting(
                &state.db,
                &args.required_string(&["key"], "key")?,
                &args.required_string(&["value"], "value")?,
            )
            .await?;
            Ok(action_ok("set_setting"))
        }
        "get_mobile_bridge_status" => action_result(
            "get_mobile_bridge_status",
            webhook_service::resolve_mobile_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_mcp_bridge_status" => action_result(
            "get_mcp_bridge_status",
            webhook_service::resolve_mcp_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_database_health" => {
            let migrations = sqlx::query_as::<_, MigrationStatus>(
                "SELECT version, description, success, datetime(installed_on, 'unixepoch') AS installed_on
                 FROM _sqlx_migrations
                 ORDER BY version ASC",
            )
            .fetch_all(&state.db)
            .await?;
            let latest_version = migrations.last().map(|migration| migration.version);
            action_result(
                "get_database_health",
                DatabaseHealth {
                    applied_migrations: migrations.len(),
                    latest_version,
                    migrations,
                },
            )
        }
        "get_active_database_path" => {
            let rows = sqlx::query("PRAGMA database_list")
                .fetch_all(&state.db)
                .await?;
            let main_path = rows
                .iter()
                .find(|row| row.get::<String, _>("name") == "main")
                .map(|row| row.get::<String, _>("file"))
                .ok_or_else(|| {
                    AppError::Internal("Unable to resolve active SQLite database path".to_string())
                })?;
            action_result("get_active_database_path", json!({ "path": main_path }))
        }
        "get_database_path_override" => {
            let override_path = state.app_data_dir.join("db_override_path.txt");
            let value = match std::fs::read_to_string(&override_path) {
                Ok(content) => {
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            action_result("get_database_path_override", json!({ "path": value }))
        }
        "set_database_path_override" => {
            let db_path = args.required_string(&["db_path", "dbPath"], "db_path")?;
            if !std::path::Path::new(&db_path).is_absolute() {
                return Err(AppError::Validation(
                    "Database path must be an absolute path".to_string(),
                ));
            }
            std::fs::write(
                state.app_data_dir.join("db_override_path.txt"),
                db_path.trim(),
            )?;
            Ok(action_ok("set_database_path_override"))
        }
        "clear_database_path_override" => {
            let override_path = state.app_data_dir.join("db_override_path.txt");
            match std::fs::remove_file(override_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Ok(action_ok("clear_database_path_override"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_settings action: {other}"
        ))),
    }
}

async fn handle_channels(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "send_twilio_whatsapp_message" => {
            channel_service::send_whatsapp_message(
                state,
                args.required_string(&["to"], "to")?,
                args.required_string(&["content"], "content")?,
            )
            .await?;
            Ok(action_ok("send_twilio_whatsapp_message"))
        }
        "start_twilio_voice_call" => {
            channel_service::start_voice_call(
                state,
                args.required_string(&["to"], "to")?,
                args.optional_string(&["initial_prompt", "initialPrompt"])?,
            )
            .await?;
            Ok(action_ok("start_twilio_voice_call"))
        }
        "route_planner_contact" => action_result(
            "route_planner_contact",
            channel_service::route_planner_contact(
                state,
                PlannerContactRequest {
                    to: args.required_string(&["to"], "to")?,
                    content: args.required_string(&["content"], "content")?,
                    preferred_channel: args
                        .optional_string(&["preferred_channel", "preferredChannel"])?,
                    allow_after_hours: args
                        .optional_bool(&["allow_after_hours", "allowAfterHours"])?,
                },
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_channels action: {other}"
        ))),
    }
}

async fn handle_speech(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "transcribe_audio" => {
            let provider_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_provider_id").await?;
            let model_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_model_name").await?;
            let provider_id = args
                .optional_string(&["provider_id", "providerId"])?
                .filter(|value| !value.trim().is_empty())
                .or(provider_setting)
                .ok_or_else(|| {
                    AppError::Validation("A speech transcription provider is required".to_string())
                })?;
            let requested_model_name = args
                .optional_string(&["model_name", "modelName"])?
                .filter(|value| !value.trim().is_empty())
                .or(model_setting);
            let provider_models = model_repo::list_model_definitions(&state.db)
                .await?
                .into_iter()
                .filter(|model| model.provider_id == provider_id);
            let speech_models = provider_models
                .filter(model_supports_transcription)
                .collect::<Vec<_>>();
            let model_name = if let Some(model_name) = requested_model_name {
                let known_model = speech_models.iter().any(|model| model.name == model_name);
                if known_model || looks_like_transcription_model(&model_name) {
                    model_name
                } else {
                    return Err(AppError::Validation(format!(
                        "Configured speech model '{}' does not look like a transcription model for this provider. Choose a Whisper/STT model in Settings.",
                        model_name
                    )));
                }
            } else if let Some(model) = speech_models.first() {
                model.name.clone()
            } else {
                "whisper-1".to_string()
            };
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let transcript = transcribe_audio_with_provider(
                &provider,
                &model_name,
                crate::services::speech_service::SpeechToTextRequest {
                    audio_bytes_base64: args.required_string(
                        &["audio_bytes_base64", "audioBytesBase64"],
                        "audio_bytes_base64",
                    )?,
                    mime_type: args.required_string(&["mime_type", "mimeType"], "mime_type")?,
                    locale: args
                        .optional_string(&["locale"])?
                        .or(settings_repo::get_setting(&state.db, "speech.locale").await?),
                },
            )
            .await?;
            action_result("transcribe_audio", transcript)
        }
        "speak_text_natively" => {
            let voice = args
                .optional_string(&["voice"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.native_voice").await?);
            let locale = args
                .optional_string(&["locale"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.locale").await?);
            speak_text_natively(TextToSpeechRequest {
                text: args.required_string(&["text"], "text")?,
                voice,
                locale,
            })?;
            Ok(action_ok("speak_text_natively"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_speech action: {other}"
        ))),
    }
}

fn model_supports_transcription(model: &ModelDefinition) -> bool {
    model.enabled
        && (model
            .capability_tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "speech_to_text" | "transcription" | "audio"))
            || looks_like_transcription_model(&model.name))
}

fn slugify(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_include_first_class_tools_after_legacy_tools() {
        let definitions = definitions();
        let legacy_index = definitions
            .iter()
            .position(|tool| tool.name == "aruvi_catalog")
            .expect("legacy aruvi_catalog tool");
        let first_class_index = definitions
            .iter()
            .position(|tool| tool.name == "catalog.products.get_tree")
            .expect("catalog.products.get_tree");
        let first_class_tool = &definitions[first_class_index];

        assert!(legacy_index < first_class_index);
        assert_eq!(first_class_tool.title.as_deref(), Some("Get Product Tree"));
        assert_eq!(
            first_class_tool
                .input_schema
                .get("additionalProperties")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn discovery_exposes_node_kind_constraints_in_catalog_tool_schemas() {
        let definitions = definitions();
        let product_area_create_tool = definitions
            .iter()
            .find(|tool| tool.name == "catalog.product_areas.create")
            .expect("catalog.product_areas.create");
        let capability_create_tool = definitions
            .iter()
            .find(|tool| tool.name == "catalog.capabilities.create")
            .expect("catalog.capabilities.create");

        let root_kind_enum = product_area_create_tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("nodeKind"))
            .and_then(Value::as_object)
            .and_then(|node_kind| node_kind.get("enum"))
            .and_then(Value::as_array)
            .expect("product area nodeKind enum");

        assert_eq!(root_kind_enum, &vec![json!("area")]);
        assert!(product_area_create_tool
            .description
            .contains("aruvi://catalog/node-kind-constraints"));
        assert!(capability_create_tool
            .description
            .contains("Feature is the product-management leaf"));
    }

    #[test]
    fn discovery_exposes_catalog_reference_tools() {
        let definitions = definitions();
        let reference_create_tool = definitions
            .iter()
            .find(|tool| tool.name == "catalog.references.create")
            .expect("catalog.references.create");

        let scope_type_enum = reference_create_tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("scopeType"))
            .and_then(Value::as_object)
            .and_then(|scope_type| scope_type.get("enum"))
            .and_then(Value::as_array)
            .expect("reference scopeType enum");

        assert!(scope_type_enum.contains(&json!("product_area")));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.references.list"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.references.delete"));
    }

    #[test]
    fn discovery_exposes_bulk_import_tools() {
        let definitions = definitions();
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.bulk_import.schema"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.bulk_import.submit"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.bulk_import.get_status"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "catalog.bulk_import.list_jobs"));
    }

    #[test]
    fn discovery_exposes_agent_work_coordination_tools() {
        let definitions = definitions();
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "aruvi_agent_work"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.items.claim_next"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.items.heartbeat"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.items.requeue_expired"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.dependencies.upsert"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.evidence.append"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.commits.link"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.materialize_catalog"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.link_catalog_work_items"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.runs.summary"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.runs.health"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.context.get_feature"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "agent_work.context.export_feature"));
        assert!(definitions
            .iter()
            .any(|tool| tool.name == "repositories.git.status"));
    }

    #[test]
    fn translate_first_class_tool_wraps_action_payload_for_legacy_handlers() {
        let translated = translate_first_class_tool(
            "catalog.product_areas.create",
            json!({
                "productId": "product-123",
                "name": "Runtime Model"
            }),
        )
        .expect("translation should succeed")
        .expect("known first-class tool");

        assert_eq!(translated.0, "aruvi_catalog");
        assert_eq!(
            translated.1,
            json!({
                "action": "create_product_area",
                "arguments": {
                    "productId": "product-123",
                    "name": "Runtime Model"
                }
            })
        );
    }

    #[test]
    fn translate_bulk_import_first_class_tool_wraps_action_payload() {
        let translated = translate_first_class_tool(
            "catalog.bulk_import.submit",
            json!({
                "filePath": "/tmp/import.json",
                "format": "json",
                "productId": "product-123"
            }),
        )
        .expect("translation should succeed")
        .expect("known bulk import tool");

        assert_eq!(translated.0, "aruvi_catalog");
        assert_eq!(
            translated.1,
            json!({
                "action": "submit_bulk_import",
                "arguments": {
                    "filePath": "/tmp/import.json",
                    "format": "json",
                    "productId": "product-123"
                }
            })
        );
    }

    #[test]
    fn translate_agent_work_first_class_tool_wraps_action_payload() {
        let translated = translate_first_class_tool(
            "agent_work.items.claim_next",
            json!({
                "runId": "run-123",
                "agent": "agent-a"
            }),
        )
        .expect("translation should succeed")
        .expect("known first-class tool");

        assert_eq!(translated.0, "aruvi_agent_work");
        assert_eq!(
            translated.1,
            json!({
                "action": "claim_next_item",
                "arguments": {
                    "runId": "run-123",
                    "agent": "agent-a"
                }
            })
        );
    }

    #[test]
    fn translate_feature_context_tool_wraps_action_payload() {
        let translated = translate_first_class_tool(
            "agent_work.context.get_feature",
            json!({
                "featureId": "feature-123",
                "runId": "run-123"
            }),
        )
        .expect("translation should succeed")
        .expect("known context tool");

        assert_eq!(translated.0, "aruvi_agent_work");
        assert_eq!(
            translated.1,
            json!({
                "action": "get_feature_context",
                "arguments": {
                    "featureId": "feature-123",
                    "runId": "run-123"
                }
            })
        );
    }

    #[test]
    fn translate_link_catalog_work_items_tool_wraps_action_payload() {
        let translated = translate_first_class_tool(
            "agent_work.link_catalog_work_items",
            json!({
                "runId": "run-123",
                "productId": "product-123",
                "syncStatuses": true
            }),
        )
        .expect("translation should succeed")
        .expect("known link tool");

        assert_eq!(translated.0, "aruvi_agent_work");
        assert_eq!(
            translated.1,
            json!({
                "action": "link_catalog_work_items",
                "arguments": {
                    "runId": "run-123",
                    "productId": "product-123",
                    "syncStatuses": true
                }
            })
        );
    }

    #[test]
    fn translate_materialize_catalog_tool_wraps_action_payload() {
        let translated = translate_first_class_tool(
            "agent_work.materialize_catalog",
            json!({
                "runId": "run-001",
                "productId": "mayyam",
                "createWorkItems": true
            }),
        )
        .expect("translation should succeed")
        .expect("known materialization tool");

        assert_eq!(translated.0, "aruvi_agent_work");
        assert_eq!(
            translated.1,
            json!({
                "action": "materialize_catalog",
                "arguments": {
                    "runId": "run-001",
                    "productId": "mayyam",
                    "createWorkItems": true
                }
            })
        );
    }

    #[test]
    fn translate_legacy_module_tool_names_for_backward_compatibility() {
        let translated = translate_first_class_tool(
            "catalog.modules.create",
            json!({
                "productId": "product-123",
                "name": "Runtime Model"
            }),
        )
        .expect("translation should succeed")
        .expect("known compatibility tool");

        assert_eq!(translated.0, "aruvi_catalog");
        assert_eq!(
            translated.1,
            json!({
                "action": "create_module",
                "arguments": {
                    "productId": "product-123",
                    "name": "Runtime Model"
                }
            })
        );
    }

    #[test]
    fn translate_first_class_tool_rejects_non_object_arguments() {
        let error = translate_first_class_tool("work_items.list", json!("bad payload"))
            .expect_err("translation should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert_eq!(
            error.to_string(),
            "Validation error: work_items.list arguments must be a JSON object"
        );
    }
}

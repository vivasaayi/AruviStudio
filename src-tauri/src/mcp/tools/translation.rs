use crate::error::AppError;
use serde_json::{json, Map, Value};

pub(super) fn is_legacy_tool_name(tool_name: &str) -> bool {
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

pub(super) fn translate_first_class_tool(
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

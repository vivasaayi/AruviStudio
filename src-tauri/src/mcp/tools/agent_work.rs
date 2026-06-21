use crate::error::AppError;
use crate::persistence::agent_work_repo;
use crate::state::AppState;
use serde_json::{json, Value};

use super::action_args::ToolAction;
use super::feature_context::{build_feature_context, export_feature_context_to_file};
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
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
                &args.string_or_default(&["product_area"], "")?,
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

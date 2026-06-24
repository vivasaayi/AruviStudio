use crate::error::AppError;
use crate::persistence::agent_work_repo;
use crate::state::AppState;
use serde_json::{json, Value};

use super::action_args::ToolAction;
use super::agent_work_lifecycle::handle_lifecycle_action;
use super::feature_context::{build_feature_context, export_feature_context_to_file};
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();
    if let Some(result) = handle_lifecycle_action(state, tool_action.action.as_str(), &args).await?
    {
        return Ok(result);
    }

    match tool_action.action.as_str() {
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
        "reserve_conflict_zone" => action_result("reserve_conflict_zone", {
            let run_id = args.required_string(&["run_id", "runId"], "run_id")?;
            let zone_key = args.required_string(&["zone_key", "zoneKey"], "zone_key")?;
            let agent = args.required_string(&["agent"], "agent")?;
            let batch_id = args.optional_string(&["batch_id", "batchId"])?;
            let feature_id = args.optional_string(&["feature_id", "featureId"])?;
            let claim_token = args.optional_string(&["claim_token", "claimToken"])?;
            agent_work_repo::reserve_conflict_zone(
                &state.db,
                agent_work_repo::ReserveConflictZoneInput {
                    run_id: &run_id,
                    zone_key: &zone_key,
                    agent: &agent,
                    batch_id: batch_id.as_deref(),
                    feature_id: feature_id.as_deref(),
                    claim_token: claim_token.as_deref(),
                    lease_seconds: args.optional_i64(&["lease_seconds", "leaseSeconds"])?,
                },
            )
            .await?
        }),
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
            let run_id = args.required_string(&["run_id", "runId"], "run_id")?;
            let batch_id = args.optional_string(&["batch_id", "batchId"])?;
            let feature_id = args.optional_string(&["feature_id", "featureId"])?;
            let work_item_id = args.optional_string(&["work_item_id", "workItemId"])?;
            let agent = args.optional_string(&["agent"])?;
            let evidence_type =
                args.required_string(&["evidence_type", "evidenceType"], "evidence_type")?;
            let command = args.optional_string(&["command"])?;
            let status = args.optional_string(&["status"])?;
            let summary = args.string_or_default(&["summary"], "")?;
            let details = args.string_or_default(&["details"], "")?;
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
                    agent_work_repo::AppendAgentWorkEvidenceInput {
                        run_id: &run_id,
                        batch_id: batch_id.as_deref(),
                        feature_id: feature_id.as_deref(),
                        work_item_id: work_item_id.as_deref(),
                        agent: agent.as_deref(),
                        evidence_type: &evidence_type,
                        command: command.as_deref(),
                        exit_code: args.optional_i64(&["exit_code", "exitCode"])?,
                        status: status.as_deref(),
                        summary: &summary,
                        details: &details,
                        changed_files,
                        artifact_refs,
                        metadata: args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
                    },
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
        "append_event" => {
            let run_id = args.required_string(&["run_id", "runId"], "run_id")?;
            let event_type = args.required_string(&["event_type", "eventType"], "event_type")?;
            let batch_id = args.optional_string(&["batch_id", "batchId"])?;
            let feature_id = args.optional_string(&["feature_id", "featureId"])?;
            let work_item_id = args.optional_string(&["work_item_id", "workItemId"])?;
            let agent = args.optional_string(&["agent"])?;
            let command = args.optional_string(&["command"])?;
            let status = args.optional_string(&["status"])?;
            let details = args.optional_string(&["details"])?;
            action_result(
                "append_event",
                agent_work_repo::append_event(
                    &state.db,
                    agent_work_repo::AppendAgentWorkEventInput {
                        run_id: &run_id,
                        event_type: &event_type,
                        batch_id: batch_id.as_deref(),
                        feature_id: feature_id.as_deref(),
                        work_item_id: work_item_id.as_deref(),
                        agent: agent.as_deref(),
                        command: command.as_deref(),
                        status: status.as_deref(),
                        details: details.as_deref(),
                        metadata: args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
                    },
                )
                .await?,
            )
        }
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

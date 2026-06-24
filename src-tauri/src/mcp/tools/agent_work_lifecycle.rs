use crate::error::AppError;
use crate::persistence::agent_work_repo;
use crate::state::AppState;
use serde_json::{json, Value};

use super::action_args::ActionArgs;
use super::{action_ok, action_result};

pub(super) async fn handle_lifecycle_action(
    state: &AppState,
    action: &str,
    args: &ActionArgs<'_>,
) -> Result<Option<Value>, AppError> {
    let result = match action {
        "upsert_run" => {
            let id = args.required_string(&["id", "run_id", "runId"], "id")?;
            let product_id = args.optional_string(&["product_id", "productId"])?;
            let repository_id = args.optional_string(&["repository_id", "repositoryId"])?;
            let roadmap_hash = args.string_or_default(&["roadmap_hash", "roadmapHash"], "")?;
            let status = args.optional_string(&["status"])?;
            let last_commit_sha = args.optional_string(&[
                "last_commit_sha",
                "lastCommitSha",
                "last_commit",
                "lastCommit",
            ])?;
            let current_batch_id = args.optional_string(&["current_batch_id", "currentBatchId"])?;
            let next_action = args.optional_string(&["next_action", "nextAction"])?;
            let run = agent_work_repo::upsert_run(
                &state.db,
                agent_work_repo::UpsertAgentWorkRunInput {
                    id: &id,
                    product_id: product_id.as_deref(),
                    repository_id: repository_id.as_deref(),
                    roadmap_hash: &roadmap_hash,
                    status: status.as_deref(),
                    last_commit_sha: last_commit_sha.as_deref(),
                    current_batch_id: current_batch_id.as_deref(),
                    next_action: next_action.as_deref(),
                    metadata: args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
                },
            )
            .await?;
            action_result("upsert_run", run)?
        }
        "get_run" => action_result(
            "get_run",
            agent_work_repo::get_run(
                &state.db,
                &args.required_string(&["run_id", "runId", "id"], "run_id")?,
            )
            .await?,
        )?,
        "list_runs" => action_result(
            "list_runs",
            agent_work_repo::list_runs(
                &state.db,
                args.optional_string(&["status"])?.as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        )?,
        "get_run_summary" => action_result(
            "get_run_summary",
            agent_work_repo::get_run_summary(
                &state.db,
                &args.required_string(&["run_id", "runId", "id"], "run_id")?,
                args.optional_i64(&["event_limit", "eventLimit"])?
                    .unwrap_or(20),
            )
            .await?,
        )?,
        "upsert_item" => {
            let run_id = args.required_string(&["run_id", "runId"], "run_id")?;
            let feature_id = args.required_string(&["feature_id", "featureId"], "feature_id")?;
            let work_item_id = args.optional_string(&["work_item_id", "workItemId"])?;
            let product_area = args.string_or_default(&["product_area"], "")?;
            let service_or_domain =
                args.optional_string(&["service_or_domain", "serviceOrDomain"])?;
            let priority = args.optional_string(&["priority"])?;
            let release_phase = args.optional_string(&["release_phase", "releasePhase"])?;
            let title = args
                .optional_string(&["title"])?
                .unwrap_or_else(|| feature_id.clone());
            let description = args.string_or_default(&["description"], "")?;
            let status = args.optional_string(&["status"])?;
            let batch_id = args.optional_string(&["batch_id", "batchId"])?;
            let agent = args.optional_string(&["agent"])?;
            let commit_sha = args.optional_string(&["commit_sha", "commitSha"])?;
            let conflict_zones = args
                .optional_string_list(&["conflict_zones", "conflictZones"])?
                .map(|zones| json!(zones));
            let item = agent_work_repo::upsert_item(
                &state.db,
                agent_work_repo::UpsertAgentWorkItemInput {
                    run_id: &run_id,
                    feature_id: &feature_id,
                    work_item_id: work_item_id.as_deref(),
                    product_area: &product_area,
                    service_or_domain: service_or_domain.as_deref(),
                    priority: priority.as_deref(),
                    release_phase: release_phase.as_deref(),
                    title: &title,
                    description: &description,
                    status: status.as_deref(),
                    batch_id: batch_id.as_deref(),
                    agent: agent.as_deref(),
                    commit_sha: commit_sha.as_deref(),
                    conflict_zones,
                    metadata: args.optional_deserialize::<Value>(&["metadata"], "metadata")?,
                },
            )
            .await?;
            action_result("upsert_item", item)?
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
        )?,
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
            )?
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
        )?,
        "update_item_status" => {
            let run_id = args.required_string(&["run_id", "runId"], "run_id")?;
            let feature_id = args.required_string(&["feature_id", "featureId"], "feature_id")?;
            let status = args.required_string(&["status"], "status")?;
            let agent = args.optional_string(&["agent"])?;
            let batch_id = args.optional_string(&["batch_id", "batchId"])?;
            let claim_token = args.optional_string(&["claim_token", "claimToken"])?;
            let commit_sha = args.optional_string(&["commit_sha", "commitSha"])?;
            let details = args.optional_string(&["details"])?;
            action_result(
                "update_item_status",
                agent_work_repo::update_item_status(
                    &state.db,
                    agent_work_repo::UpdateAgentWorkItemStatusInput {
                        run_id: &run_id,
                        feature_id: &feature_id,
                        status: &status,
                        agent: agent.as_deref(),
                        batch_id: batch_id.as_deref(),
                        claim_token: claim_token.as_deref(),
                        commit_sha: commit_sha.as_deref(),
                        details: details.as_deref(),
                    },
                )
                .await?,
            )?
        }
        "release_item_locks" => {
            agent_work_repo::release_item_locks(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                &args.required_string(&["feature_id", "featureId"], "feature_id")?,
                args.optional_string(&["claim_token", "claimToken"])?
                    .as_deref(),
            )
            .await?;
            action_ok("release_item_locks")
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
        )?,
        "requeue_expired_items" => action_result(
            "requeue_expired_items",
            agent_work_repo::requeue_expired_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_string(&["agent"])?.as_deref(),
                args.optional_string(&["details"])?.as_deref(),
            )
            .await?,
        )?,
        "list_ready_items" => action_result(
            "list_ready_items",
            agent_work_repo::list_ready_items(
                &state.db,
                &args.required_string(&["run_id", "runId"], "run_id")?,
                args.optional_i64(&["limit"])?.unwrap_or(100),
                args.optional_i64(&["offset"])?.unwrap_or(0),
            )
            .await?,
        )?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}

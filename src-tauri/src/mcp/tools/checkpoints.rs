use crate::error::AppError;
use crate::persistence::{
    approval_repo, artifact_repo, finding_repo, observability_repo, settings_repo,
};
use crate::state::AppState;
use serde_json::{json, Value};
use tracing::error;

use super::action_args::ToolAction;
use super::action_result;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        action @ ("approve_work_items" | "reject_work_items") => {
            let approved = action == "approve_work_items";
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let items: Vec<crate::services::approval_service::BatchDecisionItem> = args.required_deserialize(&["items"], "items")?;
            let notes = args.optional_string(&["notes"])?.unwrap_or_default();
            let auto_start = if approved { settings_repo::get_bool_setting(&state.db, AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY, true).await? } else { false };
            let approvals = crate::services::approval_service::record_batch_task_decisions(&state.db, &product_id, approved, &notes, &items).await?;
            if auto_start { crate::services::approval_service::schedule_batch_workflows(state, items.into_iter().map(|item| item.id).collect()); }
            action_result(action, json!({"approvals":approvals,"auto_start_requested":auto_start}))
        }
        "approve_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let approval = crate::services::approval_service::record_task_decision(&state.db, &work_item_id, true, &args.optional_string(&["notes"])?.unwrap_or_default(), None).await?;

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
            let approval = crate::services::approval_service::record_task_decision(&state.db, &work_item_id, false, &args.required_string(&["notes"], "notes")?, None).await?;
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

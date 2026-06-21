use crate::error::AppError;
use crate::persistence::{
    approval_repo, artifact_repo, finding_repo, observability_repo, settings_repo, work_item_repo,
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
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item_id,
                    status: Some("approved"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
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
                work_item_repo::UpdateWorkItemPatch {
                    id: &work_item_id,
                    status: Some("draft"),
                    title: None,
                    description: None,
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
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

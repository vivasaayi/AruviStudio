use crate::error::AppError;
use serde_json::{json, Value};
use super::action_args::ToolAction;
use super::action_result;

pub(super) async fn handle(state: &crate::state::AppState, payload: Value) -> Result<Value, AppError> {
    let parsed = ToolAction::parse(payload)?;
    let action = &parsed.action;
    let args = parsed.args();
    match action.as_str() {
        "list_feedback" | "create_feedback" => {
            let id = args.optional_i64(&["id"])?.filter(|id| *id > 0).ok_or_else(|| AppError::Validation("A positive run ID is required".into()))?;
            if action == "list_feedback" { return action_result(action, crate::commands::ci_commands::list_feedback(state, id).await.map_err(AppError::Validation)?); }
            let verdict = args.required_string(&["verdict"], "verdict")?;
            let notes = args.string_or_default(&["notes"], "")?;
            return action_result(action, crate::commands::ci_commands::record_verified_feedback(state, id, verdict, notes).await.map_err(AppError::Validation)?);
        },
        "list_branches" => return action_result(action, crate::commands::ci_commands::list_repository_branches(state, &args.required_string(&["repository_id"], "repository_id")?).await.map_err(AppError::Validation)?),
        "list_commits" => return action_result(action, crate::commands::ci_commands::list_repository_commits(state, &args.required_string(&["repository_id"], "repository_id")?).await.map_err(AppError::Validation)?),
        "list_bindings" => return action_result(action, crate::commands::ci_commands::list_target_bindings(state).await.map_err(AppError::Validation)?),
        "save_binding" => {
            let binding = crate::commands::ci_commands::SaveCiTargetBinding {
                product_id: args.required_string(&["product_id"], "product_id")?,
                repository_id: args.required_string(&["repository_id"], "repository_id")?,
                target_id: args.required_string(&["target_id"], "target_id")?,
                auto_preview: args.bool_or_default(&["auto_preview"], false)?,
            };
            return action_result(action, crate::commands::ci_commands::save_target_binding(state, binding).await.map_err(AppError::Validation)?);
        }
        "delete_binding" => {
            crate::commands::ci_commands::delete_target_binding(state, args.required_string(&["id"], "id")?).await.map_err(AppError::Validation)?;
            return action_result(action, json!({"ok":true}));
        }
        "queue_build" => {
            let result = crate::commands::ci_commands::queue_target_reference(state,
                args.required_string(&["target_id"], "target_id")?, args.required_string(&["repository_id"], "repository_id")?,
                args.optional_string(&["branch"])? , args.optional_string(&["commit"])? , args.required_string(&["product_id"], "product_id")?).await.map_err(AppError::Validation)?;
            return action_result(action, result);
        }
        _ => {}
    }
    let payload = match action.as_str() {
        "get_status" => json!({"method":"health"}),
        "list_targets" => json!({"method":"targets"}),
        "list_runs" => json!({"method":"runs"}),
        "get_run" | "get_logs" | "cancel_run" | "retry_run" | "artifact_readiness" => {
            let id = args.optional_i64(&["id"])?.filter(|id| *id > 0).ok_or_else(|| AppError::Validation("A positive run ID is required".into()))?;
            match action.as_str() {
                "get_run" => json!({"method":"run","id":id}),
                "artifact_readiness" => json!({"method":"promotion_readiness","id":id}),
                "get_logs" => {
                    let stage = args.required_string(&["stage"], "stage")?;
                    let offset = args.optional_i64(&["offset"])?.unwrap_or(0);
                    if stage.trim().is_empty() || stage.len() > 256 || offset < 0 { return Err(AppError::Validation("Invalid stage or offset".into())); }
                    json!({"method":"logs","id":id,"stage":stage,"offset":offset})
                },
                "cancel_run" => json!({"method":"cancel","id":id}),
                _ => json!({"method":"retry","id":id,"key":format!("studio-retry-{id}-{}", uuid::Uuid::new_v4())}),
            }
        },
        _ => return Err(AppError::Validation(format!("Unknown CI action: {action}"))),
    };
    let result = tokio::task::spawn_blocking(move || crate::commands::ci_commands::request(payload))
        .await.map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(AppError::Internal)?;
    action_result(action, result)
}

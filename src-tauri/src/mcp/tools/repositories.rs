use crate::commands::repository_commands::create_local_workspace_for_scope;
use crate::error::AppError;
use crate::persistence::repository_repo;
use crate::services::repo_service;
use crate::state::AppState;
use serde_json::{json, Value};

use super::action_args::ToolAction;
use super::repository_helpers::{
    normalize_repository_scope_type, repository_git_diff, repository_git_status,
};
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
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
                args.optional_string(&["product_area_id", "productAreaId"])?
                    .as_deref(),
            )
            .await?,
        ),
        "create_local_workspace" => {
            let workspace = create_local_workspace_for_scope(
                state,
                args.optional_string(&["product_id", "productId"])?,
                args.optional_string(&["product_area_id", "productAreaId"])?,
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
            let tree = repo_service::list_repository_tree(
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
            let content = repo_service::read_repository_file(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("read_repository_file", json!({ "content": content }))
        }
        "write_repository_file" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            repo_service::write_repository_file(
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
            let sha = repo_service::get_repository_file_sha256(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("get_repository_file_sha256", json!({ "sha256": sha }))
        }
        "apply_repository_patch" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let sha = repo_service::apply_repository_patch(
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

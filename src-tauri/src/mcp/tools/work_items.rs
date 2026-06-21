use crate::error::AppError;
use crate::persistence::work_item_repo;
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
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
            let product_area_id = args.optional_string(&["product_area_id", "productAreaId"])?;
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
                args.optional_string(&["product_area_id", "productAreaId"])?
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

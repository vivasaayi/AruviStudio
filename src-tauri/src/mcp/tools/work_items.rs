use crate::error::AppError;
use crate::persistence::work_item_repo;
use crate::state::AppState;
use serde_json::{json, Value};

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
            let id = uuid::Uuid::new_v4().to_string();
            let problem_statement =
                args.string_or_default(&["problem_statement", "problemStatement"], "")?;
            let description = args.string_or_default(&["description"], "")?;
            let acceptance_criteria =
                args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?;
            let constraints = args.string_or_default(&["constraints"], "")?;
            let priority = args.string_or_default(&["priority"], "medium")?;
            let complexity = args.string_or_default(&["complexity"], "medium")?;
            let work_item = work_item_repo::create_work_item(
                &state.db,
                work_item_repo::CreateWorkItemInput {
                    id: &id,
                    product_id: &product_id,
                    product_area_id: product_area_id.as_deref(),
                    capability_id: feature_id.as_deref(),
                    source_node_id: source_node_id.as_deref(),
                    source_node_type: source_node_type.as_deref(),
                    parent_work_item_id: parent_work_item_id.as_deref(),
                    title: &title,
                    problem_statement: &problem_statement,
                    description: &description,
                    acceptance_criteria: &acceptance_criteria,
                    constraints: &constraints,
                    work_item_type: &work_item_type,
                    priority: &priority,
                    complexity: &complexity,
                },
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
        "list_work_items" => {
            let product_id = args.optional_string(&["product_id", "productId"])?;
            let product_area_id = args.optional_string(&["product_area_id", "productAreaId"])?;
            let capability_id = args.optional_string(&[
                "feature_id",
                "featureId",
                "capability_id",
                "capabilityId",
            ])?;
            let source_node_id = args.optional_string(&["source_node_id", "sourceNodeId"])?;
            let source_node_type = args.optional_string(&["source_node_type", "sourceNodeType"])?;
            let status = args.optional_string(&["status"])?;
            let include_pagination =
                args.bool_or_default(&["include_pagination", "includePagination"], false)?;
            let query = work_item_repo::WorkItemListQuery {
                product_id: product_id.as_deref(),
                product_area_id: product_area_id.as_deref(),
                capability_id: capability_id.as_deref(),
                source_node_id: source_node_id.as_deref(),
                source_node_type: source_node_type.as_deref(),
                status: status.as_deref(),
                limit: args.optional_i64(&["limit"])?,
                offset: args.optional_i64(&["offset"])?,
            };
            if include_pagination {
                let page =
                    work_item_repo::list_work_items_page_with_metadata(&state.db, query).await?;
                let returned = page.items.len();
                let next_offset = page.has_more.then_some(page.offset + page.limit);
                action_result(
                    "list_work_items",
                    json!({
                        "workItems": page.items,
                        "pagination": {
                            "limit": page.limit,
                            "offset": page.offset,
                            "returned": returned,
                            "hasMore": page.has_more,
                            "nextOffset": next_offset,
                        }
                    }),
                )
            } else {
                action_result(
                    "list_work_items",
                    work_item_repo::list_work_items_page(&state.db, query).await?,
                )
            }
        }
        "summarize_work_items_by_product" => action_result(
            "summarize_work_items_by_product",
            work_item_repo::summarize_work_items_by_product(&state.db).await?,
        ),
        "update_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            let title = args.optional_string(&["title"])?;
            let description = args.optional_string(&["description"])?;
            let status = args.optional_string(&["status"])?;
            let problem_statement =
                args.optional_string(&["problem_statement", "problemStatement"])?;
            let acceptance_criteria =
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?;
            let constraints = args.optional_string(&["constraints"])?;
            let work_item = work_item_repo::update_work_item(
                &state.db,
                work_item_repo::UpdateWorkItemPatch {
                    id: &id,
                    title: title.as_deref(),
                    description: description.as_deref(),
                    status: status.as_deref(),
                    problem_statement: problem_statement.as_deref(),
                    acceptance_criteria: acceptance_criteria.as_deref(),
                    constraints: constraints.as_deref(),
                },
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
                work_item_repo::get_sub_work_items_page(
                    &state.db,
                    &work_item_id,
                    args.optional_i64(&["limit"])?,
                    args.optional_i64(&["offset"])?,
                )
                .await?,
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

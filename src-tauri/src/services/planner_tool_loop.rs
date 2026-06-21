use crate::domain::product::Product;
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::providers::types::ChatMessage;
use crate::services::planner_action_parser::parse_agent_turn;
use crate::services::planner_catalog::find_product;
use crate::services::planner_model::{
    planner_system_prompt, run_completion, PlannerModelCallContext,
};
use crate::services::planner_service::{PlannerDraftPlan, PlannerPlan, PlannerTraceEvent};
use crate::services::planner_session::PlannerConversationEntry;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::Path;

const PLANNER_TOOL_WORK_ITEM_LIMIT: i64 = 200;

fn push_trace(
    trace: &mut Vec<PlannerTraceEvent>,
    stage: impl Into<String>,
    title: impl Into<String>,
    detail: impl Into<String>,
) {
    let step = trace.len() + 1;
    trace.push(PlannerTraceEvent {
        step,
        stage: stage.into(),
        title: title.into(),
        detail: detail.into(),
    });
}

async fn get_product_tree_tool(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<Value, AppError> {
    let product = find_product(db, product_name).await?;
    let tree = product_repo::get_product_tree(db, &product.id).await?;
    Ok(serde_json::to_value(tree)?)
}

async fn list_work_items_tool(
    db: &SqlitePool,
    product_name: Option<&str>,
    status: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Value, AppError> {
    let product_id = if let Some(name) = product_name {
        Some(find_product(db, Some(name)).await?.id)
    } else {
        None
    };
    let work_items = work_item_repo::list_work_items_page(
        db,
        work_item_repo::WorkItemListQuery {
            product_id: product_id.as_deref(),
            status,
            limit: Some(limit.unwrap_or(PLANNER_TOOL_WORK_ITEM_LIMIT)),
            offset,
            ..Default::default()
        },
    )
    .await?;
    Ok(serde_json::to_value(work_items)?)
}

pub(crate) struct PlannerToolLoopInput<'a> {
    pub(crate) db: &'a SqlitePool,
    pub(crate) artifact_base_path: &'a Path,
    pub(crate) session_id: &'a str,
    pub(crate) provider_id: &'a str,
    pub(crate) model_name: &'a str,
    pub(crate) conversation: &'a [PlannerConversationEntry],
    pub(crate) pending_plan: Option<&'a PlannerPlan>,
    pub(crate) draft_plan: Option<&'a PlannerDraftPlan>,
    pub(crate) selected_draft_node_id: Option<&'a str>,
    pub(crate) selected_product: Option<&'a Product>,
    pub(crate) user_input: &'a str,
}

pub(crate) async fn run_tool_loop(
    input: PlannerToolLoopInput<'_>,
    trace: &mut Vec<PlannerTraceEvent>,
) -> Result<PlannerPlan, AppError> {
    let mut messages = vec![ChatMessage {
        role: "system".to_string(),
        content: planner_system_prompt(),
    }];

    let history = input
        .conversation
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|entry| format!("{}: {}", entry.role.to_uppercase(), entry.content))
        .collect::<Vec<_>>()
        .join("\n");

    let user_context = format!(
        "Recent conversation:\n{}\n\nSelected product:\n{}\n\nCurrent pending proposal:\n{}\n\nCurrent staged design tree:\n{}\n\nSelected design node:\n{}\n\nLatest user request:\n{}",
        if history.is_empty() {
            "No prior conversation."
        } else {
            &history
        },
        input
            .selected_product
            .map(serde_json::to_string_pretty)
            .transpose()?
            .unwrap_or_else(|| "No product selected. Ask the user to select or create a product before planning.".to_string()),
        input
            .pending_plan
            .map(serde_json::to_string_pretty)
            .transpose()?
            .unwrap_or_else(|| "No pending proposal.".to_string()),
        input
            .draft_plan
            .map(serde_json::to_string_pretty)
            .transpose()?
            .unwrap_or_else(|| "No staged design yet.".to_string()),
        input
            .selected_draft_node_id
            .and_then(|node_id| input.draft_plan.and_then(|draft| draft.nodes.iter().find(|node| node.id == node_id)))
            .map(serde_json::to_string_pretty)
            .transpose()?
            .unwrap_or_else(|| "No design node selected.".to_string()),
        input.user_input
    );
    push_trace(
        trace,
        "input",
        "Planner turn context",
        format!(
            "provider={}\nmodel={}\n\n{}",
            input.provider_id, input.model_name, user_context
        ),
    );

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_context,
    });

    for step in 0..6 {
        let completion = run_completion(
            input.db,
            input.artifact_base_path,
            input.provider_id,
            input.model_name,
            messages.clone(),
            PlannerModelCallContext {
                source_kind: "desktop_planner",
                source_id: Some(input.session_id),
                source_label: "Desktop Planner",
                session_id: Some(input.session_id),
                product_id: input.selected_product.map(|product| product.id.as_str()),
            },
        )
        .await?;
        push_trace(
            trace,
            "model",
            format!("Model completion {}", step + 1),
            completion.clone(),
        );
        match parse_agent_turn(&completion)? {
            Ok(tool_call) => {
                push_trace(
                    trace,
                    "tool_call",
                    format!("Requested tool {}", tool_call.tool),
                    serde_json::to_string_pretty(&tool_call)?,
                );
                let args = tool_call.arguments.clone().unwrap_or_else(|| json!({}));
                let tool_result = match tool_call.tool.as_str() {
                    "get_product_tree" => {
                        get_product_tree_tool(
                            input.db,
                            args.get("productName").and_then(Value::as_str),
                        )
                        .await
                    }
                    "list_work_items" => {
                        list_work_items_tool(
                            input.db,
                            args.get("productName").and_then(Value::as_str),
                            args.get("status").and_then(Value::as_str),
                            args.get("limit").and_then(Value::as_i64),
                            args.get("offset").and_then(Value::as_i64),
                        )
                        .await
                    }
                    _ => Err(AppError::Validation(format!(
                        "Unsupported planner tool {}",
                        tool_call.tool
                    ))),
                };
                let tool_result = match tool_result {
                    Ok(result) => result,
                    Err(error) => json!({
                        "error": error.to_string(),
                        "tool": tool_call.tool,
                        "note": "Tool execution failed. If this refers to a proposed entity that is not created yet, continue planning using the pending proposal."
                    }),
                };
                push_trace(
                    trace,
                    "tool_result",
                    format!("Tool result {}", tool_call.tool),
                    serde_json::to_string_pretty(&tool_result)?,
                );
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: serde_json::to_string(&tool_call)?,
                });
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Tool result for {}:\n{}",
                        tool_call.tool,
                        serde_json::to_string_pretty(&tool_result)?
                    ),
                });
            }
            Err(plan) => {
                push_trace(
                    trace,
                    "plan",
                    "Parsed planner plan",
                    serde_json::to_string_pretty(&plan)?,
                );
                return Ok(plan);
            }
        }
    }

    Err(AppError::Validation(
        "Planner exceeded tool-step limit before returning a final plan".to_string(),
    ))
}

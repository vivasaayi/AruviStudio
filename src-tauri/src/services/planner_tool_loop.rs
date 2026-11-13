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

fn resolve_work_item_page(limit: Option<i64>, offset: Option<i64>) -> (i64, i64) {
    (
        limit
            .unwrap_or(PLANNER_TOOL_WORK_ITEM_LIMIT)
            .clamp(1, PLANNER_TOOL_WORK_ITEM_LIMIT),
        offset.unwrap_or(0).max(0),
    )
}

pub(crate) async fn list_work_items_tool(
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
    let (page_limit, page_offset) = resolve_work_item_page(limit, offset);
    let mut work_items = work_item_repo::list_work_items_page(
        db,
        work_item_repo::WorkItemListQuery {
            product_id: product_id.as_deref(),
            status,
            limit: Some(page_limit + 1),
            offset: Some(page_offset),
            ..Default::default()
        },
    )
    .await?;
    let has_more = work_items.len() > page_limit as usize;
    work_items.truncate(page_limit as usize);
    Ok(json!({
        "workItems": work_items,
        "pagination": {
            "limit": page_limit,
            "offset": page_offset,
            "returned": work_items.len(),
            "hasMore": has_more,
            "nextOffset": if has_more { Some(page_offset + page_limit) } else { None },
        },
    }))
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

    let mut format_retries = 0;
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
        let parsed = match parse_agent_turn(&completion) {
            Ok(parsed) => parsed,
            Err(error) if format_retries < 2 => {
                format_retries += 1;
                push_trace(
                    trace,
                    "format_retry",
                    "Invalid planner response",
                    error.to_string(),
                );
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: completion,
                });
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("Validation error: {}. Your previous answer did not follow the required JSON envelope. Return one JSON object with type final, assistant_response (string), needs_confirmation (boolean), clarification_question (string or null), and actions (array). Put each requested design action inside actions, not at the top level. Do not use Markdown.", error),
                });
                continue;
            }
            Err(error) => return Err(error),
        };
        match parsed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{db as db_service, product_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_planner_tool_loop_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ))
    }

    async fn create_test_pool(name: &str) -> SqlitePool {
        let temp_root = make_temp_dir(name);
        std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
        let db_path = temp_root.join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());
        db_service::create_pool(&database_url)
            .await
            .expect("test database should be created")
    }

    async fn create_test_product(pool: &SqlitePool, product_id: &str, name: &str) {
        product_repo::create_product(
            pool,
            product_repo::CreateProductInput {
                id: product_id,
                name,
                description: "",
                vision: "",
                goals: "[]",
                tags: "[]",
                lifecycle: Some("active"),
                health: Some("healthy"),
                owner_label: None,
                investment_status: Some("invest"),
                roadmap: None,
                evidence: None,
            },
        )
        .await
        .expect("product should be created");
    }

    async fn create_test_work_item(pool: &SqlitePool, id: &str, product_id: &str, title: &str) {
        work_item_repo::create_work_item(
            pool,
            work_item_repo::CreateWorkItemInput {
                id,
                product_id,
                product_area_id: None,
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title,
                problem_statement: "",
                description: "",
                acceptance_criteria: "",
                constraints: "",
                work_item_type: "story",
                priority: "medium",
                complexity: "medium",
            },
        )
        .await
        .expect("work item should be created");
    }

    #[tokio::test]
    async fn list_work_items_tool_returns_pagination_metadata() {
        let pool = create_test_pool("list_work_items_pagination").await;
        create_test_product(&pool, "product-planner-tool", "Planner Product").await;
        for index in 0..3 {
            create_test_work_item(
                &pool,
                &format!("work-item-{index}"),
                "product-planner-tool",
                &format!("Planner story {index}"),
            )
            .await;
        }

        let result = list_work_items_tool(&pool, Some("Planner Product"), None, Some(2), Some(0))
            .await
            .expect("tool should return work items");

        assert_eq!(
            result
                .get("workItems")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            result
                .pointer("/pagination/limit")
                .and_then(serde_json::Value::as_i64),
            Some(2)
        );
        assert_eq!(
            result
                .pointer("/pagination/offset")
                .and_then(serde_json::Value::as_i64),
            Some(0)
        );
        assert_eq!(
            result
                .pointer("/pagination/hasMore")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result
                .pointer("/pagination/nextOffset")
                .and_then(serde_json::Value::as_i64),
            Some(2)
        );
    }
    #[tokio::test]
    async fn repairs_invalid_model_envelope_and_bounds_retries() {
        use crate::persistence::model_repo;
        use axum::{extract::State, routing::post, Json, Router};
        use std::sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        };

        #[derive(Clone)]
        struct Fixture {
            count: Arc<AtomicUsize>,
            always_invalid: Arc<AtomicBool>,
        }
        async fn complete(
            State(fixture): State<Fixture>,
            Json(request): Json<Value>,
        ) -> Json<Value> {
            let count = fixture.count.fetch_add(1, Ordering::SeqCst);
            if count > 0 {
                assert!(request["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|message| message["content"]
                        .as_str()
                        .unwrap_or("")
                        .contains("previous answer did not follow")));
            }
            let content = if count == 0 || fixture.always_invalid.load(Ordering::SeqCst) {
                r#"{"type":"create_product_area","actions":[]}"#
            } else {
                r#"{"type":"final","assistant_response":"Draft prepared","needs_confirmation":false,"clarification_question":null,"actions":[{"type":"create_product_area","name":"Inventory"}]}"#
            };
            Json(
                json!({"id":"fixture", "model":"fixture", "choices":[{"message":{"role":"assistant","content":content},"finish_reason":"stop"}]}),
            )
        }
        let fixture = Fixture {
            count: Arc::new(AtomicUsize::new(0)),
            always_invalid: Arc::new(AtomicBool::new(false)),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_fixture = fixture.clone();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/v1/chat/completions", post(complete))
                    .with_state(server_fixture),
            )
            .await
            .unwrap()
        });
        let pool = create_test_pool("format_repair").await;
        model_repo::create_provider(
            &pool,
            "format-provider",
            "Fixture",
            "openai_compatible",
            &format!("http://{address}/v1"),
            None,
        )
        .await
        .unwrap();
        let artifacts = make_temp_dir("format_artifacts");
        std::fs::create_dir_all(&artifacts).unwrap();
        let input = || PlannerToolLoopInput {
            db: &pool,
            artifact_base_path: &artifacts,
            session_id: "fixture",
            provider_id: "format-provider",
            model_name: "fixture",
            conversation: &[],
            pending_plan: None,
            draft_plan: None,
            selected_draft_node_id: None,
            selected_product: None,
            user_input: "Add inventory",
        };
        let mut trace = Vec::new();
        let plan = run_tool_loop(input(), &mut trace).await.unwrap();
        assert_eq!(plan.actions[0]["name"], "Inventory");
        assert_eq!(fixture.count.load(Ordering::SeqCst), 2);
        assert!(trace.iter().any(|event| event.stage == "format_retry"));
        fixture.count.store(0, Ordering::SeqCst);
        fixture.always_invalid.store(true, Ordering::SeqCst);
        assert!(run_tool_loop(input(), &mut Vec::new()).await.is_err());
        assert_eq!(fixture.count.load(Ordering::SeqCst), 3);
        server.abort();
    }
}

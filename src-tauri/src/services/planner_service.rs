use crate::error::AppError;
use crate::persistence::{
    approval_repo, model_repo, planner_repo, product_repo, settings_repo, work_item_repo,
    workflow_repo,
};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::secrets;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerSessionInfo {
    pub session_id: String,
    pub provider_id: Option<String>,
    pub model_name: Option<String>,
    pub has_pending_plan: bool,
    pub has_draft_plan: bool,
    pub selected_draft_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerPlan {
    pub assistant_response: String,
    pub needs_confirmation: bool,
    pub clarification_question: Option<String>,
    pub actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTreeNode {
    pub id: String,
    pub label: String,
    pub meta: Option<String>,
    pub children: Vec<PlannerTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDraftNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub node_type: String,
    pub name: String,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDraftPlan {
    pub nodes: Vec<PlannerDraftNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTurnResponse {
    pub session_id: String,
    pub status: String,
    pub assistant_message: String,
    pub pending_plan: Option<PlannerPlan>,
    pub tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub draft_tree_nodes: Option<Vec<PlannerTreeNode>>,
    pub selected_draft_node_id: Option<String>,
    pub execution_lines: Vec<String>,
    pub execution_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlannerConversationEntry {
    role: String,
    content: String,
}

#[derive(Debug, Clone)]
struct PlannerSession {
    provider_id: Option<String>,
    model_name: Option<String>,
    pending_plan: Option<PlannerPlan>,
    draft_plan: Option<PlannerDraftPlan>,
    selected_draft_node_id: Option<String>,
    conversation: Vec<PlannerConversationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlannerToolCall {
    #[serde(rename = "type")]
    kind: String,
    tool: String,
    arguments: Option<Value>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlannerFinalResponse {
    #[serde(rename = "type")]
    kind: Option<String>,
    assistant_response: String,
    needs_confirmation: bool,
    clarification_question: Option<String>,
    actions: Vec<Value>,
}

pub struct PlannerService {
    sessions: HashMap<String, PlannerSession>,
}

impl PlannerService {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    fn create_session(
        &mut self,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> PlannerSessionInfo {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.sessions.insert(
            session_id.clone(),
            PlannerSession {
                provider_id: provider_id.clone(),
                model_name: model_name.clone(),
                pending_plan: None,
                draft_plan: None,
                selected_draft_node_id: None,
                conversation: vec![],
            },
        );
        PlannerSessionInfo {
            session_id,
            provider_id,
            model_name,
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        }
    }

    fn update_session(
        &mut self,
        session_id: &str,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.provider_id = provider_id.clone();
        session.model_name = model_name.clone();
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id,
            model_name,
            has_pending_plan: session.pending_plan.is_some(),
            has_draft_plan: session.draft_plan.is_some(),
            selected_draft_node_id: session.selected_draft_node_id.clone(),
        })
    }

    fn clear_pending(&mut self, session_id: &str) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.pending_plan = None;
        session.draft_plan = None;
        session.selected_draft_node_id = None;
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id: session.provider_id.clone(),
            model_name: session.model_name.clone(),
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        })
    }

    fn get_session(&self, session_id: &str) -> Result<PlannerSession, AppError> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Planner session {} not found", session_id)))
    }

    fn save_session(&mut self, session_id: &str, session: PlannerSession) {
        self.sessions.insert(session_id.to_string(), session);
    }
}

async fn load_session_from_db(
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSession, AppError> {
    let record = planner_repo::get_session(db, session_id).await?;
    let conversation = planner_repo::list_conversation_entries(db, session_id)
        .await?
        .into_iter()
        .map(|entry| PlannerConversationEntry {
            role: entry.role,
            content: entry.content,
        })
        .collect::<Vec<_>>();
    let pending_plan = match record.pending_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerPlan>(&value)?),
        None => None,
    };
    let draft_plan = match record.draft_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerDraftPlan>(&value)?),
        None => None,
    };
    Ok(PlannerSession {
        provider_id: record.provider_id,
        model_name: record.model_name,
        pending_plan,
        draft_plan,
        selected_draft_node_id: record.selected_draft_node_id,
        conversation,
    })
}

async fn persist_pending_plan(
    db: &SqlitePool,
    session_id: &str,
    pending_plan: Option<&PlannerPlan>,
) -> Result<(), AppError> {
    let serialized = pending_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_pending_plan(db, session_id, serialized.as_deref()).await?;
    Ok(())
}

async fn persist_draft_state(
    db: &SqlitePool,
    session_id: &str,
    draft_plan: Option<&PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
) -> Result<(), AppError> {
    let serialized = draft_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_draft_state(
        db,
        session_id,
        serialized.as_deref(),
        selected_draft_node_id,
    )
    .await?;
    Ok(())
}

async fn append_conversation(
    db: &SqlitePool,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), AppError> {
    planner_repo::append_conversation_entry(
        db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        role,
        content,
    )
    .await?;
    Ok(())
}

fn planner_system_prompt() -> &'static str {
    r#"You are an AI planning lead for a product-management desktop app.
You can inspect the workspace with tools before proposing changes.
You are editing a draft planning tree, not writing directly to the persisted database.
Return exactly one JSON object each turn.

If you need more context, return:
{
  "type": "tool_call",
  "tool": "list_products|get_product_tree|list_work_items",
  "arguments": {},
  "reason": "brief reason"
}

When you are done, return:
{
  "type": "final",
  "assistant_response": "brief natural-language reply",
  "needs_confirmation": false,
  "clarification_question": null,
  "actions": []
}

Rules:
- Output valid JSON only. No markdown.
- Behave conversationally. First reason about what already exists in the supplied context, then suggest what should be added, changed, or removed from the draft.
- If the user is exploring or describing a need, prefer proposing actions rather than assuming immediate execution.
- If the user asks for a detailed plan, architecture, modules, capabilities, or work items, prefer returning a single comprehensive proposal with all relevant create_* actions in one response instead of asking to create only the top-level product first.
- If an entity already seems to exist, do not suggest creating a duplicate unless the user explicitly asks for a separate one.
- For draft edits, set needs_confirmation=false. Confirmation is only for committing the whole draft later.
- Only use needs_confirmation=true if you are asking for final persistence or another risky action.
- If the request is ambiguous, set actions=[] and put the missing detail in clarification_question.
- Use tools when the request depends on current repo state or structure instead of guessing from the prompt alone.
- If a tool reports that a proposed entity does not exist yet, treat that as expected for proposal refinement and continue planning against the pending proposal instead of failing.
- Do not call mutation tools. Draft edits go in final.actions.
- After receiving tool results, continue reasoning and either call another tool or return type=final.
- Use these action types only:
create_product, update_product, archive_product,
create_module, update_module, delete_module,
create_capability, update_capability, delete_capability,
create_work_item, update_work_item, delete_work_item,
approve_work_item, reject_work_item, approve_work_item_plan, reject_work_item_plan, approve_work_item_test_review,
start_workflow, workflow_action, report_status, report_tree.
- Use product/module/capability/work item names in target fields, never IDs.
- assistant_response should sound like a planning lead: mention what already exists, what changed in the draft, and what should be refined next.
- Use selected node context if supplied."#
}

fn normalize(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

fn extract_json_object(raw: &str) -> Result<String, AppError> {
    let without_fences = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();
    if without_fences.starts_with('{') && without_fences.ends_with('}') {
        return Ok(without_fences);
    }

    let bytes = without_fences.as_bytes();
    let mut depth = 0_i32;
    let mut start: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        let ch = *byte as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '{' {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                if let Some(start_idx) = start {
                    return Ok(without_fences[start_idx..=index].to_string());
                }
            }
        }
    }
    Err(AppError::Validation(
        "Planner model did not return JSON".to_string(),
    ))
}

fn parse_final_response(raw: &str) -> Result<PlannerPlan, AppError> {
    let object = extract_json_object(raw)?;
    let parsed: PlannerFinalResponse = serde_json::from_str(&object)?;
    let actions = parsed
        .actions
        .into_iter()
        .filter_map(normalize_planner_action)
        .collect::<Vec<_>>();
    Ok(PlannerPlan {
        assistant_response: parsed.assistant_response,
        needs_confirmation: parsed.needs_confirmation,
        clarification_question: parsed.clarification_question,
        actions,
    })
}

fn parse_agent_turn(raw: &str) -> Result<Result<PlannerToolCall, PlannerPlan>, AppError> {
    let object = extract_json_object(raw)?;
    let value: Value = serde_json::from_str(&object)?;
    if value.get("type").and_then(Value::as_str) == Some("tool_call") {
        let tool_call: PlannerToolCall = serde_json::from_value(value)?;
        return Ok(Ok(tool_call));
    }
    Ok(Err(parse_final_response(&object)?))
}

fn normalized_target_string(action: &Value, key: &str) -> Option<String> {
    action
        .get("target")
        .and_then(|target| target.get(key))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn normalize_planner_action(action: Value) -> Option<Value> {
    let action_type = action.get("type").and_then(Value::as_str)?.to_string();
    let target_product_name = normalized_target_string(&action, "productName");
    let target_module_name = normalized_target_string(&action, "moduleName");
    let target_capability_name = normalized_target_string(&action, "capabilityName");
    let target_work_item_title = normalized_target_string(&action, "workItemTitle");
    let mut action = action;
    let object = action.as_object_mut()?;

    match action_type.as_str() {
        "create_product" => {
            if !object.contains_key("name") {
                if let Some(name) = target_product_name {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
        }
        "create_module" => {
            if !object.contains_key("name") {
                if let Some(name) = target_module_name {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
        }
        "create_capability" => {
            if !object.contains_key("name") {
                if let Some(name) = target_capability_name {
                    object.insert("name".to_string(), Value::String(name));
                }
            }
        }
        "create_work_item" => {
            if !object.contains_key("title") {
                if let Some(title) = target_work_item_title {
                    object.insert("title".to_string(), Value::String(title));
                } else if let Some(title) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                {
                    object.insert("title".to_string(), Value::String(title));
                }
            }
        }
        _ => {}
    }

    Some(action)
}

async fn run_completion(
    db: &SqlitePool,
    provider_id: &str,
    model_name: &str,
    messages: Vec<ChatMessage>,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(db, provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gateway = OpenAiCompatibleProvider::new(provider.base_url, api_key);
    let response = gateway
        .run_completion(CompletionRequest {
            model: model_name.to_string(),
            messages,
            temperature: Some(0.1),
            max_tokens: Some(1800),
        })
        .await?;
    Ok(response.content)
}

async fn list_products_tool(db: &SqlitePool) -> Result<Value, AppError> {
    let products = product_repo::list_products(db).await?;
    Ok(serde_json::to_value(products)?)
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
) -> Result<Value, AppError> {
    let product_id = if let Some(name) = product_name {
        Some(find_product(db, Some(name)).await?.id)
    } else {
        None
    };
    let work_items =
        work_item_repo::list_work_items(db, product_id.as_deref(), None, None, status).await?;
    Ok(serde_json::to_value(work_items)?)
}

async fn run_tool_loop(
    db: &SqlitePool,
    provider_id: &str,
    model_name: &str,
    conversation: &[PlannerConversationEntry],
    pending_plan: Option<&PlannerPlan>,
    draft_plan: Option<&PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
    user_input: &str,
) -> Result<PlannerPlan, AppError> {
    let mut messages = vec![ChatMessage {
        role: "system".to_string(),
        content: planner_system_prompt().to_string(),
    }];

    let history = conversation
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|entry| format!("{}: {}", entry.role.to_uppercase(), entry.content))
        .collect::<Vec<_>>()
        .join("\n");

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: format!(
            "Recent conversation:\n{}\n\nCurrent pending proposal:\n{}\n\nCurrent draft tree:\n{}\n\nSelected draft node:\n{}\n\nLatest user request:\n{}",
            if history.is_empty() {
                "No prior conversation."
            } else {
                &history
            },
            pending_plan
                .map(|plan| serde_json::to_string_pretty(plan))
                .transpose()?
                .unwrap_or_else(|| "No pending proposal.".to_string()),
            draft_plan
                .map(|plan| serde_json::to_string_pretty(plan))
                .transpose()?
                .unwrap_or_else(|| "No draft yet.".to_string()),
            selected_draft_node_id
                .and_then(|node_id| draft_plan.and_then(|draft| draft.nodes.iter().find(|node| node.id == node_id)))
                .map(serde_json::to_string_pretty)
                .transpose()?
                .unwrap_or_else(|| "No draft node selected.".to_string()),
            user_input
        ),
    });

    for _ in 0..6 {
        let completion = run_completion(db, provider_id, model_name, messages.clone()).await?;
        match parse_agent_turn(&completion)? {
            Ok(tool_call) => {
                let args = tool_call.arguments.clone().unwrap_or_else(|| json!({}));
                let tool_result = match tool_call.tool.as_str() {
                    "list_products" => list_products_tool(db).await,
                    "get_product_tree" => {
                        get_product_tree_tool(db, args.get("productName").and_then(Value::as_str))
                            .await
                    }
                    "list_work_items" => {
                        list_work_items_tool(
                            db,
                            args.get("productName").and_then(Value::as_str),
                            args.get("status").and_then(Value::as_str),
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
            Err(plan) => return Ok(plan),
        }
    }

    Err(AppError::Validation(
        "Planner exceeded tool-step limit before returning a final plan".to_string(),
    ))
}

fn heuristic_plan(input: &str) -> PlannerPlan {
    let lower = input.trim().to_lowercase();
    if (lower.contains("tree") || lower.contains("hierarch"))
        && (lower.contains("work item") || lower.contains("workitem") || lower.contains("tasks"))
    {
        return PlannerPlan {
            assistant_response: "I’ll show the current work items in a hierarchical tree."
                .to_string(),
            needs_confirmation: false,
            clarification_question: None,
            actions: vec![json!({ "type": "report_tree" })],
        };
    }
    if lower.contains("status") {
        return PlannerPlan {
            assistant_response: "I’ll report the current status from local workspace data."
                .to_string(),
            needs_confirmation: false,
            clarification_question: None,
            actions: vec![json!({ "type": "report_status" })],
        };
    }
    PlannerPlan {
        assistant_response: "I need a configured model to turn open-ended planning conversation into structured suggestions.".to_string(),
        needs_confirmation: false,
        clarification_question: Some(
            "Configure a model, or tell me explicitly what product, capability, or work item you want me to assess.".to_string(),
        ),
        actions: vec![],
    }
}

fn is_informational_only(plan: &PlannerPlan) -> bool {
    !plan.actions.is_empty()
        && plan.actions.iter().all(|action| {
            matches!(
                action.get("type").and_then(Value::as_str),
                Some("report_status") | Some("report_tree")
            )
        })
}

fn has_draft_mutations(plan: &PlannerPlan) -> bool {
    plan.actions.iter().any(|action| {
        matches!(
            action.get("type").and_then(Value::as_str),
            Some(
                "create_product"
                    | "create_module"
                    | "create_capability"
                    | "create_work_item"
                    | "update_product"
                    | "update_module"
                    | "update_capability"
                    | "update_work_item"
                    | "archive_product"
                    | "delete_module"
                    | "delete_capability"
                    | "delete_work_item"
            )
        )
    })
}

fn requires_confirmation(plan: &PlannerPlan) -> bool {
    !plan.actions.is_empty() && (plan.needs_confirmation || !is_informational_only(plan))
}

async fn find_product(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<crate::domain::product::Product, AppError> {
    let products = product_repo::list_products(db).await?;
    if let Some(name) = product_name {
        let normalized = normalize(Some(name));
        let exact = products
            .iter()
            .find(|product| normalize(Some(&product.name)) == normalized)
            .cloned();
        if let Some(product) = exact {
            return Ok(product);
        }
        let partial = products
            .into_iter()
            .filter(|product| normalize(Some(&product.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple products match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!("No product matches {}", name)));
    }
    if products.len() == 1 {
        return Ok(products[0].clone());
    }
    Err(AppError::Validation("Product is required".to_string()))
}

async fn find_module(
    db: &SqlitePool,
    product_name: Option<&str>,
    module_name: Option<&str>,
) -> Result<crate::domain::product::Module, AppError> {
    let product = find_product(db, product_name).await?;
    let modules = product_repo::list_modules(db, &product.id).await?;
    if let Some(name) = module_name {
        let normalized = normalize(Some(name));
        let exact = modules
            .iter()
            .find(|module| normalize(Some(&module.name)) == normalized)
            .cloned();
        if let Some(module) = exact {
            return Ok(module);
        }
        let partial = modules
            .into_iter()
            .filter(|module| normalize(Some(&module.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple modules match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!("No module matches {}", name)));
    }
    if modules.len() == 1 {
        return Ok(modules[0].clone());
    }
    Err(AppError::Validation("Module is required".to_string()))
}

fn flatten_capabilities(
    nodes: &[crate::domain::product::CapabilityTree],
    bucket: &mut Vec<crate::domain::product::Capability>,
) {
    for node in nodes {
        bucket.push(node.capability.clone());
        flatten_capabilities(&node.children, bucket);
    }
}

async fn find_capability(
    db: &SqlitePool,
    product_name: Option<&str>,
    module_name: Option<&str>,
    capability_name: Option<&str>,
) -> Result<crate::domain::product::Capability, AppError> {
    let module = find_module(db, product_name, module_name).await?;
    let capabilities = product_repo::list_capabilities(db, &module.id).await?;
    if let Some(name) = capability_name {
        let normalized = normalize(Some(name));
        let exact = capabilities
            .iter()
            .find(|capability| normalize(Some(&capability.name)) == normalized)
            .cloned();
        if let Some(capability) = exact {
            return Ok(capability);
        }
        let partial = capabilities
            .into_iter()
            .filter(|capability| normalize(Some(&capability.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple capabilities match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!(
            "No capability matches {}",
            name
        )));
    }
    Err(AppError::Validation("Capability is required".to_string()))
}

async fn find_work_item(
    db: &SqlitePool,
    work_item_title: Option<&str>,
    product_name: Option<&str>,
) -> Result<crate::domain::work_item::WorkItem, AppError> {
    let product_id = if let Some(name) = product_name {
        Some(find_product(db, Some(name)).await?.id)
    } else {
        None
    };
    let work_items =
        work_item_repo::list_work_items(db, product_id.as_deref(), None, None, None).await?;
    if let Some(title) = work_item_title {
        let normalized = normalize(Some(title));
        let exact = work_items
            .iter()
            .find(|work_item| normalize(Some(&work_item.title)) == normalized)
            .cloned();
        if let Some(work_item) = exact {
            return Ok(work_item);
        }
        let partial = work_items
            .into_iter()
            .filter(|work_item| normalize(Some(&work_item.title)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple work items match {}",
                title
            )));
        }
        return Err(AppError::NotFound(format!(
            "No work item matches {}",
            title
        )));
    }
    Err(AppError::Validation("Work item is required".to_string()))
}

fn string_field(action: &Value, key: &str) -> Option<String> {
    action
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn string_array_field(action: &Value, key: &str) -> Option<Vec<String>> {
    action.get(key).and_then(Value::as_array).map(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    })
}

fn target_field<'a>(action: &'a Value, key: &str) -> Option<&'a str> {
    action.get("target")?.get(key)?.as_str()
}

fn fields_field<'a>(action: &'a Value, key: &str) -> Option<&'a Value> {
    action.get("fields")?.get(key)
}

fn fields_string(action: &Value, key: &str) -> Option<String> {
    fields_field(action, key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn fields_string_array(action: &Value, key: &str) -> Option<Vec<String>> {
    fields_field(action, key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
}

fn format_joined(values: Option<Vec<String>>) -> String {
    values.unwrap_or_default().join(", ")
}

fn draft_node_meta(node_type: &str) -> String {
    match node_type {
        "product" => "draft product",
        "module" => "draft module",
        "capability" => "draft capability",
        "work_item" => "draft work item",
        _ => "draft node",
    }
    .to_string()
}

fn build_draft_tree_children(
    draft_plan: &PlannerDraftPlan,
    parent_id: Option<&str>,
    selected_node_id: Option<&str>,
) -> Vec<PlannerTreeNode> {
    let mut nodes = draft_plan
        .nodes
        .iter()
        .filter(|node| node.parent_id.as_deref() == parent_id)
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.into_iter()
        .map(|node| {
            let mut meta = draft_node_meta(&node.node_type);
            if selected_node_id == Some(node.id.as_str()) {
                meta = format!("{meta} selected");
            }
            PlannerTreeNode {
                id: node.id.clone(),
                label: node.name.clone(),
                meta: Some(meta),
                children: build_draft_tree_children(draft_plan, Some(&node.id), selected_node_id),
            }
        })
        .collect()
}

fn build_draft_tree_nodes(
    draft_plan: &PlannerDraftPlan,
    selected_node_id: Option<&str>,
) -> Vec<PlannerTreeNode> {
    build_draft_tree_children(draft_plan, None, selected_node_id)
}

fn find_draft_node<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_type: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Option<&'a PlannerDraftNode> {
    let normalized = normalize(Some(name));
    draft_plan.nodes.iter().find(|node| {
        node.node_type == node_type
            && node.parent_id.as_deref() == parent_id
            && normalize(Some(&node.name)) == normalized
    })
}

fn find_draft_node_mut<'a>(
    draft_plan: &'a mut PlannerDraftPlan,
    node_type: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Option<&'a mut PlannerDraftNode> {
    let normalized = normalize(Some(name));
    draft_plan.nodes.iter_mut().find(|node| {
        node.node_type == node_type
            && node.parent_id.as_deref() == parent_id
            && normalize(Some(&node.name)) == normalized
    })
}

fn find_draft_node_by_id<'a>(
    draft_plan: &'a PlannerDraftPlan,
    node_id: Option<&str>,
) -> Option<&'a PlannerDraftNode> {
    let node_id = node_id?;
    draft_plan.nodes.iter().find(|node| node.id == node_id)
}

fn infer_selected_draft_context(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let Some(draft_plan) = draft_plan else {
        return (None, None, None, None);
    };
    let Some(selected) = find_draft_node_by_id(draft_plan, selected_node_id) else {
        return (None, None, None, None);
    };

    let mut product_name = None;
    let mut module_name = None;
    let mut capability_name = None;
    let mut work_item_title = None;
    let mut current = Some(selected);
    while let Some(node) = current {
        match node.node_type.as_str() {
            "product" => product_name = Some(node.name.clone()),
            "module" => module_name = Some(node.name.clone()),
            "capability" => capability_name = Some(node.name.clone()),
            "work_item" => work_item_title = Some(node.name.clone()),
            _ => {}
        }
        current = node
            .parent_id
            .as_deref()
            .and_then(|parent_id| find_draft_node_by_id(draft_plan, Some(parent_id)));
    }
    (product_name, module_name, capability_name, work_item_title)
}

fn resolve_draft_product_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "productName") {
        return Ok(Some(name.to_string()));
    }
    let (product_name, _, _, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    if product_name.is_some() {
        return Ok(product_name);
    }
    let Some(draft_plan) = draft_plan else {
        return Ok(None);
    };
    let products = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "product")
        .collect::<Vec<_>>();
    if products.len() == 1 {
        return Ok(Some(products[0].name.clone()));
    }
    Ok(None)
}

fn resolve_draft_module_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "moduleName") {
        return Ok(Some(name.to_string()));
    }
    let (_, module_name, _, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    Ok(module_name)
}

fn resolve_draft_capability_name(
    draft_plan: Option<&PlannerDraftPlan>,
    selected_node_id: Option<&str>,
    action: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(name) = target_field(action, "capabilityName") {
        return Ok(Some(name.to_string()));
    }
    let (_, _, capability_name, _) = infer_selected_draft_context(draft_plan, selected_node_id);
    Ok(capability_name)
}

fn remove_draft_node_subtree(draft_plan: &mut PlannerDraftPlan, node_id: &str) {
    let mut to_remove = vec![node_id.to_string()];
    let mut index = 0;
    while index < to_remove.len() {
        let current = to_remove[index].clone();
        for child in draft_plan
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(current.as_str()))
        {
            to_remove.push(child.id.clone());
        }
        index += 1;
    }
    draft_plan.nodes.retain(|node| !to_remove.contains(&node.id));
}

fn build_tree_nodes_for_items(
    items: &[crate::domain::work_item::WorkItem],
    parent_id: Option<&str>,
) -> Vec<PlannerTreeNode> {
    let mut filtered = items
        .iter()
        .filter(|item| item.parent_work_item_id.as_deref() == parent_id)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then(left.title.cmp(&right.title))
    });
    filtered
        .into_iter()
        .map(|item| PlannerTreeNode {
            id: item.id.clone(),
            label: item.title.clone(),
            meta: Some(item.status.to_string()),
            children: build_tree_nodes_for_items(items, Some(&item.id)),
        })
        .collect()
}

async fn build_tree_nodes(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<Vec<PlannerTreeNode>, AppError> {
    let products = if let Some(name) = product_name {
        vec![find_product(db, Some(name)).await?]
    } else {
        product_repo::list_products(db).await?
    };

    let mut nodes = vec![];
    for product in products {
        let tree = product_repo::get_product_tree(db, &product.id).await?;
        let product_items =
            work_item_repo::list_work_items(db, Some(&product.id), None, None, None).await?;
        let mut included = std::collections::HashSet::new();
        let mut module_nodes = vec![];
        for module_tree in tree.modules {
            let mut children = vec![];
            let direct_items = product_items
                .iter()
                .filter(|item| {
                    item.module_id.as_deref() == Some(&module_tree.module.id)
                        && item.capability_id.is_none()
                })
                .cloned()
                .collect::<Vec<_>>();
            if !direct_items.is_empty() {
                for item in &direct_items {
                    included.insert(item.id.clone());
                }
                children.push(PlannerTreeNode {
                    id: format!("{}-direct", module_tree.module.id),
                    label: "Direct Work Items".to_string(),
                    meta: None,
                    children: build_tree_nodes_for_items(&direct_items, None),
                });
            }

            let mut flattened = vec![];
            flatten_capabilities(&module_tree.features, &mut flattened);
            for capability in flattened {
                let capability_items = product_items
                    .iter()
                    .filter(|item| item.capability_id.as_deref() == Some(&capability.id))
                    .cloned()
                    .collect::<Vec<_>>();
                if capability_items.is_empty() {
                    continue;
                }
                for item in &capability_items {
                    included.insert(item.id.clone());
                }
                children.push(PlannerTreeNode {
                    id: capability.id.clone(),
                    label: capability.name.clone(),
                    meta: None,
                    children: build_tree_nodes_for_items(&capability_items, None),
                });
            }

            module_nodes.push(PlannerTreeNode {
                id: module_tree.module.id.clone(),
                label: module_tree.module.name.clone(),
                meta: None,
                children,
            });
        }

        let unscoped = product_items
            .iter()
            .filter(|item| !included.contains(&item.id) && item.parent_work_item_id.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if !unscoped.is_empty() {
            module_nodes.push(PlannerTreeNode {
                id: format!("{}-unscoped", product.id),
                label: "Unscoped".to_string(),
                meta: None,
                children: build_tree_nodes_for_items(&unscoped, None),
            });
        }

        if module_nodes.is_empty() {
            module_nodes.push(PlannerTreeNode {
                id: format!("{}-empty", product.id),
                label: "No work items".to_string(),
                meta: Some("empty".to_string()),
                children: vec![],
            });
        }

        nodes.push(PlannerTreeNode {
            id: product.id,
            label: product.name,
            meta: None,
            children: module_nodes,
        });
    }
    Ok(nodes)
}

async fn apply_actions_to_draft(
    draft_plan: Option<PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
    actions: &[Value],
) -> Result<PlannerDraftPlan, AppError> {
    let mut draft_plan = draft_plan.unwrap_or(PlannerDraftPlan { nodes: vec![] });

    for action in actions {
        let action_type = action
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("Planner action missing type".to_string()))?;
        match action_type {
            "create_product" => {
                let name = string_field(action, "name")
                    .ok_or_else(|| AppError::Validation("Draft product name is required".to_string()))?;
                if find_draft_node(&draft_plan, "product", &name, None).is_none() {
                    draft_plan.nodes.push(PlannerDraftNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        parent_id: None,
                        node_type: "product".to_string(),
                        name,
                        summary: string_field(action, "description")
                            .or_else(|| string_field(action, "vision")),
                        details: action.clone(),
                    });
                }
            }
            "create_module" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft module needs a product".to_string()))?;
                let product = find_draft_node(&draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
                let name = string_field(action, "name")
                    .ok_or_else(|| AppError::Validation("Draft module name is required".to_string()))?;
                if find_draft_node(&draft_plan, "module", &name, Some(&product.id)).is_none() {
                    draft_plan.nodes.push(PlannerDraftNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        parent_id: Some(product.id.clone()),
                        node_type: "module".to_string(),
                        name,
                        summary: string_field(action, "description")
                            .or_else(|| string_field(action, "purpose")),
                        details: action.clone(),
                    });
                }
            }
            "create_capability" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft capability needs a product".to_string()))?;
                let product = find_draft_node(&draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
                let module_name = resolve_draft_module_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft capability needs a module".to_string()))?;
                let module = find_draft_node(&draft_plan, "module", &module_name, Some(&product.id))
                    .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?;
                let name = string_field(action, "name")
                    .ok_or_else(|| AppError::Validation("Draft capability name is required".to_string()))?;
                if find_draft_node(&draft_plan, "capability", &name, Some(&module.id)).is_none() {
                    draft_plan.nodes.push(PlannerDraftNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        parent_id: Some(module.id.clone()),
                        node_type: "capability".to_string(),
                        name,
                        summary: string_field(action, "description")
                            .or_else(|| string_field(action, "acceptanceCriteria")),
                        details: action.clone(),
                    });
                }
            }
            "create_work_item" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft work item needs a product".to_string()))?;
                let product = find_draft_node(&draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
                let title = string_field(action, "title")
                    .ok_or_else(|| AppError::Validation("Draft work item title is required".to_string()))?;
                let parent_id = if let Some(capability_name) = resolve_draft_capability_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )? {
                    let module_name = resolve_draft_module_name(
                        Some(&draft_plan),
                        selected_draft_node_id,
                        action,
                    )?
                    .ok_or_else(|| AppError::Validation("Draft work item capability needs a module".to_string()))?;
                    let module =
                        find_draft_node(&draft_plan, "module", &module_name, Some(&product.id))
                            .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?;
                    let capability = find_draft_node(
                        &draft_plan,
                        "capability",
                        &capability_name,
                        Some(&module.id),
                    )
                    .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
                    Some(capability.id.clone())
                } else if let Some(module_name) = resolve_draft_module_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )? {
                    let module =
                        find_draft_node(&draft_plan, "module", &module_name, Some(&product.id))
                            .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?;
                    Some(module.id.clone())
                } else {
                    Some(product.id.clone())
                };
                if find_draft_node(&draft_plan, "work_item", &title, parent_id.as_deref()).is_none()
                {
                    draft_plan.nodes.push(PlannerDraftNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        parent_id,
                        node_type: "work_item".to_string(),
                        name: title,
                        summary: string_field(action, "description")
                            .or_else(|| string_field(action, "problemStatement")),
                        details: action.clone(),
                    });
                }
            }
            "update_product" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
                let node = find_draft_node_mut(&mut draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?;
                if let Some(name) = fields_string(action, "name") {
                    node.name = name;
                }
                node.summary = fields_string(action, "description").or_else(|| node.summary.clone());
                node.details = action.clone();
            }
            "update_module" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft module needs a product".to_string()))?;
                let product_id = find_draft_node(&draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?
                    .id
                    .clone();
                let module_name = resolve_draft_module_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?;
                let node = find_draft_node_mut(&mut draft_plan, "module", &module_name, Some(&product_id))
                    .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?;
                if let Some(name) = fields_string(action, "name") {
                    node.name = name;
                }
                node.summary = fields_string(action, "description").or_else(|| node.summary.clone());
                node.details = action.clone();
            }
            "update_capability" => {
                let product_name = resolve_draft_product_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft capability needs a product".to_string()))?;
                let product_id = find_draft_node(&draft_plan, "product", &product_name, None)
                    .ok_or_else(|| AppError::Validation("Draft product is required".to_string()))?
                    .id
                    .clone();
                let module_name = resolve_draft_module_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft capability needs a module".to_string()))?;
                let module_id = find_draft_node(&draft_plan, "module", &module_name, Some(&product_id))
                    .ok_or_else(|| AppError::Validation("Draft module is required".to_string()))?
                    .id
                    .clone();
                let capability_name = resolve_draft_capability_name(
                    Some(&draft_plan),
                    selected_draft_node_id,
                    action,
                )?
                .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
                let node = find_draft_node_mut(
                    &mut draft_plan,
                    "capability",
                    &capability_name,
                    Some(&module_id),
                )
                .ok_or_else(|| AppError::Validation("Draft capability is required".to_string()))?;
                if let Some(name) = fields_string(action, "name") {
                    node.name = name;
                }
                node.summary = fields_string(action, "description").or_else(|| node.summary.clone());
                node.details = action.clone();
            }
            "update_work_item" => {
                // First-cut: work item updates apply by title when uniquely scoped by selection.
                let (_, _, _, selected_work_item_title) =
                    infer_selected_draft_context(Some(&draft_plan), selected_draft_node_id);
                let title = target_field(action, "workItemTitle")
                    .map(ToString::to_string)
                    .or(selected_work_item_title)
                    .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
                let node = draft_plan
                    .nodes
                    .iter_mut()
                    .find(|node| {
                        node.node_type == "work_item" && normalize(Some(&node.name)) == normalize(Some(&title))
                    })
                    .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
                if let Some(name) = fields_string(action, "title") {
                    node.name = name;
                }
                node.summary = fields_string(action, "description").or_else(|| node.summary.clone());
                node.details = action.clone();
            }
            "archive_product" | "delete_module" | "delete_capability" | "delete_work_item" => {
                let candidate = match action_type {
                    "archive_product" => resolve_draft_product_name(
                        Some(&draft_plan),
                        selected_draft_node_id,
                        action,
                    )?
                    .and_then(|name| find_draft_node(&draft_plan, "product", &name, None).map(|node| node.id.clone())),
                    "delete_module" => {
                        let product_name = resolve_draft_product_name(
                            Some(&draft_plan),
                            selected_draft_node_id,
                            action,
                        )?;
                        let product = product_name
                            .as_deref()
                            .and_then(|name| find_draft_node(&draft_plan, "product", name, None));
                        let module_name = resolve_draft_module_name(
                            Some(&draft_plan),
                            selected_draft_node_id,
                            action,
                        )?;
                        match (product, module_name) {
                            (Some(product), Some(module_name)) => find_draft_node(
                                &draft_plan,
                                "module",
                                &module_name,
                                Some(&product.id),
                            )
                            .map(|node| node.id.clone()),
                            _ => None,
                        }
                    }
                    "delete_capability" => {
                        let product_name = resolve_draft_product_name(
                            Some(&draft_plan),
                            selected_draft_node_id,
                            action,
                        )?;
                        let product = product_name
                            .as_deref()
                            .and_then(|name| find_draft_node(&draft_plan, "product", name, None));
                        let module_name = resolve_draft_module_name(
                            Some(&draft_plan),
                            selected_draft_node_id,
                            action,
                        )?;
                        let module = match (product, module_name) {
                            (Some(product), Some(module_name)) => {
                                find_draft_node(&draft_plan, "module", &module_name, Some(&product.id))
                            }
                            _ => None,
                        };
                        let capability_name = resolve_draft_capability_name(
                            Some(&draft_plan),
                            selected_draft_node_id,
                            action,
                        )?;
                        match (module, capability_name) {
                            (Some(module), Some(capability_name)) => find_draft_node(
                                &draft_plan,
                                "capability",
                                &capability_name,
                                Some(&module.id),
                            )
                            .map(|node| node.id.clone()),
                            _ => None,
                        }
                    }
                    "delete_work_item" => {
                        let title = target_field(action, "workItemTitle")
                            .map(ToString::to_string)
                            .ok_or_else(|| AppError::Validation("Draft work item is required".to_string()))?;
                        draft_plan.nodes.iter().find(|node| {
                            node.node_type == "work_item"
                                && normalize(Some(&node.name)) == normalize(Some(&title))
                        })
                        .map(|node| node.id.clone())
                    }
                    _ => None,
                };
                if let Some(node_id) = candidate {
                    remove_draft_node_subtree(&mut draft_plan, &node_id);
                }
            }
            "report_tree" | "report_status" => {}
            _ => {}
        }
    }

    Ok(draft_plan)
}

async fn commit_draft_plan(
    state: &AppState,
    draft_plan: &PlannerDraftPlan,
) -> Result<Vec<String>, AppError> {
    let mut lines = vec![];
    let mut product_ids: HashMap<String, String> = HashMap::new();
    let mut module_ids: HashMap<String, String> = HashMap::new();
    let mut capability_ids: HashMap<String, String> = HashMap::new();

    let mut products = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "product" && node.parent_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    products.sort_by(|left, right| left.name.cmp(&right.name));

    for product_node in products {
        let details = &product_node.details;
        let product = product_repo::create_product(
            &state.db,
            &uuid::Uuid::new_v4().to_string(),
            &product_node.name,
            &product_node.summary.clone().unwrap_or_default(),
            &string_field(details, "vision").unwrap_or_default(),
            &format_joined(string_array_field(details, "goals")),
            &format_joined(string_array_field(details, "tags")),
        )
        .await?;
        lines.push(format!("Created product \"{}\".", product.name));
        product_ids.insert(product_node.id.clone(), product.id.clone());

        let mut modules = draft_plan
            .nodes
            .iter()
            .filter(|node| node.node_type == "module" && node.parent_id.as_deref() == Some(&product_node.id))
            .cloned()
            .collect::<Vec<_>>();
        modules.sort_by(|left, right| left.name.cmp(&right.name));

        for module_node in modules {
            let module = product_repo::create_module(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product.id,
                &module_node.name,
                &module_node.summary.clone().unwrap_or_default(),
                &string_field(&module_node.details, "purpose").unwrap_or_default(),
            )
            .await?;
            lines.push(format!("Created module \"{}\" in \"{}\".", module.name, product.name));
            module_ids.insert(module_node.id.clone(), module.id.clone());

            let mut capabilities = draft_plan
                .nodes
                .iter()
                .filter(|node| node.node_type == "capability" && node.parent_id.as_deref() == Some(&module_node.id))
                .cloned()
                .collect::<Vec<_>>();
            capabilities.sort_by(|left, right| left.name.cmp(&right.name));

            for capability_node in capabilities {
                let capability = product_repo::create_capability(
                    &state.db,
                    &uuid::Uuid::new_v4().to_string(),
                    &module.id,
                    None,
                    &capability_node.name,
                    &capability_node.summary.clone().unwrap_or_default(),
                    &string_field(&capability_node.details, "acceptanceCriteria").unwrap_or_default(),
                    string_field(&capability_node.details, "priority").as_deref().unwrap_or("medium"),
                    string_field(&capability_node.details, "risk").as_deref().unwrap_or("medium"),
                    &string_field(&capability_node.details, "technicalNotes").unwrap_or_default(),
                )
                .await?;
                lines.push(format!("Created capability \"{}\" in \"{}\".", capability.name, module.name));
                capability_ids.insert(capability_node.id.clone(), capability.id.clone());
            }
        }
    }

    let mut work_items = draft_plan
        .nodes
        .iter()
        .filter(|node| node.node_type == "work_item")
        .cloned()
        .collect::<Vec<_>>();
    work_items.sort_by(|left, right| left.name.cmp(&right.name));

    for work_item_node in work_items {
        let mut product_id = None;
        let mut module_id = None;
        let mut capability_id = None;
        let mut parent = work_item_node.parent_id.as_deref();
        while let Some(parent_id) = parent {
            if let Some(node) = draft_plan.nodes.iter().find(|candidate| candidate.id == parent_id) {
                match node.node_type.as_str() {
                    "capability" => {
                        capability_id = capability_ids.get(&node.id).cloned();
                    }
                    "module" => {
                        module_id = module_ids.get(&node.id).cloned();
                    }
                    "product" => {
                        product_id = product_ids.get(&node.id).cloned();
                    }
                    _ => {}
                }
                parent = node.parent_id.as_deref();
            } else {
                parent = None;
            }
        }
        let product_id = product_id
            .or_else(|| {
                module_id.as_ref().and_then(|module_id| {
                    draft_plan
                        .nodes
                        .iter()
                        .find(|node| module_ids.get(&node.id) == Some(module_id))
                        .and_then(|node| node.parent_id.as_ref())
                        .and_then(|parent_id| product_ids.get(parent_id))
                        .cloned()
                })
            })
            .ok_or_else(|| AppError::Validation("Draft work item is missing a product".to_string()))?;

        let work_item = work_item_repo::create_work_item(
            &state.db,
            &uuid::Uuid::new_v4().to_string(),
            &product_id,
            module_id.as_deref(),
            capability_id.as_deref(),
            None,
            &work_item_node.name,
            &string_field(&work_item_node.details, "problemStatement")
                .or_else(|| work_item_node.summary.clone())
                .unwrap_or_default(),
            &string_field(&work_item_node.details, "description")
                .or_else(|| work_item_node.summary.clone())
                .unwrap_or_default(),
            &string_field(&work_item_node.details, "acceptanceCriteria").unwrap_or_default(),
            &string_field(&work_item_node.details, "constraints").unwrap_or_default(),
            string_field(&work_item_node.details, "workItemType")
                .as_deref()
                .unwrap_or("feature"),
            string_field(&work_item_node.details, "priority")
                .as_deref()
                .unwrap_or("medium"),
            string_field(&work_item_node.details, "complexity")
                .as_deref()
                .unwrap_or("medium"),
        )
        .await?;
        lines.push(format!("Created work item \"{}\".", work_item.title));
    }

    Ok(lines)
}

async fn execute_action(state: &AppState, action: &Value) -> Result<Vec<String>, AppError> {
    let action_type = action
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Planner action missing type".to_string()))?;
    match action_type {
        "create_product" => {
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing product name".to_string()))?;
            let product = product_repo::create_product(
                &state.db,
                &id,
                &name,
                &string_field(action, "description").unwrap_or_default(),
                &string_field(action, "vision").unwrap_or_default(),
                &format_joined(string_array_field(action, "goals")),
                &format_joined(string_array_field(action, "tags")),
            )
            .await?;
            Ok(vec![format!("Created product \"{}\".", product.name)])
        }
        "update_product" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let updated = product_repo::update_product(
                &state.db,
                &product.id,
                fields_string(action, "name").as_deref(),
                fields_string(action, "description").as_deref(),
                fields_string(action, "vision").as_deref(),
                fields_string_array(action, "goals")
                    .map(|v| v.join(", "))
                    .as_deref(),
                fields_string_array(action, "tags")
                    .map(|v| v.join(", "))
                    .as_deref(),
            )
            .await?;
            Ok(vec![format!("Updated product \"{}\".", updated.name)])
        }
        "archive_product" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            product_repo::archive_product(&state.db, &product.id).await?;
            Ok(vec![format!("Archived product \"{}\".", product.name)])
        }
        "create_module" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing module name".to_string()))?;
            let module = product_repo::create_module(
                &state.db,
                &id,
                &product.id,
                &name,
                &string_field(action, "description").unwrap_or_default(),
                &string_field(action, "purpose").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!(
                "Created module \"{}\" in \"{}\".",
                module.name, product.name
            )])
        }
        "update_module" => {
            let module = find_module(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "moduleName"),
            )
            .await?;
            let updated = product_repo::update_module(
                &state.db,
                &module.id,
                fields_string(action, "name").as_deref(),
                fields_string(action, "description").as_deref(),
                fields_string(action, "purpose").as_deref(),
            )
            .await?;
            Ok(vec![format!("Updated module \"{}\".", updated.name)])
        }
        "delete_module" => {
            let module = find_module(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "moduleName"),
            )
            .await?;
            product_repo::delete_module(&state.db, &module.id).await?;
            Ok(vec![format!("Deleted module \"{}\".", module.name)])
        }
        "create_capability" => {
            let module = find_module(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "moduleName"),
            )
            .await?;
            let parent_capability_id = if target_field(action, "capabilityName").is_some() {
                Some(
                    find_capability(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "moduleName"),
                        target_field(action, "capabilityName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let id = uuid::Uuid::new_v4().to_string();
            let name = string_field(action, "name")
                .ok_or_else(|| AppError::Validation("Missing capability name".to_string()))?;
            let capability = product_repo::create_capability(
                &state.db,
                &id,
                &module.id,
                parent_capability_id.as_deref(),
                &name,
                &string_field(action, "description").unwrap_or_default(),
                &string_field(action, "acceptanceCriteria").unwrap_or_default(),
                &string_field(action, "priority").unwrap_or_else(|| "medium".to_string()),
                &string_field(action, "risk").unwrap_or_else(|| "medium".to_string()),
                &string_field(action, "technicalNotes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!(
                "Created capability \"{}\" in \"{}\".",
                capability.name, module.name
            )])
        }
        "update_capability" => {
            let capability = find_capability(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "moduleName"),
                target_field(action, "capabilityName"),
            )
            .await?;
            let updated = product_repo::update_capability(
                &state.db,
                &capability.id,
                fields_string(action, "name").as_deref(),
                fields_string(action, "description").as_deref(),
                fields_string(action, "acceptanceCriteria").as_deref(),
                fields_string(action, "priority").as_deref(),
                fields_string(action, "risk").as_deref(),
                fields_string(action, "technicalNotes").as_deref(),
            )
            .await?;
            Ok(vec![format!("Updated capability \"{}\".", updated.name)])
        }
        "delete_capability" => {
            let capability = find_capability(
                &state.db,
                target_field(action, "productName"),
                target_field(action, "moduleName"),
                target_field(action, "capabilityName"),
            )
            .await?;
            product_repo::delete_capability(&state.db, &capability.id).await?;
            Ok(vec![format!("Deleted capability \"{}\".", capability.name)])
        }
        "create_work_item" => {
            let product = find_product(&state.db, target_field(action, "productName")).await?;
            let module_id = if target_field(action, "moduleName").is_some() {
                Some(
                    find_module(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "moduleName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let capability_id = if target_field(action, "capabilityName").is_some() {
                Some(
                    find_capability(
                        &state.db,
                        target_field(action, "productName"),
                        target_field(action, "moduleName"),
                        target_field(action, "capabilityName"),
                    )
                    .await?
                    .id,
                )
            } else {
                None
            };
            let id = uuid::Uuid::new_v4().to_string();
            let title = string_field(action, "title")
                .ok_or_else(|| AppError::Validation("Missing work item title".to_string()))?;
            let work_item = work_item_repo::create_work_item(
                &state.db,
                &id,
                &product.id,
                module_id.as_deref(),
                capability_id.as_deref(),
                None,
                &title,
                &string_field(action, "problemStatement")
                    .or_else(|| string_field(action, "description"))
                    .unwrap_or_default(),
                &string_field(action, "description").unwrap_or_default(),
                &string_field(action, "acceptanceCriteria").unwrap_or_default(),
                &string_field(action, "constraints").unwrap_or_default(),
                &string_field(action, "workItemType").unwrap_or_else(|| "feature".to_string()),
                &string_field(action, "priority").unwrap_or_else(|| "medium".to_string()),
                &string_field(action, "complexity").unwrap_or_else(|| "medium".to_string()),
            )
            .await?;
            Ok(vec![format!(
                "Created work item \"{}\" in \"{}\".",
                work_item.title, product.name
            )])
        }
        "update_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let updated = work_item_repo::update_work_item(
                &state.db,
                &work_item.id,
                fields_string(action, "title").as_deref(),
                fields_string(action, "description").as_deref(),
                fields_string(action, "status").as_deref(),
                fields_string(action, "problemStatement").as_deref(),
                fields_string(action, "acceptanceCriteria").as_deref(),
                fields_string(action, "constraints").as_deref(),
            )
            .await?;
            Ok(vec![format!("Updated work item \"{}\".", updated.title)])
        }
        "delete_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            work_item_repo::delete_work_item(&state.db, &work_item.id).await?;
            Ok(vec![format!("Deleted work item \"{}\".", work_item.title)])
        }
        "approve_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item.id,
                None,
                None,
                Some("approved"),
                None,
                None,
                None,
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
                let _ = workflow_service
                    .start_work_item_workflow(&work_item.id)
                    .await?;
            }
            Ok(vec![format!("Approved work item \"{}\".", work_item.title)])
        }
        "reject_work_item" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "task_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item.id,
                None,
                None,
                Some("draft"),
                None,
                None,
                None,
            )
            .await?;
            Ok(vec![format!("Rejected work item \"{}\".", work_item.title)])
        }
        "approve_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!("Approved plan for \"{}\".", work_item.title)])
        }
        "reject_work_item_plan" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "plan_approval",
                "rejected",
                &string_field(action, "notes")
                    .unwrap_or_else(|| "Rejected from planner.".to_string()),
            )
            .await?;
            Ok(vec![format!("Rejected plan for \"{}\".", work_item.title)])
        }
        "approve_work_item_test_review" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item.id,
                None,
                "test_review",
                "approved",
                &string_field(action, "notes").unwrap_or_default(),
            )
            .await?;
            Ok(vec![format!(
                "Approved test review for \"{}\".",
                work_item.title
            )])
        }
        "start_workflow" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .start_work_item_workflow(&work_item.id)
                .await?;
            Ok(vec![format!(
                "Started workflow for \"{}\".",
                work_item.title
            )])
        }
        "workflow_action" => {
            let work_item = find_work_item(
                &state.db,
                target_field(action, "workItemTitle"),
                target_field(action, "productName"),
            )
            .await?;
            let run =
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(format!(
                            "No workflow run exists for {}",
                            work_item.title
                        ))
                    })?;
            let action_name = string_field(action, "action")
                .ok_or_else(|| AppError::Validation("Missing workflow action".to_string()))?;
            let user_action = match action_name.as_str() {
                "approve" => crate::domain::workflow::UserAction::Approve,
                "reject" => crate::domain::workflow::UserAction::Reject,
                "pause" => crate::domain::workflow::UserAction::Pause,
                "resume" => crate::domain::workflow::UserAction::Resume,
                "cancel" => crate::domain::workflow::UserAction::Cancel,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action {}",
                        action_name
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&run.id, user_action, string_field(action, "notes"))
                .await?;
            Ok(vec![format!(
                "Applied workflow action \"{}\" to \"{}\".",
                action_name, work_item.title
            )])
        }
        "report_status" => {
            if let Some(work_item_title) = target_field(action, "workItemTitle") {
                let work_item = find_work_item(
                    &state.db,
                    Some(work_item_title),
                    target_field(action, "productName"),
                )
                .await?;
                let run =
                    workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id)
                        .await?;
                let product_name = if let Some(product_id) = work_item.product_id.as_deref() {
                    product_repo::get_product(&state.db, product_id)
                        .await
                        .ok()
                        .map(|p| p.name)
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    "unknown".to_string()
                };
                let mut lines = vec![
                    format!("Status for \"{}\": {}.", work_item.title, work_item.status),
                    format!("Product: {}.", product_name),
                ];
                if let Some(run) = run {
                    lines.push(format!(
                        "Workflow: {} at {}.",
                        run.status, run.current_stage
                    ));
                } else {
                    lines.push("Workflow: not started.".to_string());
                }
                Ok(lines)
            } else {
                let product = find_product(&state.db, target_field(action, "productName")).await?;
                let items =
                    work_item_repo::list_work_items(&state.db, Some(&product.id), None, None, None)
                        .await?;
                let mut counts: HashMap<String, usize> = HashMap::new();
                for item in items {
                    *counts.entry(item.status.to_string()).or_insert(0) += 1;
                }
                let mut lines = vec![format!("Status for \"{}\".", product.name)];
                let mut entries = counts.into_iter().collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (status, count) in entries {
                    lines.push(format!("{}: {}", status, count));
                }
                Ok(lines)
            }
        }
        "report_tree" => {
            let nodes = build_tree_nodes(&state.db, target_field(action, "productName")).await?;
            let mut lines = vec![];
            fn walk(node: &PlannerTreeNode, depth: usize, lines: &mut Vec<String>) {
                lines.push(format!("{}{}", "  ".repeat(depth), node.label));
                for child in &node.children {
                    walk(child, depth + 1, lines);
                }
            }
            for node in nodes {
                walk(&node, 0, &mut lines);
            }
            Ok(lines)
        }
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

async fn execute_plan(
    state: &AppState,
    plan: &PlannerPlan,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut lines = vec![];
    let mut errors = vec![];
    for action in &plan.actions {
        match execute_action(state, action).await {
            Ok(mut action_lines) => lines.append(&mut action_lines),
            Err(error) => errors.push(error.to_string()),
        }
    }
    Ok((lines, errors))
}

pub async fn create_planner_session(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let mut service = planner_service.lock().await;
    let info = service.create_session(provider_id.clone(), model_name.clone());
    planner_repo::create_session(
        db,
        &info.session_id,
        provider_id.as_deref(),
        model_name.as_deref(),
    )
    .await?;
    Ok(info)
}

pub async fn update_planner_session(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let mut service = planner_service.lock().await;
    let info = service.update_session(&session_id, provider_id.clone(), model_name.clone())?;
    planner_repo::update_session(
        db,
        &session_id,
        provider_id.as_deref(),
        model_name.as_deref(),
    )
    .await?;
    Ok(info)
}

pub async fn clear_planner_pending(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
) -> Result<PlannerSessionInfo, AppError> {
    let mut service = planner_service.lock().await;
    let info = service.clear_pending(&session_id)?;
    persist_pending_plan(db, &session_id, None).await?;
    persist_draft_state(db, &session_id, None, None).await?;
    Ok(info)
}

pub async fn submit_planner_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: String,
    selected_draft_node_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let mut session = {
        let mut service = planner_service.lock().await;
        match service.get_session(&session_id) {
            Ok(session) => session,
            Err(_) => {
                let loaded = load_session_from_db(&state.db, &session_id).await?;
                service.save_session(&session_id, loaded.clone());
                loaded
            }
        }
    };

    if selected_draft_node_id != session.selected_draft_node_id {
        session.selected_draft_node_id = selected_draft_node_id.clone();
        persist_draft_state(
            &state.db,
            &session_id,
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
    }

    let normalized = user_input.trim().to_lowercase();
    if matches!(normalized.as_str(), "yes" | "confirm" | "go ahead") {
        if let Some(draft_plan) = session.draft_plan.clone() {
            let execution_lines = commit_draft_plan(state, &draft_plan).await?;
            append_conversation(&state.db, &session_id, "user", &user_input).await?;
            session.conversation.push(PlannerConversationEntry {
                role: "user".to_string(),
                content: user_input.clone(),
            });
            append_conversation(&state.db, &session_id, "assistant", "Committed draft plan.")
                .await?;
            session.conversation.push(PlannerConversationEntry {
                role: "assistant".to_string(),
                content: "Committed draft plan.".to_string(),
            });
            session.pending_plan = None;
            session.draft_plan = None;
            session.selected_draft_node_id = None;
            persist_pending_plan(&state.db, &session_id, None).await?;
            persist_draft_state(&state.db, &session_id, None, None).await?;
            let mut service = planner_service.lock().await;
            service.save_session(&session_id, session);
            return Ok(PlannerTurnResponse {
                session_id,
                status: "execution".to_string(),
                assistant_message: "Committed draft plan.".to_string(),
                pending_plan: None,
                tree_nodes: None,
                draft_tree_nodes: None,
                selected_draft_node_id: None,
                execution_lines,
                execution_errors: vec![],
            });
        }
    }

    let plan = if let (Some(provider_id), Some(model_name)) =
        (session.provider_id.clone(), session.model_name.clone())
    {
        run_tool_loop(
            &state.db,
            &provider_id,
            &model_name,
            &session.conversation,
            session.pending_plan.as_ref(),
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
            &user_input,
        )
        .await?
    } else {
        heuristic_plan(&user_input)
    };

    let tree_nodes = if plan
        .actions
        .iter()
        .any(|action| action.get("type").and_then(Value::as_str) == Some("report_tree"))
    {
        build_tree_nodes(
            &state.db,
            plan.actions
                .iter()
                .find(|action| action.get("type").and_then(Value::as_str) == Some("report_tree"))
                .and_then(|action| target_field(action, "productName")),
        )
        .await
        .ok()
    } else {
        None
    };

    let draft_tree_nodes = session
        .draft_plan
        .as_ref()
        .map(|draft_plan| build_draft_tree_nodes(draft_plan, session.selected_draft_node_id.as_deref()));

    append_conversation(&state.db, &session_id, "user", &user_input).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: user_input.clone(),
    });

    if has_draft_mutations(&plan) {
        let updated_draft = apply_actions_to_draft(
            session.draft_plan.clone(),
            session.selected_draft_node_id.as_deref(),
            &plan.actions,
        )
        .await?;
        let updated_draft_tree_nodes =
            Some(build_draft_tree_nodes(&updated_draft, session.selected_draft_node_id.as_deref()));
        session.draft_plan = Some(updated_draft.clone());
        session.pending_plan = Some(plan.clone());
        persist_pending_plan(&state.db, &session_id, Some(&plan)).await?;
        persist_draft_state(
            &state.db,
            &session_id,
            Some(&updated_draft),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
        append_conversation(&state.db, &session_id, "assistant", &plan.assistant_response).await?;
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: plan.assistant_response.clone(),
        });
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
        return Ok(PlannerTurnResponse {
            session_id,
            status: "proposal".to_string(),
            assistant_message: plan.assistant_response.clone(),
            pending_plan: Some(plan),
            tree_nodes,
            draft_tree_nodes: updated_draft_tree_nodes,
            selected_draft_node_id: session.selected_draft_node_id.clone(),
            execution_lines: vec!["Updated the draft plan.".to_string()],
            execution_errors: vec![],
        });
    }

    if requires_confirmation(&plan) {
        session.pending_plan = Some(plan.clone());
        persist_pending_plan(&state.db, &session_id, Some(&plan)).await?;
        append_conversation(
            &state.db,
            &session_id,
            "assistant",
            &plan.assistant_response,
        )
        .await?;
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: plan.assistant_response.clone(),
        });
        let selected_draft_node_id = session.selected_draft_node_id.clone();
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session);
        return Ok(PlannerTurnResponse {
            session_id,
            status: "proposal".to_string(),
            assistant_message: plan.assistant_response.clone(),
            pending_plan: Some(plan),
            tree_nodes,
            draft_tree_nodes,
            selected_draft_node_id,
            execution_lines: vec![],
            execution_errors: vec![],
        });
    }

    if plan.actions.is_empty() {
        let assistant_message = plan
            .clarification_question
            .clone()
            .unwrap_or_else(|| plan.assistant_response.clone());
        append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });
        let pending_plan = session.pending_plan.clone();
        let selected_draft_node_id = session.selected_draft_node_id.clone();
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session);
        return Ok(PlannerTurnResponse {
            session_id,
            status: "clarification".to_string(),
            assistant_message,
            pending_plan,
            tree_nodes,
            draft_tree_nodes,
            selected_draft_node_id,
            execution_lines: vec![],
            execution_errors: vec![],
        });
    }

    let (execution_lines, execution_errors) = execute_plan(state, &plan).await?;
    let assistant_message = plan.assistant_response.clone();
    session.pending_plan = None;
    persist_pending_plan(&state.db, &session_id, None).await?;
    append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "assistant".to_string(),
        content: assistant_message.clone(),
    });
    let selected_draft_node_id = session.selected_draft_node_id.clone();
    let mut service = planner_service.lock().await;
    service.save_session(&session_id, session);
    Ok(PlannerTurnResponse {
        session_id,
        status: if is_informational_only(&plan) {
            "report".to_string()
        } else {
            "execution".to_string()
        },
        assistant_message,
        pending_plan: None,
        tree_nodes,
        draft_tree_nodes,
        selected_draft_node_id,
        execution_lines,
        execution_errors,
    })
}

pub async fn confirm_planner_plan(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
) -> Result<PlannerTurnResponse, AppError> {
    submit_planner_turn(
        planner_service,
        state,
        session_id,
        "confirm".to_string(),
        None,
    )
    .await
}

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
pub struct PlannerTurnResponse {
    pub session_id: String,
    pub status: String,
    pub assistant_message: String,
    pub pending_plan: Option<PlannerPlan>,
    pub tree_nodes: Option<Vec<PlannerTreeNode>>,
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
                conversation: vec![],
            },
        );
        PlannerSessionInfo {
            session_id,
            provider_id,
            model_name,
            has_pending_plan: false,
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
        })
    }

    fn clear_pending(&mut self, session_id: &str) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.pending_plan = None;
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id: session.provider_id.clone(),
            model_name: session.model_name.clone(),
            has_pending_plan: false,
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
    Ok(PlannerSession {
        provider_id: record.provider_id,
        model_name: record.model_name,
        pending_plan,
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
  "needs_confirmation": true,
  "clarification_question": null,
  "actions": []
}

Rules:
- Output valid JSON only. No markdown.
- Behave conversationally. First reason about what already exists in the supplied context, then suggest what should be added, changed, approved, or tracked.
- If the user is exploring or describing a need, prefer proposing actions rather than assuming immediate execution.
- If an entity already seems to exist, do not suggest creating a duplicate unless the user explicitly asks for a separate one.
- For any mutating action, assume confirmation is required before execution. Set needs_confirmation=true.
- Only set needs_confirmation=false for purely informational replies such as status reporting with no mutations.
- If the request is ambiguous, set actions=[] and put the missing detail in clarification_question.
- Use tools when the request depends on current repo state or structure instead of guessing from the prompt alone.
- Do not call mutation tools. You are only planning. Proposed mutations go in final.actions.
- After receiving tool results, continue reasoning and either call another tool or return type=final.
- Use these action types only:
create_product, update_product, archive_product,
create_module, update_module, delete_module,
create_capability, update_capability, delete_capability,
create_work_item, update_work_item, delete_work_item,
approve_work_item, reject_work_item, approve_work_item_plan, reject_work_item_plan, approve_work_item_test_review,
start_workflow, workflow_action, report_status, report_tree.
- Use product/module/capability/work item names in target fields, never IDs.
- assistant_response should sound like a planning lead: mention what already exists, what is missing, and what you recommend doing next.
- When you propose actions, phrase assistant_response as a suggestion awaiting confirmation."#
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
        .filter(|action| action.get("type").and_then(Value::as_str).is_some())
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
            "Recent conversation:\n{}\n\nLatest user request:\n{}",
            if history.is_empty() {
                "No prior conversation."
            } else {
                &history
            },
            user_input
        ),
    });

    for _ in 0..6 {
        let completion = run_completion(db, provider_id, model_name, messages.clone()).await?;
        match parse_agent_turn(&completion)? {
            Ok(tool_call) => {
                let args = tool_call.arguments.clone().unwrap_or_else(|| json!({}));
                let tool_result = match tool_call.tool.as_str() {
                    "list_products" => list_products_tool(db).await?,
                    "get_product_tree" => {
                        get_product_tree_tool(db, args.get("productName").and_then(Value::as_str))
                            .await?
                    }
                    "list_work_items" => {
                        list_work_items_tool(
                            db,
                            args.get("productName").and_then(Value::as_str),
                            args.get("status").and_then(Value::as_str),
                        )
                        .await?
                    }
                    _ => {
                        return Err(AppError::Validation(format!(
                            "Unsupported planner tool {}",
                            tool_call.tool
                        )))
                    }
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
    Ok(info)
}

pub async fn submit_planner_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: String,
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

    let normalized = user_input.trim().to_lowercase();
    if matches!(normalized.as_str(), "yes" | "confirm" | "go ahead") {
        if let Some(plan) = session.pending_plan.clone() {
            let (execution_lines, execution_errors) = execute_plan(state, &plan).await?;
            let tree_nodes = if plan
                .actions
                .iter()
                .any(|action| action.get("type").and_then(Value::as_str) == Some("report_tree"))
            {
                build_tree_nodes(
                    &state.db,
                    plan.actions
                        .iter()
                        .find(|action| {
                            action.get("type").and_then(Value::as_str) == Some("report_tree")
                        })
                        .and_then(|action| target_field(action, "productName")),
                )
                .await
                .ok()
            } else {
                None
            };
            append_conversation(&state.db, &session_id, "user", &user_input).await?;
            session.conversation.push(PlannerConversationEntry {
                role: "user".to_string(),
                content: user_input.clone(),
            });
            append_conversation(
                &state.db,
                &session_id,
                "assistant",
                "Executed pending plan.",
            )
            .await?;
            session.conversation.push(PlannerConversationEntry {
                role: "assistant".to_string(),
                content: "Executed pending plan.".to_string(),
            });
            session.pending_plan = None;
            persist_pending_plan(&state.db, &session_id, None).await?;
            let mut service = planner_service.lock().await;
            service.save_session(&session_id, session);
            return Ok(PlannerTurnResponse {
                session_id,
                status: "execution".to_string(),
                assistant_message: "Executed pending plan.".to_string(),
                pending_plan: None,
                tree_nodes,
                execution_lines,
                execution_errors,
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

    append_conversation(&state.db, &session_id, "user", &user_input).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: user_input.clone(),
    });

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
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session);
        return Ok(PlannerTurnResponse {
            session_id,
            status: "proposal".to_string(),
            assistant_message: plan.assistant_response.clone(),
            pending_plan: Some(plan),
            tree_nodes,
            execution_lines: vec![],
            execution_errors: vec![],
        });
    }

    if plan.actions.is_empty() {
        let assistant_message = plan
            .clarification_question
            .clone()
            .unwrap_or_else(|| plan.assistant_response.clone());
        session.pending_plan = None;
        persist_pending_plan(&state.db, &session_id, None).await?;
        append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session);
        return Ok(PlannerTurnResponse {
            session_id,
            status: "clarification".to_string(),
            assistant_message,
            pending_plan: None,
            tree_nodes,
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
        execution_lines,
        execution_errors,
    })
}

pub async fn confirm_planner_plan(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
) -> Result<PlannerTurnResponse, AppError> {
    submit_planner_turn(planner_service, state, session_id, "confirm".to_string()).await
}

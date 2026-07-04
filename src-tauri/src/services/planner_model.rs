use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo, settings_repo};
use crate::planning_doctrine;
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::secrets;
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

const DEFAULT_PROVIDER_SETTING_KEY: &str = "planner.default_provider_id";
const DEFAULT_MODEL_SETTING_KEY: &str = "planner.default_model_name";

#[derive(Clone, Copy)]
pub(crate) struct PlannerModelCallContext<'a> {
    pub(crate) source_kind: &'a str,
    pub(crate) source_id: Option<&'a str>,
    pub(crate) source_label: &'a str,
    pub(crate) session_id: Option<&'a str>,
    pub(crate) product_id: Option<&'a str>,
}

pub(crate) fn planner_system_prompt() -> String {
    let base_prompt = r#"You are an AI planning lead for a product-management desktop app.
You can inspect the workspace with tools before proposing changes.
You are editing a staged design tree, not writing directly to the persisted database.
Return exactly one JSON object each turn.

If you need more context, return:
{
  "type": "tool_call",
  "tool": "get_product_tree|list_work_items",
  "arguments": {"productName": "optional product name", "status": "optional work item status", "limit": 200, "offset": 0},
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
- Behave conversationally. First reason about what already exists in the supplied context, then suggest what should be added, changed, or removed from the staged design.
- If the user is exploring or describing a need, prefer proposing actions rather than assuming immediate execution.
- If the user asks for a detailed plan, architecture, product areas, capabilities, features, stories, or tasks, prefer returning a single comprehensive proposal with all relevant create_* actions in one response instead of asking to create only the top-level product first.
- If an entity already seems to exist, do not suggest creating a duplicate unless the user explicitly asks for a separate one.
- For design edits, set needs_confirmation=false. Confirmation is only for applying the approved design later.
- Only use needs_confirmation=true if you are asking for final persistence or another risky action.
- If the request is ambiguous, set actions=[] and put the missing detail in clarification_question.
- Use tools when the request depends on current repo state or structure instead of guessing from the prompt alone.
- If a tool reports that a proposed entity does not exist yet, treat that as expected for proposal refinement and continue planning against the pending proposal instead of failing.
- Do not call mutation tools. Staged design edits go in final.actions.
- list_work_items is paginated and returns {workItems, pagination}; request the smallest useful limit and use pagination.hasMore/nextOffset only when you need another page.
- After receiving tool results, continue reasoning and either call another tool or return type=final.
- The selected product is the root. Strategy hierarchy is not editable here.
- Model the product hierarchy as Product > Product Area > Capability > Feature. Use create_product_area only as the legacy action name for product areas and create_capability for capabilities/features.
- Set product area nodeKind to product_area. Set capability nodeKind to capability under a product area, and feature under a capability.
- Features are leaves in the product management hierarchy. Delivery stories and tasks belong in work items attached to the feature.
- For book-grade technical authoring, prefer long-form fields:
  explanation, examples, implementationNotes, testGuidance.
- Use apply_capability_template when the user wants a chapter scaffold such as definition/examples/implementation/tests.
- Use convert_capability_kind when an existing staged design node should change between capability and feature. If the target kind is a leaf and the node already has structural children, set childStrategy to reparent_to_parent.
- Use these action types only:
update_product,
create_product_area, update_product_area, delete_product_area,
create_capability, update_capability, delete_capability,
apply_capability_template, convert_capability_kind,
create_work_item, update_work_item, delete_work_item,
approve_work_item, reject_work_item, approve_work_item_plan, reject_work_item_plan, approve_work_item_test_review,
start_workflow, workflow_action, report_status, report_tree.
- Use the selected product as the root. Do not create or archive products from Planner; users create products in the Products page first.
- Use product/product area/capability/work item terminology in responses; productAreaName may appear only as a legacy JSON field name for product area targets. Never expose IDs.
- assistant_response should sound like a product/design lead: mention what already exists, what changed in the staged design packet, and what should be refined next.
- Use selected node context if supplied."#;

    format!(
        "{base_prompt}\n\n{}",
        planning_doctrine::planner_model_context()
    )
}

pub(crate) async fn run_completion(
    db: &SqlitePool,
    artifact_base_path: &Path,
    provider_id: &str,
    model_name: &str,
    messages: Vec<ChatMessage>,
    context: PlannerModelCallContext<'_>,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(db, provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
    let started = Instant::now();
    let response = match gateway
        .run_completion(CompletionRequest {
            model: model_name.to_string(),
            messages: messages.clone(),
            temperature: Some(0.1),
            max_tokens: Some(1800),
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_planner_model_call(PlannerModelCallRecord {
                db,
                artifact_base_path,
                context,
                provider: &provider,
                model_name,
                messages: &messages,
                max_tokens: 1800,
                temperature: 0.1,
                response_chars: 0,
                token_count_input: None,
                token_count_output: None,
                duration_ms: elapsed_ms(started),
                status: "failed",
                error_message: Some(&error_message),
                response_text: None,
            })
            .await
            {
                tracing::warn!(error = %record_error, "Failed to record planner model call telemetry");
            }
            return Err(error);
        }
    };
    if let Err(record_error) = record_planner_model_call(PlannerModelCallRecord {
        db,
        artifact_base_path,
        context,
        provider: &provider,
        model_name,
        messages: &messages,
        max_tokens: 1800,
        temperature: 0.1,
        response_chars: char_count_i64(&response.content),
        token_count_input: response.token_count_input,
        token_count_output: response.token_count_output,
        duration_ms: elapsed_ms(started),
        status: "completed",
        error_message: None,
        response_text: Some(&response.content),
    })
    .await
    {
        tracing::warn!(error = %record_error, "Failed to record planner model call telemetry");
    }
    Ok(response.content)
}

pub(crate) async fn resolve_planner_model_binding(
    db: &SqlitePool,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<(Option<String>, Option<String>), AppError> {
    let requested_provider_id = normalize_optional_config(provider_id);
    let requested_model_name = normalize_optional_config(model_name);
    let env_provider_id =
        normalize_optional_config(std::env::var("ARUVI_PLANNER_PROVIDER_ID").ok());
    let env_model_name = normalize_optional_config(std::env::var("ARUVI_PLANNER_MODEL_NAME").ok());
    let default_provider_id = normalize_optional_config(
        settings_repo::get_setting(db, DEFAULT_PROVIDER_SETTING_KEY).await?,
    );
    let default_model_name =
        normalize_optional_config(settings_repo::get_setting(db, DEFAULT_MODEL_SETTING_KEY).await?);

    let providers = model_repo::list_providers(db).await?;
    let enabled_provider_ids = providers
        .into_iter()
        .filter(|provider| provider.enabled)
        .map(|provider| provider.id)
        .collect::<BTreeSet<_>>();
    let enabled_models = model_repo::list_model_definitions(db)
        .await?
        .into_iter()
        .filter(|model| model.enabled && enabled_provider_ids.contains(&model.provider_id))
        .collect::<Vec<_>>();

    let first_valid_pair = |pairs: Vec<(Option<String>, Option<String>)>| {
        pairs.into_iter().find_map(|(provider_id, model_name)| {
            let provider_id = provider_id?;
            let model_name = model_name?;
            enabled_models
                .iter()
                .find(|model| model.provider_id == provider_id && model.name == model_name)
                .map(|model| (Some(model.provider_id.clone()), Some(model.name.clone())))
        })
    };

    if let Some(pair) = first_valid_pair(vec![
        (requested_provider_id.clone(), requested_model_name.clone()),
        (requested_provider_id.clone(), default_model_name.clone()),
        (env_provider_id.clone(), env_model_name.clone()),
        (default_provider_id.clone(), default_model_name.clone()),
    ]) {
        return Ok(pair);
    }

    let provider_candidates = [
        requested_provider_id.clone(),
        env_provider_id.clone(),
        default_provider_id.clone(),
    ];
    for provider_id in provider_candidates.into_iter().flatten() {
        let provider_models = enabled_models
            .iter()
            .filter(|model| model.provider_id == provider_id)
            .collect::<Vec<_>>();
        if provider_models.len() == 1 {
            let model = provider_models[0];
            return Ok((Some(model.provider_id.clone()), Some(model.name.clone())));
        }
    }

    let model_candidates = [
        requested_model_name.clone(),
        env_model_name.clone(),
        default_model_name.clone(),
    ];
    for model_name in model_candidates.into_iter().flatten() {
        let matches = enabled_models
            .iter()
            .filter(|model| model.name == model_name)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let model = matches[0];
            return Ok((Some(model.provider_id.clone()), Some(model.name.clone())));
        }
    }

    if enabled_models.len() == 1 {
        let model = &enabled_models[0];
        return Ok((Some(model.provider_id.clone()), Some(model.name.clone())));
    }

    Ok((requested_provider_id, requested_model_name))
}

fn normalize_optional_config(value: Option<String>) -> Option<String> {
    value
        .map(|candidate| candidate.trim().to_string())
        .filter(|candidate| !candidate.is_empty())
}

fn char_count_i64(content: &str) -> i64 {
    i64::try_from(content.chars().count()).unwrap_or(i64::MAX)
}

fn planner_message_char_count(messages: &[ChatMessage]) -> i64 {
    messages
        .iter()
        .map(|message| char_count_i64(&message.content))
        .sum()
}

fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

struct PlannerModelCallRecord<'a> {
    db: &'a SqlitePool,
    artifact_base_path: &'a Path,
    context: PlannerModelCallContext<'a>,
    provider: &'a crate::domain::model::ModelProvider,
    model_name: &'a str,
    messages: &'a [ChatMessage],
    max_tokens: i64,
    temperature: f64,
    response_chars: i64,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
    duration_ms: i64,
    status: &'a str,
    error_message: Option<&'a str>,
    response_text: Option<&'a str>,
}

async fn record_planner_model_call(record: PlannerModelCallRecord<'_>) -> Result<(), AppError> {
    let call_index = model_call_repo::next_model_call_index(
        record.db,
        record.context.source_kind,
        record.context.source_id,
    )
    .await?;
    let call_id = uuid::Uuid::new_v4().to_string();
    let request_messages_json = serde_json::to_string_pretty(record.messages)?;
    let snapshots = model_call_repo::write_model_call_snapshots(
        record.artifact_base_path,
        &call_id,
        Some(&request_messages_json),
        record.response_text,
    )
    .await?;
    model_call_repo::create_model_call(
        record.db,
        model_call_repo::CreateModelCallParams {
            id: &call_id,
            source_kind: record.context.source_kind,
            source_id: record.context.source_id,
            source_label: record.context.source_label,
            workflow_run_id: None,
            agent_run_id: None,
            work_item_id: None,
            product_id: record.context.product_id,
            session_id: record.context.session_id,
            agent_id: None,
            stage: None,
            provider_id: &record.provider.id,
            provider_name: &record.provider.name,
            provider_type: record.provider.provider_type.as_str(),
            provider_base_url: &record.provider.base_url,
            model_id: None,
            model_name: record.model_name,
            call_index,
            request_message_count: i64::try_from(record.messages.len()).unwrap_or(i64::MAX),
            prompt_chars: planner_message_char_count(record.messages),
            response_chars: record.response_chars,
            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
            max_tokens: Some(record.max_tokens),
            temperature: Some(record.temperature),
            token_count_input: record.token_count_input,
            token_count_output: record.token_count_output,
            duration_ms: Some(record.duration_ms),
            status: record.status,
            error_message: record.error_message,
        },
    )
    .await?;
    Ok(())
}

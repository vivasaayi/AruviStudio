use crate::domain::product::Product;
use crate::error::AppError;
use crate::persistence::{planner_repo, product_repo, repository_repo};
use crate::providers::types::ChatMessage;
use crate::services::planner_action_fields::target_field;
use crate::services::planner_action_parser::parse_final_response;
use crate::services::planner_catalog::build_tree_nodes;
pub(crate) use crate::services::planner_commit::commit_draft_plan;
#[cfg(test)]
pub(crate) use crate::services::planner_draft::{
    add_draft_child_node, delete_draft_node, rename_draft_node,
};
use crate::services::planner_draft::{
    build_draft_tree_nodes, find_draft_node_by_id, parse_voice_node_reference,
    resolve_voice_draft_node_reference, summarize_selected_draft_node,
};
pub(crate) use crate::services::planner_draft_apply::apply_actions_to_draft;
pub use crate::services::planner_draft_commands::{
    add_planner_draft_child, delete_planner_draft_node, rename_planner_draft_node,
};
pub(crate) use crate::services::planner_execution::execute_plan;
use crate::services::planner_model::{
    resolve_planner_model_binding, run_completion, PlannerModelCallContext,
};
use crate::services::planner_repository_analysis::{
    build_repository_analysis_snapshot, repository_analysis_prompt, truncate_text,
};
use crate::services::planner_repository_annotation::annotate_repository_analysis_plan;
use crate::services::planner_response::{build_session_state_response, push_trace};
pub(crate) use crate::services::planner_session::persist_draft_state;
use crate::services::planner_session::{
    append_conversation, get_or_load_session, load_session_from_db, persist_pending_plan,
    PlannerConversationEntry,
};
pub use crate::services::planner_session::{PlannerService, PlannerSessionInfo};
use crate::services::planner_tool_loop::{run_tool_loop, PlannerToolLoopInput};
pub use crate::services::planner_types::{
    PlannerDraftNode, PlannerDraftPlan, PlannerPlan, PlannerTraceEvent, PlannerTreeNode,
    PlannerTurnResponse,
};
use crate::state::AppState;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

fn normalize(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

fn seed_draft_with_product(
    draft_plan: Option<PlannerDraftPlan>,
    product: &Product,
) -> PlannerDraftPlan {
    let mut draft_plan = draft_plan.unwrap_or(PlannerDraftPlan { nodes: vec![] });
    let has_matching_product_root = draft_plan.nodes.iter().any(|node| {
        node.node_type == "product"
            && node.parent_id.is_none()
            && (node.id == product.id
                || normalize(Some(node.name.as_str())) == normalize(Some(product.name.as_str())))
    });
    if has_matching_product_root {
        return draft_plan;
    }
    if draft_plan
        .nodes
        .iter()
        .any(|node| node.node_type == "product" && node.parent_id.is_none())
    {
        draft_plan.nodes.clear();
    }
    draft_plan.nodes.push(PlannerDraftNode {
        id: product.id.clone(),
        parent_id: None,
        node_type: "product".to_string(),
        name: product.name.clone(),
        summary: Some(if product.description.trim().is_empty() {
            product.vision.clone()
        } else {
            product.description.clone()
        }),
        details: json!({
            "type": "update_product",
            "name": product.name.clone(),
            "description": product.description.clone(),
            "vision": product.vision.clone(),
            "goals": product.goals.clone(),
            "tags": product.tags.clone(),
            "target": {
                "productName": product.name.clone(),
            },
        }),
    });
    draft_plan
}

fn scope_plan_to_selected_product(plan: &mut PlannerPlan, product: &Product) {
    plan.actions.retain(|action| {
        !matches!(
            action.get("type").and_then(Value::as_str),
            Some("create_product" | "archive_product")
        )
    });
    for action in &mut plan.actions {
        let Some(action_object) = action.as_object_mut() else {
            continue;
        };
        let target = action_object
            .entry("target")
            .or_insert_with(|| json!({ "productName": product.name.clone() }));
        if !target.is_object() {
            *target = json!({});
        }
        if let Some(target_object) = target.as_object_mut() {
            target_object
                .entry("productName".to_string())
                .or_insert_with(|| Value::String(product.name.clone()));
        }
    }
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
                    | "create_product_area"
                    | "create_capability"
                    | "apply_capability_template"
                    | "convert_capability_kind"
                    | "create_work_item"
                    | "update_product"
                    | "update_product_area"
                    | "update_capability"
                    | "update_work_item"
                    | "archive_product"
                    | "delete_product_area"
                    | "delete_capability"
                    | "delete_work_item"
            )
        )
    })
}

fn requires_confirmation(plan: &PlannerPlan) -> bool {
    !plan.actions.is_empty() && (plan.needs_confirmation || !is_informational_only(plan))
}

pub async fn create_planner_session(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    provider_id: Option<String>,
    model_name: Option<String>,
) -> Result<PlannerSessionInfo, AppError> {
    let (provider_id, model_name) =
        resolve_planner_model_binding(db, provider_id, model_name).await?;
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
    let (provider_id, model_name) =
        resolve_planner_model_binding(db, provider_id, model_name).await?;
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

pub async fn submit_planner_voice_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    transcript: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let spoken = transcript.trim().to_string();
    if spoken.is_empty() {
        return Err(AppError::Validation(
            "Voice transcript cannot be empty".to_string(),
        ));
    }

    let mut trace = vec![];
    let mut session = get_or_load_session(&planner_service, &state.db, &session_id).await?;
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nprovider_id={:?}\nmodel_name={:?}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.provider_id,
            session.model_name,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );

    if selected_draft_node_id != session.selected_draft_node_id {
        push_trace(
            &mut trace,
            "selection",
            "Updated selected design node",
            format!(
                "previous={:?}\nnext={:?}",
                session.selected_draft_node_id, selected_draft_node_id
            ),
        );
        session.selected_draft_node_id = selected_draft_node_id.clone();
        persist_draft_state(
            &state.db,
            &session_id,
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
    }

    let normalized_transcript = normalize(Some(&spoken));
    push_trace(
        &mut trace,
        "voice",
        "Routing planner voice transcript",
        format!(
            "transcript={}\nselected_draft_node_id={:?}",
            spoken, session.selected_draft_node_id
        ),
    );

    if matches!(
        normalized_transcript.as_str(),
        "yes"
            | "confirm"
            | "go ahead"
            | "apply"
            | "apply design"
            | "apply the design"
            | "approve"
            | "approve design"
            | "approve the design"
            | "commit"
            | "commit draft"
            | "commit the draft"
            | "confirm draft"
            | "confirm proposal"
            | "commit plan"
    ) {
        push_trace(
            &mut trace,
            "voice",
            "Matched apply-design voice command",
            spoken.clone(),
        );
        return confirm_planner_plan(planner_service, state, session_id).await;
    }

    if matches!(
        normalized_transcript.as_str(),
        "clear draft"
            | "clear the draft"
            | "clear proposal"
            | "dismiss proposal"
            | "dismiss draft"
            | "cancel draft"
    ) {
        push_trace(
            &mut trace,
            "voice",
            "Matched clear voice command",
            spoken.clone(),
        );
        let had_state = session.pending_plan.is_some() || session.draft_plan.is_some();
        session.pending_plan = None;
        session.draft_plan = None;
        session.selected_draft_node_id = None;
        append_conversation(&state.db, &session_id, "user", &spoken).await?;
        let assistant_message = if had_state {
            "Cleared the current staged design.".to_string()
        } else {
            "There is no active design to clear.".to_string()
        };
        append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
        session.conversation.push(PlannerConversationEntry {
            role: "user".to_string(),
            content: spoken,
        });
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });
        persist_pending_plan(&state.db, &session_id, None).await?;
        persist_draft_state(&state.db, &session_id, None, None).await?;
        {
            let mut service = planner_service.lock().await;
            service.save_session(&session_id, session.clone());
        }
        return Ok(build_session_state_response(
            session_id,
            "execution",
            assistant_message.clone(),
            &session,
            vec![assistant_message],
            vec![],
            trace,
        ));
    }

    let is_draft_view_command = matches!(
        normalized_transcript.as_str(),
        "view draft"
            | "open draft"
            | "show draft"
            | "show draft tree"
            | "view draft tree"
            | "view design"
            | "open design"
            | "show design"
            | "show design tree"
            | "view design tree"
            | "open workspace"
            | "show workspace"
            | "expand draft"
            | "expand the draft"
            | "expand design"
            | "expand the design"
            | "expand tree"
    );
    let is_draft_selection_command = ["select ", "choose ", "highlight ", "open ", "expand "]
        .iter()
        .any(|prefix| normalized_transcript.starts_with(prefix));

    if session.draft_plan.is_none() && (is_draft_view_command || is_draft_selection_command) {
        let assistant_message = "There is no staged design tree yet.".to_string();
        append_conversation(&state.db, &session_id, "user", &spoken).await?;
        append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
        session.conversation.push(PlannerConversationEntry {
            role: "user".to_string(),
            content: spoken,
        });
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });
        {
            let mut service = planner_service.lock().await;
            service.save_session(&session_id, session.clone());
        }
        return Ok(build_session_state_response(
            session_id,
            "session_update",
            assistant_message.clone(),
            &session,
            vec![assistant_message],
            vec![],
            trace,
        ));
    }

    if let Some(draft_plan) = session.draft_plan.clone() {
        if is_draft_view_command {
            let target =
                find_draft_node_by_id(&draft_plan, session.selected_draft_node_id.as_deref())
                    .or_else(|| {
                        draft_plan
                            .nodes
                            .iter()
                            .find(|node| node.node_type == "product")
                    });
            let assistant_message = target
                .map(|node| summarize_selected_draft_node(&draft_plan, node))
                .unwrap_or_else(|| "There is no staged design tree yet.".to_string());
            append_conversation(&state.db, &session_id, "user", &spoken).await?;
            append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
            session.conversation.push(PlannerConversationEntry {
                role: "user".to_string(),
                content: spoken,
            });
            session.conversation.push(PlannerConversationEntry {
                role: "assistant".to_string(),
                content: assistant_message.clone(),
            });
            {
                let mut service = planner_service.lock().await;
                service.save_session(&session_id, session.clone());
            }
            return Ok(build_session_state_response(
                session_id,
                "session_update",
                assistant_message.clone(),
                &session,
                vec![assistant_message],
                vec![],
                trace,
            ));
        }

        let command_matchers = ["select ", "choose ", "highlight ", "open ", "expand "];
        let matched_remainder = command_matchers.iter().find_map(|prefix| {
            normalized_transcript
                .strip_prefix(prefix)
                .map(|_| spoken[prefix.len()..].trim().to_string())
        });

        if let Some(reference_text) = matched_remainder {
            push_trace(
                &mut trace,
                "voice",
                "Matched design selection voice command",
                reference_text.clone(),
            );
            let (explicit_type, reference) = parse_voice_node_reference(&reference_text);
            let resolved = resolve_voice_draft_node_reference(
                &draft_plan,
                session.selected_draft_node_id.as_deref(),
                &reference,
                explicit_type,
            );
            let target_node = match resolved {
                Ok(Some(node)) => node,
                Ok(None) => {
                    let assistant_message = format!(
                        "I could not find a design node matching \"{}\".",
                        reference_text
                    );
                    append_conversation(&state.db, &session_id, "user", &spoken).await?;
                    append_conversation(&state.db, &session_id, "assistant", &assistant_message)
                        .await?;
                    session.conversation.push(PlannerConversationEntry {
                        role: "user".to_string(),
                        content: spoken,
                    });
                    session.conversation.push(PlannerConversationEntry {
                        role: "assistant".to_string(),
                        content: assistant_message.clone(),
                    });
                    {
                        let mut service = planner_service.lock().await;
                        service.save_session(&session_id, session.clone());
                    }
                    return Ok(build_session_state_response(
                        session_id,
                        "session_update",
                        assistant_message.clone(),
                        &session,
                        vec![assistant_message],
                        vec![],
                        trace,
                    ));
                }
                Err(error) => {
                    let assistant_message = error.to_string();
                    append_conversation(&state.db, &session_id, "user", &spoken).await?;
                    append_conversation(&state.db, &session_id, "assistant", &assistant_message)
                        .await?;
                    session.conversation.push(PlannerConversationEntry {
                        role: "user".to_string(),
                        content: spoken,
                    });
                    session.conversation.push(PlannerConversationEntry {
                        role: "assistant".to_string(),
                        content: assistant_message.clone(),
                    });
                    {
                        let mut service = planner_service.lock().await;
                        service.save_session(&session_id, session.clone());
                    }
                    return Ok(build_session_state_response(
                        session_id,
                        "session_update",
                        assistant_message.clone(),
                        &session,
                        vec![],
                        vec![assistant_message],
                        trace,
                    ));
                }
            };

            session.selected_draft_node_id = Some(target_node.id.clone());
            persist_draft_state(
                &state.db,
                &session_id,
                Some(&draft_plan),
                session.selected_draft_node_id.as_deref(),
            )
            .await?;
            let mut assistant_message = summarize_selected_draft_node(&draft_plan, &target_node);
            if session.pending_plan.is_some() {
                assistant_message.push_str(" The current draft proposal is still staged; say confirm when you want to commit it.");
            }
            append_conversation(&state.db, &session_id, "user", &spoken).await?;
            append_conversation(&state.db, &session_id, "assistant", &assistant_message).await?;
            session.conversation.push(PlannerConversationEntry {
                role: "user".to_string(),
                content: spoken,
            });
            session.conversation.push(PlannerConversationEntry {
                role: "assistant".to_string(),
                content: assistant_message.clone(),
            });
            {
                let mut service = planner_service.lock().await;
                service.save_session(&session_id, session.clone());
            }
            return Ok(build_session_state_response(
                session_id,
                "session_update",
                assistant_message.clone(),
                &session,
                vec![assistant_message],
                vec![],
                trace,
            ));
        }
    }

    push_trace(
        &mut trace,
        "voice",
        "No deterministic voice command matched",
        "Falling back to the planner turn pipeline.",
    );
    submit_planner_turn(
        planner_service,
        state,
        session_id,
        spoken,
        session.selected_draft_node_id.clone(),
        selected_product_id,
    )
    .await
}

pub async fn submit_planner_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let mut trace = vec![];
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
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nprovider_id={:?}\nmodel_name={:?}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.provider_id,
            session.model_name,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );

    if selected_draft_node_id != session.selected_draft_node_id {
        push_trace(
            &mut trace,
            "selection",
            "Updated selected design node",
            format!(
                "previous={:?}\nnext={:?}",
                session.selected_draft_node_id, selected_draft_node_id
            ),
        );
        session.selected_draft_node_id = selected_draft_node_id.clone();
        persist_draft_state(
            &state.db,
            &session_id,
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
    }

    let selected_product = if let Some(product_id) = selected_product_id.as_deref() {
        Some(product_repo::get_product(&state.db, product_id).await?)
    } else {
        None
    };

    let normalized = user_input.trim().to_lowercase();
    if matches!(normalized.as_str(), "yes" | "confirm" | "go ahead") {
        if let Some(draft_plan) = session.draft_plan.clone() {
            push_trace(
                &mut trace,
                "commit",
                "Attempting draft commit",
                serde_json::to_string_pretty(&draft_plan)?,
            );
            let execution_lines = match commit_draft_plan(state, &draft_plan).await {
                Ok(lines) => lines,
                Err(error) => {
                    push_trace(
                        &mut trace,
                        "error",
                        "Draft commit failed",
                        error.to_string(),
                    );
                    return Ok(PlannerTurnResponse {
                        session_id,
                        status: "error".to_string(),
                        assistant_message: error.to_string(),
                        pending_plan: session.pending_plan.clone(),
                        tree_nodes: None,
                        draft_tree_nodes: session.draft_plan.as_ref().map(|draft| {
                            build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())
                        }),
                        selected_draft_node_id: session.selected_draft_node_id.clone(),
                        execution_lines: vec![],
                        execution_errors: vec![error.to_string()],
                        trace_events: trace,
                    });
                }
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
                "Applied design to catalog.",
            )
            .await?;
            session.conversation.push(PlannerConversationEntry {
                role: "assistant".to_string(),
                content: "Applied design to catalog.".to_string(),
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
                assistant_message: "Applied design to catalog.".to_string(),
                pending_plan: None,
                tree_nodes: None,
                draft_tree_nodes: None,
                selected_draft_node_id: None,
                execution_lines,
                execution_errors: vec![],
                trace_events: trace,
            });
        }
    }

    if selected_product.is_none() && session.draft_plan.is_none() {
        return Ok(PlannerTurnResponse {
            session_id,
            status: "clarification".to_string(),
            assistant_message: "Select a product before planning. Create the product in Products first, then return to Planner.".to_string(),
            pending_plan: session.pending_plan.clone(),
            tree_nodes: None,
            draft_tree_nodes: None,
            selected_draft_node_id: session.selected_draft_node_id.clone(),
            execution_lines: vec![],
            execution_errors: vec![],
            trace_events: trace,
        });
    }

    if let Some(product) = selected_product.as_ref() {
        let seeded_draft = seed_draft_with_product(session.draft_plan.clone(), product);
        session.draft_plan = Some(seeded_draft);
        if session.selected_draft_node_id.is_none() {
            session.selected_draft_node_id = Some(product.id.clone());
        }
        persist_draft_state(
            &state.db,
            &session_id,
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
    }

    let mut plan = if let (Some(provider_id), Some(model_name)) =
        (session.provider_id.clone(), session.model_name.clone())
    {
        match run_tool_loop(
            PlannerToolLoopInput {
                db: &state.db,
                artifact_base_path: &state.artifact_base_path,
                session_id: &session_id,
                provider_id: &provider_id,
                model_name: &model_name,
                conversation: &session.conversation,
                pending_plan: session.pending_plan.as_ref(),
                draft_plan: session.draft_plan.as_ref(),
                selected_draft_node_id: session.selected_draft_node_id.as_deref(),
                selected_product: selected_product.as_ref(),
                user_input: &user_input,
            },
            &mut trace,
        )
        .await
        {
            Ok(plan) => plan,
            Err(error) => {
                push_trace(
                    &mut trace,
                    "error",
                    "Planner tool loop failed",
                    error.to_string(),
                );
                return Ok(PlannerTurnResponse {
                    session_id,
                    status: "error".to_string(),
                    assistant_message: error.to_string(),
                    pending_plan: session.pending_plan.clone(),
                    tree_nodes: None,
                    draft_tree_nodes: session.draft_plan.as_ref().map(|draft| {
                        build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())
                    }),
                    selected_draft_node_id: session.selected_draft_node_id.clone(),
                    execution_lines: vec![],
                    execution_errors: vec![error.to_string()],
                    trace_events: trace,
                });
            }
        }
    } else {
        push_trace(
            &mut trace,
            "fallback",
            "Using heuristic planner",
            "No configured provider/model for planner session.",
        );
        heuristic_plan(&user_input)
    };
    if let Some(product) = selected_product.as_ref() {
        scope_plan_to_selected_product(&mut plan, product);
    }
    push_trace(
        &mut trace,
        "plan",
        "Planner plan ready",
        serde_json::to_string_pretty(&plan)?,
    );

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

    let draft_tree_nodes = session.draft_plan.as_ref().map(|draft_plan| {
        build_draft_tree_nodes(draft_plan, session.selected_draft_node_id.as_deref())
    });

    append_conversation(&state.db, &session_id, "user", &user_input).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: user_input.clone(),
    });

    if has_draft_mutations(&plan) {
        push_trace(
            &mut trace,
            "draft",
            "Applying actions to staged design",
            serde_json::to_string_pretty(&plan.actions)?,
        );
        let updated_draft = match apply_actions_to_draft(
            session.draft_plan.clone(),
            session.selected_draft_node_id.as_deref(),
            &plan.actions,
        ) {
            Ok(draft) => draft,
            Err(error) => {
                push_trace(
                    &mut trace,
                    "error",
                    "Draft mutation failed",
                    error.to_string(),
                );
                return Ok(PlannerTurnResponse {
                    session_id,
                    status: "error".to_string(),
                    assistant_message: error.to_string(),
                    pending_plan: session.pending_plan.clone(),
                    tree_nodes,
                    draft_tree_nodes: session.draft_plan.as_ref().map(|draft| {
                        build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())
                    }),
                    selected_draft_node_id: session.selected_draft_node_id.clone(),
                    execution_lines: vec![],
                    execution_errors: vec![error.to_string()],
                    trace_events: trace,
                });
            }
        };
        let updated_draft_tree_nodes = Some(build_draft_tree_nodes(
            &updated_draft,
            session.selected_draft_node_id.as_deref(),
        ));
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
        service.save_session(&session_id, session.clone());
        return Ok(PlannerTurnResponse {
            session_id,
            status: "proposal".to_string(),
            assistant_message: plan.assistant_response.clone(),
            pending_plan: Some(plan),
            tree_nodes,
            draft_tree_nodes: updated_draft_tree_nodes,
            selected_draft_node_id: session.selected_draft_node_id.clone(),
            execution_lines: vec!["Updated the design plan.".to_string()],
            execution_errors: vec![],
            trace_events: trace,
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
            trace_events: trace,
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
            trace_events: trace,
        });
    }

    push_trace(
        &mut trace,
        "execution",
        "Executing planner actions immediately",
        serde_json::to_string_pretty(&plan.actions)?,
    );
    let (execution_lines, execution_errors) = execute_plan(state, &plan).await?;
    if !execution_errors.is_empty() {
        push_trace(
            &mut trace,
            "execution",
            "Execution errors",
            execution_errors.join("\n"),
        );
    }
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
        trace_events: trace,
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
        None,
    )
    .await
}

pub async fn analyze_repository_for_planner(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    artifact_base_path: &Path,
    session_id: String,
    repository_id: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let mut trace = vec![];
    let mut session = get_or_load_session(&planner_service, db, &session_id).await?;
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nprovider_id={:?}\nmodel_name={:?}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.provider_id,
            session.model_name,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );

    if selected_draft_node_id != session.selected_draft_node_id {
        session.selected_draft_node_id = selected_draft_node_id.clone();
        persist_draft_state(
            db,
            &session_id,
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
        )
        .await?;
    }

    let provider_id = session.provider_id.clone().ok_or_else(|| {
        AppError::Validation("Configure a planner model before analyzing a repository.".to_string())
    })?;
    let model_name = session.model_name.clone().ok_or_else(|| {
        AppError::Validation("Configure a planner model before analyzing a repository.".to_string())
    })?;
    let selected_product_id = selected_product_id.ok_or_else(|| {
        AppError::Validation("Select a product before analyzing a repository.".to_string())
    })?;
    let selected_product = product_repo::get_product(db, &selected_product_id).await?;
    let seeded_draft = seed_draft_with_product(session.draft_plan.clone(), &selected_product);
    session.draft_plan = Some(seeded_draft);
    if session.selected_draft_node_id.is_none() {
        session.selected_draft_node_id = Some(selected_product.id.clone());
    }
    persist_draft_state(
        db,
        &session_id,
        session.draft_plan.as_ref(),
        session.selected_draft_node_id.as_deref(),
    )
    .await?;
    let repository = repository_repo::get_repository(db, &repository_id).await?;
    let repo_snapshot = build_repository_analysis_snapshot(&repository)?;
    let repo_snapshot_json = serde_json::to_string_pretty(&repo_snapshot)?;
    push_trace(
        &mut trace,
        "repository",
        "Captured structured repository analysis snapshot",
        truncate_text(&repo_snapshot_json, 12_000),
    );

    let draft_context = session
        .draft_plan
        .as_ref()
        .map(serde_json::to_string_pretty)
        .transpose()?
        .unwrap_or_else(|| "No staged design yet.".to_string());
    let selected_context = session
        .selected_draft_node_id
        .as_deref()
        .and_then(|node_id| {
            session
                .draft_plan
                .as_ref()
                .and_then(|draft| draft.nodes.iter().find(|node| node.id == node_id))
        })
        .map(serde_json::to_string_pretty)
        .transpose()?
        .unwrap_or_else(|| "No design node selected.".to_string());
    let analysis_request = format!(
        "Selected product:\n{}\n\nCurrent staged design tree:\n{}\n\nSelected design node:\n{}\n\nStructured repository analysis snapshot:\n{}\n\nTask:\nReverse engineer this repository into the selected product's staged design tree. Infer product areas, capabilities, features, and starter work items from the codebase. Use the structured evidence first, and only make cautious inferences when the evidence is incomplete. Merge into the selected design node if it exists; otherwise add structure under the selected product root. Do not create a new product.",
        serde_json::to_string_pretty(&selected_product)?, draft_context, selected_context, repo_snapshot_json
    );
    push_trace(
        &mut trace,
        "input",
        "Repository analysis request",
        truncate_text(&analysis_request, 12_000),
    );

    let completion = run_completion(
        db,
        artifact_base_path,
        &provider_id,
        &model_name,
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: repository_analysis_prompt().to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: analysis_request,
            },
        ],
        PlannerModelCallContext {
            source_kind: "desktop_repository_analysis",
            source_id: Some(&session_id),
            source_label: "Desktop Repository Analysis",
            session_id: Some(&session_id),
            product_id: Some(selected_product.id.as_str()),
        },
    )
    .await?;
    push_trace(
        &mut trace,
        "model",
        "Repository analysis completion",
        completion.clone(),
    );

    let mut plan = parse_final_response(&completion)?;
    annotate_repository_analysis_plan(&repo_snapshot, &mut plan);
    scope_plan_to_selected_product(&mut plan, &selected_product);
    push_trace(
        &mut trace,
        "plan",
        "Parsed repository analysis plan",
        serde_json::to_string_pretty(&plan)?,
    );

    if plan.actions.is_empty() {
        return Ok(PlannerTurnResponse {
            session_id,
            status: "clarification".to_string(),
            assistant_message: plan
                .clarification_question
                .clone()
                .unwrap_or_else(|| plan.assistant_response.clone()),
            pending_plan: Some(plan),
            tree_nodes: None,
            draft_tree_nodes: session.draft_plan.as_ref().map(|draft| {
                build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())
            }),
            selected_draft_node_id: session.selected_draft_node_id.clone(),
            execution_lines: vec![],
            execution_errors: vec![],
            trace_events: trace,
        });
    }

    let updated_draft = apply_actions_to_draft(
        session.draft_plan.clone(),
        session.selected_draft_node_id.as_deref(),
        &plan.actions,
    )?;
    session.draft_plan = Some(updated_draft.clone());
    session.pending_plan = Some(plan.clone());
    persist_pending_plan(db, &session_id, Some(&plan)).await?;
    persist_draft_state(
        db,
        &session_id,
        Some(&updated_draft),
        session.selected_draft_node_id.as_deref(),
    )
    .await?;
    append_conversation(
        db,
        &session_id,
        "user",
        &format!(
            "Analyze repository {} into a design packet.",
            repository.name
        ),
    )
    .await?;
    append_conversation(db, &session_id, "assistant", &plan.assistant_response).await?;
    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: format!(
            "Analyze repository {} into a design packet.",
            repository.name
        ),
    });
    session.conversation.push(PlannerConversationEntry {
        role: "assistant".to_string(),
        content: plan.assistant_response.clone(),
    });
    {
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
    }

    Ok(PlannerTurnResponse {
        session_id,
        status: "proposal".to_string(),
        assistant_message: plan.assistant_response.clone(),
        pending_plan: Some(plan),
        tree_nodes: None,
        draft_tree_nodes: Some(build_draft_tree_nodes(
            &updated_draft,
            session.selected_draft_node_id.as_deref(),
        )),
        selected_draft_node_id: session.selected_draft_node_id,
        execution_lines: vec!["Updated the design plan from repository analysis.".to_string()],
        execution_errors: vec![],
        trace_events: trace,
    })
}

#[cfg(test)]
mod tests;

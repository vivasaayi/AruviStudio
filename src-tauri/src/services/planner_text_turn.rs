use crate::error::AppError;
use crate::persistence::product_repo;
use crate::services::planner_action_fields::target_field;
use crate::services::planner_catalog::build_tree_nodes;
use crate::services::planner_commit::commit_draft_plan;
use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_draft_apply::apply_actions_to_draft;
use crate::services::planner_execution::execute_plan;
use crate::services::planner_response::push_trace;
use crate::services::planner_session::{
    append_conversation, get_or_load_session, persist_draft_state, persist_pending_plan,
    PlannerConversationEntry, PlannerService,
};
use crate::services::planner_text_turn_response::{
    clarification_response, planner_error_response, proposal_response, selection_required_response,
    session_draft_tree_nodes,
};
use crate::services::planner_tool_loop::{run_tool_loop, PlannerToolLoopInput};
use crate::services::planner_turn_policy::{
    has_draft_mutations, heuristic_plan, is_informational_only, requires_confirmation,
    scope_plan_to_selected_product, seed_draft_with_product,
};
use crate::services::planner_types::PlannerTurnResponse;
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn submit_planner_turn(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: String,
    selected_draft_node_id: Option<String>,
    selected_product_id: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
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
                    return Ok(planner_error_response(
                        session_id,
                        &session,
                        error.to_string(),
                        None,
                        trace,
                    ));
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
        return Ok(selection_required_response(session_id, &session, trace));
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
                return Ok(planner_error_response(
                    session_id,
                    &session,
                    error.to_string(),
                    None,
                    trace,
                ));
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

    let draft_tree_nodes = session_draft_tree_nodes(&session);

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
                return Ok(planner_error_response(
                    session_id,
                    &session,
                    error.to_string(),
                    tree_nodes,
                    trace,
                ));
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
        return Ok(proposal_response(
            session_id,
            plan.assistant_response.clone(),
            plan,
            tree_nodes,
            updated_draft_tree_nodes,
            session.selected_draft_node_id.clone(),
            vec!["Updated the design plan.".to_string()],
            trace,
        ));
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
        return Ok(proposal_response(
            session_id,
            plan.assistant_response.clone(),
            plan,
            tree_nodes,
            draft_tree_nodes,
            selected_draft_node_id,
            vec![],
            trace,
        ));
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
        return Ok(clarification_response(
            session_id,
            assistant_message,
            pending_plan,
            tree_nodes,
            draft_tree_nodes,
            selected_draft_node_id,
            trace,
        ));
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

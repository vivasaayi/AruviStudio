use crate::error::AppError;
use crate::services::planner_draft::find_draft_node_by_id;
use crate::services::planner_draft_voice::{
    parse_voice_node_reference, resolve_voice_draft_node_reference, summarize_selected_draft_node,
};
use crate::services::planner_response::{build_session_state_response, push_trace};
use crate::services::planner_service::{confirm_planner_plan, submit_planner_turn};
use crate::services::planner_session::{
    append_conversation, get_or_load_session, persist_draft_state, persist_pending_plan,
    PlannerConversationEntry, PlannerService,
};
use crate::services::planner_turn_policy::normalize;
use crate::services::planner_types::PlannerTurnResponse;
use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

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

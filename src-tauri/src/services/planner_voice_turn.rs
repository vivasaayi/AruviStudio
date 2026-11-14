use crate::error::AppError;
use crate::services::planner_draft::find_draft_node_by_id;
use crate::services::planner_draft_voice::{
    parse_voice_node_reference, resolve_voice_draft_node_reference, summarize_selected_draft_node,
};
use crate::services::planner_response::{build_session_state_response, push_trace};
use crate::services::planner_session::{
    get_or_load_session, lock_planner_session, persist_draft_state, persist_session_exchange,
    PlannerConversationEntry, PlannerService,
};
use crate::services::planner_text_turn::submit_planner_turn_locked;
use crate::services::planner_turn_policy::normalize;

fn exact_voice_command_text(normalized: &str) -> &str {
    normalized
        .strip_suffix('.')
        .unwrap_or(normalized)
        .trim_end()
}

#[cfg(test)]
mod command_text_tests {
    use super::exact_voice_command_text;

    #[test]
    fn ignores_one_final_period_without_treating_questions_as_commands() {
        assert_eq!(exact_voice_command_text("confirm."), "confirm");
        assert_eq!(exact_voice_command_text("confirm?"), "confirm?");
        assert_eq!(exact_voice_command_text("confirm.."), "confirm.");
    }
}
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

    let _session_guard = lock_planner_session(&planner_service, &state.db, &session_id).await?;
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
    // Speech recognition commonly adds a terminal period. Strip one only for
    // exact commands; questions and named-node references retain their text.
    let normalized_command = exact_voice_command_text(&normalized_transcript);
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
        normalized_command,
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
        return submit_planner_turn_locked(
            planner_service,
            state,
            session_id,
            "confirm".to_string(),
            None,
            None,
        )
        .await;
    }

    if matches!(
        normalized_command,
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
        let assistant_message = if had_state {
            "Cleared the current staged design.".to_string()
        } else {
            "There is no active design to clear.".to_string()
        };
        persist_session_exchange(
            &state.db,
            &session_id,
            session.pending_plan.as_ref(),
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
            &spoken,
            &assistant_message,
        )
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
            "execution",
            assistant_message.clone(),
            &session,
            vec![assistant_message],
            vec![],
            trace,
        ));
    }

    let view_hint = match normalized_command {
        "view trace" | "show trace" | "open trace" => Some("trace"),
        "expand draft" | "expand the draft" | "expand tree" | "expand all"
        | "open all branches" | "expand design" | "expand the design" => Some("expand_draft"),
        "collapse draft" | "collapse the draft" | "collapse tree" | "collapse all" | "close draft" | "close tree" | "close all" => Some("collapse_draft"),
        "view conversation" | "open conversation" | "show conversation" | "back to chat"
        | "view chat" => Some("conversation"),
        _ => None,
    };
    if let Some(view) = view_hint {
        let assistant_message = match view {
            "trace" => "Showing the planner trace.",
            "expand_draft" if session.draft_plan.is_none() => "There is no staged design tree yet.",
            "collapse_draft" if session.draft_plan.is_none() => "There is no staged design tree yet.",
            "expand_draft" => "Expanded the staged design tree.",
            "collapse_draft" => "Collapsed the staged design tree.",
            _ => "Showing the planner conversation.",
        }
        .to_string();
        persist_session_exchange(
            &state.db,
            &session_id,
            session.pending_plan.as_ref(),
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
            &spoken,
            &assistant_message,
        )
        .await?;
        session.conversation.push(PlannerConversationEntry {
            role: "user".to_string(),
            content: spoken,
        });
        session.conversation.push(PlannerConversationEntry {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });
        planner_service
            .lock()
            .await
            .save_session(&session_id, session.clone());
        let mut response = build_session_state_response(
            session_id,
            "session_update",
            assistant_message,
            &session,
            vec![],
            vec![],
            trace,
        );
        response.view_hint = Some(view.to_string());
        return Ok(response);
    }

    let is_draft_view_command = matches!(
        normalized_command,
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
    let is_draft_selection_command = ["select ", "choose ", "highlight ", "open ", "expand ", "collapse ", "close "]
        .iter()
        .any(|prefix| normalized_transcript.starts_with(prefix));

    if session.draft_plan.is_none() && (is_draft_view_command || is_draft_selection_command) {
        let assistant_message = "There is no staged design tree yet.".to_string();
        persist_session_exchange(
            &state.db,
            &session_id,
            session.pending_plan.as_ref(),
            session.draft_plan.as_ref(),
            session.selected_draft_node_id.as_deref(),
            &spoken,
            &assistant_message,
        )
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
            persist_session_exchange(
                &state.db,
                &session_id,
                session.pending_plan.as_ref(),
                session.draft_plan.as_ref(),
                session.selected_draft_node_id.as_deref(),
                &spoken,
                &assistant_message,
            )
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
            let mut response = build_session_state_response(
                session_id,
                "session_update",
                assistant_message.clone(),
                &session,
                vec![assistant_message],
                vec![],
                trace,
            );
            response.view_hint = Some("draft".to_string());
            return Ok(response);
        }

        let command_matchers = ["select ", "choose ", "highlight ", "open ", "expand ", "collapse ", "close "];
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
                    persist_session_exchange(
                        &state.db,
                        &session_id,
                        session.pending_plan.as_ref(),
                        session.draft_plan.as_ref(),
                        session.selected_draft_node_id.as_deref(),
                        &spoken,
                        &assistant_message,
                    )
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
                    persist_session_exchange(
                        &state.db,
                        &session_id,
                        session.pending_plan.as_ref(),
                        session.draft_plan.as_ref(),
                        session.selected_draft_node_id.as_deref(),
                        &spoken,
                        &assistant_message,
                    )
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

            let tree_action = if normalized_transcript.starts_with("expand ") {
                Some("expand_node")
            } else if normalized_transcript.starts_with("collapse ") || normalized_transcript.starts_with("close ") {
                Some("collapse_node")
            } else {
                None
            };
            if let Some(action) = tree_action {
                let assistant_message = format!("{} {}.", if action == "expand_node" { "Expanded" } else { "Collapsed" }, target_node.name);
                persist_session_exchange(&state.db, &session_id, session.pending_plan.as_ref(), session.draft_plan.as_ref(), session.selected_draft_node_id.as_deref(), &spoken, &assistant_message).await?;
                session.conversation.push(PlannerConversationEntry { role: "user".to_string(), content: spoken });
                session.conversation.push(PlannerConversationEntry { role: "assistant".to_string(), content: assistant_message.clone() });
                planner_service.lock().await.save_session(&session_id, session.clone());
                let mut response = build_session_state_response(session_id, "session_update", assistant_message, &session, vec![], vec![], trace);
                response.view_hint = Some(action.to_string());
                response.view_node_id = Some(target_node.id);
                return Ok(response);
            }

            session.selected_draft_node_id = Some(target_node.id.clone());
            let mut assistant_message = summarize_selected_draft_node(&draft_plan, &target_node);
            if session.pending_plan.is_some() {
                assistant_message.push_str(" The current draft proposal is still staged; say confirm when you want to commit it.");
            }
            persist_session_exchange(
                &state.db,
                &session_id,
                session.pending_plan.as_ref(),
                session.draft_plan.as_ref(),
                session.selected_draft_node_id.as_deref(),
                &spoken,
                &assistant_message,
            )
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
            let mut response = build_session_state_response(
                session_id,
                "session_update",
                assistant_message.clone(),
                &session,
                vec![assistant_message],
                vec![],
                trace,
            );
            response.view_hint = Some("draft".to_string());
            return Ok(response);
        }
    }

    push_trace(
        &mut trace,
        "voice",
        "No deterministic voice command matched",
        "Falling back to the planner turn pipeline.",
    );
    submit_planner_turn_locked(
        planner_service,
        state,
        session_id,
        spoken,
        session.selected_draft_node_id.clone(),
        selected_product_id,
    )
    .await
}

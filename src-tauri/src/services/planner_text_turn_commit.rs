use crate::error::AppError;
use crate::services::planner_commit::commit_draft_plan_in_conn;
use crate::services::planner_response::push_trace;
use crate::services::planner_session::{
    PlannerConversationEntry, PlannerService, PlannerSession,
};
use crate::services::planner_text_turn_response::planner_error_response;
use crate::services::planner_types::{PlannerTraceEvent, PlannerTurnResponse};
use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) async fn commit_confirmed_draft_plan(
    planner_service: Arc<Mutex<PlannerService>>,
    state: &AppState,
    session_id: String,
    user_input: &str,
    mut session: PlannerSession,
    mut trace: Vec<PlannerTraceEvent>,
) -> Result<PlannerTurnResponse, AppError> {
    let Some(draft_plan) = session.draft_plan.clone() else {
        return Ok(planner_error_response(
            session_id,
            &session,
            "No staged design is available to apply.".to_string(),
            None,
            trace,
        ));
    };

    push_trace(
        &mut trace,
        "commit",
        "Attempting draft commit",
        serde_json::to_string_pretty(&draft_plan)?,
    );
    let mut tx = state.db.begin().await?;
    let execution_lines = match commit_draft_plan_in_conn(&mut tx, &draft_plan).await {
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
    for (role, content) in [
        ("user", user_input),
        ("assistant", "Applied design to catalog."),
    ] {
        sqlx::query(
            "INSERT INTO planner_conversation_entries (id, session_id, role, content) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&session_id)
        .bind(role)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    }
    let updated = sqlx::query(
        "UPDATE planner_sessions SET pending_plan_json = NULL, draft_plan_json = NULL, selected_draft_node_id = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&session_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("Planner session {session_id} not found")));
    }
    tx.commit().await?;

    session.conversation.push(PlannerConversationEntry {
        role: "user".to_string(),
        content: user_input.to_string(),
    });
    session.conversation.push(PlannerConversationEntry {
        role: "assistant".to_string(),
        content: "Applied design to catalog.".to_string(),
    });
    session.pending_plan = None;
    session.draft_plan = None;
    session.selected_draft_node_id = None;
    let mut service = planner_service.lock().await;
    service.save_session(&session_id, session);
    Ok(PlannerTurnResponse {
        view_hint: None,
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
    })
}

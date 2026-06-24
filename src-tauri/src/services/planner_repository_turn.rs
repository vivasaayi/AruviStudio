use crate::error::AppError;
use crate::persistence::{product_repo, repository_repo};
use crate::providers::types::ChatMessage;
use crate::services::planner_action_parser::parse_final_response;
use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_draft_apply::apply_actions_to_draft;
use crate::services::planner_model::{run_completion, PlannerModelCallContext};
use crate::services::planner_repository_analysis::{
    build_repository_analysis_snapshot, repository_analysis_prompt, truncate_text,
};
use crate::services::planner_repository_annotation::annotate_repository_analysis_plan;
use crate::services::planner_response::push_trace;
use crate::services::planner_session::{
    append_conversation, get_or_load_session, persist_draft_state, persist_pending_plan,
    PlannerConversationEntry, PlannerService,
};
use crate::services::planner_turn_policy::{
    scope_plan_to_selected_product, seed_draft_with_product,
};
use crate::services::planner_types::PlannerTurnResponse;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        serde_json::to_string_pretty(&selected_product)?,
        draft_context,
        selected_context,
        repo_snapshot_json
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

use crate::error::AppError;
use crate::services::planner_draft::{
    build_draft_tree_nodes, find_draft_ancestor_name, find_draft_node_by_id,
};
use crate::services::planner_draft_mutation::{
    add_draft_child_node, delete_draft_node, rename_draft_node,
};
use crate::services::planner_service::{PlannerPlan, PlannerTraceEvent, PlannerTurnResponse};
use crate::services::planner_session::{
    get_or_load_session, persist_draft_state, persist_pending_plan, PlannerService,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

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

pub async fn rename_planner_draft_node(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    node_id: String,
    next_name: String,
) -> Result<PlannerTurnResponse, AppError> {
    let mut trace = vec![];
    let mut session = get_or_load_session(&planner_service, db, &session_id).await?;
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );
    let mut draft_plan = session
        .draft_plan
        .clone()
        .ok_or_else(|| AppError::Validation("No staged design is available".to_string()))?;
    let previous = find_draft_node_by_id(&draft_plan, Some(&node_id))
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))?;
    push_trace(
        &mut trace,
        "draft",
        "Renaming design node",
        format!(
            "node_id={}\ntype={}\nprevious_name={}\nnext_name={}",
            node_id, previous.node_type, previous.name, next_name
        ),
    );
    let renamed = rename_draft_node(&mut draft_plan, &node_id, &next_name)?;
    session.draft_plan = Some(draft_plan.clone());
    session.selected_draft_node_id = Some(renamed.id.clone());
    session.pending_plan = None;
    persist_pending_plan(db, &session_id, None).await?;
    persist_draft_state(
        db,
        &session_id,
        Some(&draft_plan),
        session.selected_draft_node_id.as_deref(),
    )
    .await?;
    {
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
    }
    let action = match renamed.node_type.as_str() {
        "product" => json!({
            "type": "update_product",
            "target": { "productName": previous.name },
            "fields": { "name": renamed.name }
        }),
        "product_area" => json!({
            "type": "update_product_area",
            "target": {
                "productName": find_draft_ancestor_name(&draft_plan, &renamed, "product"),
                "productAreaName": previous.name
            },
            "fields": { "name": renamed.name }
        }),
        "capability" => json!({
            "type": "update_capability",
            "target": {
                "productName": find_draft_ancestor_name(&draft_plan, &renamed, "product"),
                "productAreaName": find_draft_ancestor_name(&draft_plan, &renamed, "product_area"),
                "capabilityName": previous.name
            },
            "fields": { "name": renamed.name }
        }),
        "work_item" => json!({
            "type": "update_work_item",
            "target": { "workItemTitle": previous.name },
            "fields": { "title": renamed.name }
        }),
        _ => json!({ "type": "update_node" }),
    };
    let plan = PlannerPlan {
        assistant_response: format!(
            "Renamed draft {} to \"{}\".",
            renamed.node_type.replace('_', " "),
            renamed.name
        ),
        needs_confirmation: false,
        clarification_question: None,
        actions: vec![action],
    };
    Ok(PlannerTurnResponse {
        session_id,
        status: "proposal".to_string(),
        assistant_message: plan.assistant_response.clone(),
        pending_plan: Some(plan),
        tree_nodes: None,
        draft_tree_nodes: Some(build_draft_tree_nodes(
            &draft_plan,
            session.selected_draft_node_id.as_deref(),
        )),
        selected_draft_node_id: session.selected_draft_node_id,
        execution_lines: vec![format!("Renamed \"{}\".", renamed.name)],
        execution_errors: vec![],
        trace_events: trace,
    })
}

pub async fn add_planner_draft_child(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    parent_node_id: String,
    child_type: String,
    name: String,
    summary: Option<String>,
) -> Result<PlannerTurnResponse, AppError> {
    let mut trace = vec![];
    let mut session = get_or_load_session(&planner_service, db, &session_id).await?;
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );
    let mut draft_plan = session
        .draft_plan
        .clone()
        .ok_or_else(|| AppError::Validation("No staged design is available".to_string()))?;
    let parent = find_draft_node_by_id(&draft_plan, Some(&parent_node_id))
        .cloned()
        .ok_or_else(|| AppError::Validation("Parent design node was not found".to_string()))?;
    push_trace(
        &mut trace,
        "draft",
        "Adding draft child node",
        format!(
            "parent_id={}\nparent_type={}\nparent_name={}\nchild_type={}\nchild_name={}",
            parent_node_id, parent.node_type, parent.name, child_type, name
        ),
    );
    let created = add_draft_child_node(
        &mut draft_plan,
        &parent_node_id,
        &child_type,
        &name,
        summary.as_deref(),
    )?;
    session.draft_plan = Some(draft_plan.clone());
    session.selected_draft_node_id = Some(created.id.clone());
    session.pending_plan = None;
    persist_pending_plan(db, &session_id, None).await?;
    persist_draft_state(
        db,
        &session_id,
        Some(&draft_plan),
        session.selected_draft_node_id.as_deref(),
    )
    .await?;
    {
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
    }
    let plan = PlannerPlan {
        assistant_response: format!(
            "Added draft {} \"{}\" under \"{}\".",
            created.node_type.replace('_', " "),
            created.name,
            parent.name
        ),
        needs_confirmation: false,
        clarification_question: None,
        actions: vec![created.details.clone()],
    };
    Ok(PlannerTurnResponse {
        session_id,
        status: "proposal".to_string(),
        assistant_message: plan.assistant_response.clone(),
        pending_plan: Some(plan),
        tree_nodes: None,
        draft_tree_nodes: Some(build_draft_tree_nodes(
            &draft_plan,
            session.selected_draft_node_id.as_deref(),
        )),
        selected_draft_node_id: session.selected_draft_node_id,
        execution_lines: vec![format!(
            "Added {} \"{}\".",
            created.node_type.replace('_', " "),
            created.name
        )],
        execution_errors: vec![],
        trace_events: trace,
    })
}

pub async fn delete_planner_draft_node(
    planner_service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    node_id: String,
) -> Result<PlannerTurnResponse, AppError> {
    let mut trace = vec![];
    let mut session = get_or_load_session(&planner_service, db, &session_id).await?;
    push_trace(
        &mut trace,
        "session",
        "Loaded planner session",
        format!(
            "session_id={}\nhas_pending_plan={}\nhas_draft_plan={}\nselected_draft_node_id={:?}",
            session_id,
            session.pending_plan.is_some(),
            session.draft_plan.is_some(),
            session.selected_draft_node_id
        ),
    );
    let mut draft_plan = session
        .draft_plan
        .clone()
        .ok_or_else(|| AppError::Validation("No staged design is available".to_string()))?;
    let target = find_draft_node_by_id(&draft_plan, Some(&node_id))
        .cloned()
        .ok_or_else(|| AppError::Validation("Draft node was not found".to_string()))?;
    push_trace(
        &mut trace,
        "draft",
        "Deleting design node",
        format!(
            "node_id={}\ntype={}\nname={}",
            node_id, target.node_type, target.name
        ),
    );
    let (removed, fallback_parent_id) = delete_draft_node(&mut draft_plan, &node_id)?;
    session.draft_plan = if draft_plan.nodes.is_empty() {
        None
    } else {
        Some(draft_plan.clone())
    };
    session.selected_draft_node_id = fallback_parent_id;
    session.pending_plan = None;
    persist_pending_plan(db, &session_id, None).await?;
    persist_draft_state(
        db,
        &session_id,
        session.draft_plan.as_ref(),
        session.selected_draft_node_id.as_deref(),
    )
    .await?;
    {
        let mut service = planner_service.lock().await;
        service.save_session(&session_id, session.clone());
    }
    let action = match removed.node_type.as_str() {
        "product" => json!({
            "type": "archive_product",
            "target": { "productName": removed.name }
        }),
        "product_area" => json!({
            "type": "delete_product_area",
            "target": {
                "productName": find_draft_ancestor_name(&draft_plan, &removed, "product"),
                "productAreaName": removed.name
            }
        }),
        "capability" => json!({
            "type": "delete_capability",
            "target": {
                "productName": find_draft_ancestor_name(&draft_plan, &removed, "product"),
                "productAreaName": find_draft_ancestor_name(&draft_plan, &removed, "product_area"),
                "capabilityName": removed.name
            }
        }),
        "work_item" => json!({
            "type": "delete_work_item",
            "target": { "workItemTitle": removed.name }
        }),
        _ => json!({ "type": "delete_node" }),
    };
    let plan = PlannerPlan {
        assistant_response: format!(
            "Removed draft {} \"{}\".",
            removed.node_type.replace('_', " "),
            removed.name
        ),
        needs_confirmation: false,
        clarification_question: None,
        actions: vec![action],
    };
    Ok(PlannerTurnResponse {
        session_id,
        status: "proposal".to_string(),
        assistant_message: plan.assistant_response.clone(),
        pending_plan: Some(plan),
        tree_nodes: None,
        draft_tree_nodes: session
            .draft_plan
            .as_ref()
            .map(|draft| build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref())),
        selected_draft_node_id: session.selected_draft_node_id,
        execution_lines: vec![format!(
            "Removed {} \"{}\".",
            removed.node_type.replace('_', " "),
            removed.name
        )],
        execution_errors: vec![],
        trace_events: trace,
    })
}

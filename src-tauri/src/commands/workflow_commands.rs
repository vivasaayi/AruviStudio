use crate::domain::agent::AgentRun;
use crate::domain::model::ModelCall;
use crate::domain::workflow::{UserAction, WorkflowRun, WorkflowStageHistory};
use crate::error::AppError;
use crate::persistence::{agent_repo, model_call_repo, workflow_repo};
use crate::state::AppState;
use tauri::State;

fn resolve_work_item_id(work_item_id: Option<String>) -> Result<String, AppError> {
    work_item_id.ok_or_else(|| AppError::Validation("missing work item id".to_string()))
}

#[tauri::command]
pub async fn start_work_item_workflow(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
) -> Result<String, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id)?;
    let workflow_service = state.workflow_service.lock().await;
    let workflow_run = workflow_service
        .start_work_item_workflow(&work_item_id)
        .await?;
    Ok(workflow_run.id)
}

#[tauri::command]
pub async fn get_workflow_run(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<WorkflowRun, AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service.get_workflow_run(&workflow_run_id).await
}

#[tauri::command]
pub async fn get_latest_workflow_run_for_work_item(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
) -> Result<Option<WorkflowRun>, AppError> {
    let work_item_id = resolve_work_item_id(work_item_id)?;
    workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item_id).await
}

#[tauri::command]
pub async fn get_workflow_history(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<Vec<WorkflowStageHistory>, AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service
        .get_workflow_history(&workflow_run_id)
        .await
}

#[tauri::command]
pub async fn handle_workflow_user_action(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    action: String,
    notes: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let parsed_action = match action.as_str() {
        "approve" => UserAction::Approve,
        "reject" => UserAction::Reject,
        "pause" => UserAction::Pause,
        "resume" => UserAction::Resume,
        "cancel" => UserAction::Cancel,
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported workflow action: {}",
                action
            )))
        }
    };
    let workflow_service = state.workflow_service.lock().await;
    workflow_service
        .handle_user_action(&workflow_run_id, parsed_action, notes)
        .await
}

#[tauri::command]
pub async fn advance_workflow(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let workflow_service = state.workflow_service.lock().await;
    workflow_service.advance_workflow(&workflow_run_id).await
}

#[tauri::command]
pub async fn list_agent_runs_for_workflow(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<Vec<AgentRun>, AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    agent_repo::list_agent_runs_for_workflow(&state.db, &workflow_run_id).await
}

#[tauri::command]
pub async fn list_agent_model_calls_for_workflow(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<Vec<ModelCall>, AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    model_call_repo::list_model_calls_for_workflow(&state.db, &workflow_run_id).await
}

#[tauri::command]
pub async fn mark_workflow_run_failed(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
    reason: Option<String>,
) -> Result<(), AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
    if run.current_stage != "failed" {
        workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "failed").await?;
        let transition_id = uuid::Uuid::new_v4().to_string();
        workflow_repo::record_stage_transition(
            &state.db,
            &transition_id,
            &workflow_run_id,
            &run.current_stage,
            "failed",
            "user_override",
            reason
                .as_deref()
                .unwrap_or("Marked failed by operator from UI"),
        )
        .await?;
    }
    workflow_repo::update_workflow_lifecycle(
        &state.db,
        &workflow_run_id,
        "failed",
        Some(
            reason
                .as_deref()
                .unwrap_or("Marked failed by operator from UI"),
        ),
        true,
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn restart_workflow_run(
    state: State<'_, AppState>,
    workflow_run_id: Option<String>,
) -> Result<String, AppError> {
    let workflow_run_id = workflow_run_id
        .ok_or_else(|| AppError::Validation("missing workflow run id".to_string()))?;
    let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
    let workflow_service = state.workflow_service.lock().await;
    let next = workflow_service
        .start_work_item_workflow(&run.work_item_id)
        .await?;
    Ok(next.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{product_commands, test_helpers::make_test_app, work_item_commands};
    use crate::domain::work_item::WorkItem;
    use crate::persistence::workflow_repo;
    use tauri::test::MockRuntime;
    use tauri::Manager;

    async fn create_approved_work_item(
        state: State<'_, AppState>,
        product_name: &str,
        work_item_title: &str,
    ) -> WorkItem {
        let product = product_commands::create_product(
            state.clone(),
            product_commands::CreateProductCommand {
                name: product_name.to_string(),
                description: "".to_string(),
                vision: "".to_string(),
                goals: "[]".to_string(),
                tags: "[]".to_string(),
                lifecycle: None,
                health: None,
                owner_label: None,
                investment_status: None,
                roadmap: None,
                evidence: None,
            },
        )
        .await
        .expect("product should be created");
        let product_area = product_commands::create_product_area(
            state.clone(),
            product_commands::CreateProductAreaCommand {
                product_id: product.id.clone(),
                name: "Area".to_string(),
                description: "".to_string(),
                purpose: "".to_string(),
                node_kind: Some("product_area".to_string()),
                explanation: None,
                examples: None,
                implementation_notes: None,
                test_guidance: None,
            },
        )
        .await
        .expect("product_area should be created");
        let work_item = work_item_commands::create_work_item(
            state.clone(),
            work_item_commands::CreateWorkItemCommand {
                product_id: Some(product.id),
                product_area_id: Some(product_area.id),
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: work_item_title.to_string(),
                problem_statement: "".to_string(),
                description: "".to_string(),
                acceptance_criteria: "".to_string(),
                constraints: "".to_string(),
                work_item_type: "story".to_string(),
                priority: "medium".to_string(),
                complexity: "medium".to_string(),
            },
        )
        .await
        .expect("work item should be created");

        work_item_commands::update_work_item(
            state,
            work_item_commands::UpdateWorkItemCommand {
                id: work_item.id.clone(),
                title: None,
                description: None,
                status: Some("approved".to_string()),
                problem_statement: None,
                acceptance_criteria: None,
                constraints: None,
            },
        )
        .await
        .expect("work item should be approved")
    }
    #[tokio::test]
    async fn workflow_commands_expose_seeded_runs_and_validate_required_ids() {
        let app: tauri::App<MockRuntime> = make_test_app("workflow_commands_seeded").await;
        let state = app.state::<AppState>();

        let work_item =
            create_approved_work_item(state.clone(), "Workflow Seed Product", "Workflow Seed Item")
                .await;
        let seeded =
            workflow_repo::create_workflow_run(&state.db, "workflow-seeded", &work_item.id)
                .await
                .expect("workflow should be seeded");
        workflow_repo::update_workflow_stage(&state.db, &seeded.id, "pending_plan_approval")
            .await
            .expect("workflow stage should update");

        let workflow_run = get_workflow_run(state.clone(), Some(seeded.id.clone()))
            .await
            .expect("workflow run should load");
        let latest =
            get_latest_workflow_run_for_work_item(state.clone(), Some(work_item.id.clone()))
                .await
                .expect("latest workflow should load")
                .expect("latest workflow should exist");
        let missing = start_work_item_workflow(state, None)
            .await
            .expect_err("missing work item id should fail");

        assert_eq!(workflow_run.id, seeded.id);
        assert_eq!(workflow_run.current_stage, "pending_plan_approval");
        assert_eq!(latest.id, seeded.id);
        assert!(
            matches!(missing, AppError::Validation(message) if message == "missing work item id")
        );
    }

    #[tokio::test]
    async fn handle_invalid_action_and_mark_failed_from_command_layer() {
        let app: tauri::App<MockRuntime> = make_test_app("workflow_commands_fail").await;
        let state = app.state::<AppState>();
        let work_item =
            create_approved_work_item(state.clone(), "Workflow Fail Product", "Workflow Fail Item")
                .await;
        let workflow_run_id =
            workflow_repo::create_workflow_run(&state.db, "workflow-fail-seeded", &work_item.id)
                .await
                .expect("workflow should be seeded")
                .id;
        workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "pending_plan_approval")
            .await
            .expect("workflow stage should update");

        let invalid = handle_workflow_user_action(
            state.clone(),
            Some(workflow_run_id.clone()),
            "unsupported".to_string(),
            None,
        )
        .await
        .expect_err("invalid action should fail");
        assert!(
            matches!(invalid, AppError::Validation(message) if message.contains("Unsupported workflow action"))
        );

        mark_workflow_run_failed(
            state.clone(),
            Some(workflow_run_id.clone()),
            Some("operator fail".to_string()),
        )
        .await
        .expect("workflow should be marked failed");

        let updated = get_workflow_run(state, Some(workflow_run_id.clone()))
            .await
            .expect("workflow run should load");
        assert_eq!(updated.current_stage, "failed");
        assert_eq!(updated.status, "failed");
        assert_eq!(updated.error_message.as_deref(), Some("operator fail"));
    }
}

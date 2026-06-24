use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::persistence::workflow_repo;
use crate::services::workflow_git_push;
use crate::workflows::engine::WorkflowEngine;
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::{debug, info};

pub(crate) async fn advance_workflow(
    db: &SqlitePool,
    engine: &WorkflowEngine,
    workflow_run_id: &str,
) -> Result<(), AppError> {
    debug!(workflow_run_id = %workflow_run_id, "Advancing workflow");

    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
        .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;
    debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Retrieved current workflow stage");

    if current_stage.is_terminal() {
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Workflow is in terminal state, cannot advance");
        return Ok(());
    }

    let Some(next_stage) = engine.next_stage(&current_stage) else {
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "No next stage available, workflow is complete");
        return Ok(());
    };
    debug!(workflow_run_id = %workflow_run_id, next_stage = %next_stage.as_str(), "Determined next workflow stage");

    if next_stage == WorkflowStage::GitPush {
        workflow_git_push::push_workflow_changes(db, &workflow_run).await?;
    }

    workflow_repo::update_workflow_stage(db, workflow_run_id, next_stage.as_str()).await?;
    workflow_repo::record_stage_transition(
        db,
        &uuid::Uuid::new_v4().to_string(),
        workflow_run_id,
        current_stage.as_str(),
        next_stage.as_str(),
        "automatic",
        "Workflow advancement",
    )
    .await?;
    info!(workflow_run_id = %workflow_run_id, from_stage = %current_stage.as_str(), to_stage = %next_stage.as_str(), "Successfully advanced workflow stage");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{db as db_service, product_repo, work_item_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_workflow_advance_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ))
    }

    async fn create_test_pool(name: &str) -> SqlitePool {
        let temp_root = make_temp_dir(name);
        std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
        let db_path = temp_root.join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());
        db_service::create_pool(&database_url)
            .await
            .expect("test database should be created")
    }

    async fn create_test_work_item(pool: &SqlitePool, product_id: &str, work_item_id: &str) {
        product_repo::create_product(
            pool,
            product_repo::CreateProductInput {
                id: product_id,
                name: "Workflow Advance Product",
                description: "",
                vision: "",
                goals: "[]",
                tags: "[]",
                lifecycle: Some("active"),
                health: Some("healthy"),
                owner_label: None,
                investment_status: Some("invest"),
                roadmap: None,
                evidence: None,
            },
        )
        .await
        .expect("product should be created");

        work_item_repo::create_work_item(
            pool,
            work_item_repo::CreateWorkItemInput {
                id: work_item_id,
                product_id,
                product_area_id: None,
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: "Workflow advance work item",
                problem_statement: "",
                description: "",
                acceptance_criteria: "",
                constraints: "",
                work_item_type: "story",
                priority: "medium",
                complexity: "medium",
            },
        )
        .await
        .expect("work item should be created");
    }

    #[tokio::test]
    async fn advance_workflow_moves_non_terminal_stage_and_records_history() {
        let pool = create_test_pool("moves_stage").await;
        create_test_work_item(&pool, "product-advance", "work-item-advance").await;
        workflow_repo::create_workflow_run(&pool, "workflow-advance", "work-item-advance")
            .await
            .expect("workflow should be created");
        workflow_repo::update_workflow_stage(&pool, "workflow-advance", "planning")
            .await
            .expect("stage should update");

        advance_workflow(&pool, &WorkflowEngine::new(), "workflow-advance")
            .await
            .expect("workflow should advance");

        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-advance")
            .await
            .expect("workflow should load");
        assert_eq!(workflow.current_stage, "pending_plan_approval");

        let history = workflow_repo::get_workflow_history(&pool, "workflow-advance")
            .await
            .expect("history should load");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_stage, "planning");
        assert_eq!(history[0].to_stage, "pending_plan_approval");
        assert_eq!(history[0].trigger, "automatic");
        assert_eq!(history[0].notes, "Workflow advancement");
    }

    #[tokio::test]
    async fn advance_workflow_ignores_terminal_stage() {
        let pool = create_test_pool("terminal_stage").await;
        create_test_work_item(&pool, "product-terminal", "work-item-terminal").await;
        workflow_repo::create_workflow_run(&pool, "workflow-terminal", "work-item-terminal")
            .await
            .expect("workflow should be created");
        workflow_repo::update_workflow_stage(&pool, "workflow-terminal", "done")
            .await
            .expect("stage should update");

        advance_workflow(&pool, &WorkflowEngine::new(), "workflow-terminal")
            .await
            .expect("terminal workflow should be ignored");

        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-terminal")
            .await
            .expect("workflow should load");
        assert_eq!(workflow.current_stage, "done");

        let history = workflow_repo::get_workflow_history(&pool, "workflow-terminal")
            .await
            .expect("history should load");
        assert!(history.is_empty());
    }
}

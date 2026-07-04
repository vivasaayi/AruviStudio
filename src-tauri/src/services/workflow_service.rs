use crate::domain::workflow::{UserAction, WorkflowRun, WorkflowStage, WorkflowStageHistory};
use crate::error::AppError;
use crate::persistence::workflow_repo;
use crate::services::agent_service;
use crate::services::{workflow_advance, workflow_user_action};
use crate::workflows::engine::WorkflowEngine;
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

mod agent_stages;
mod approval_stages;
mod stages;
mod start;
mod system_stages;

pub struct WorkflowService {
    db: Arc<SqlitePool>,
    engine: WorkflowEngine,
    agent_service: Arc<Mutex<agent_service::AgentService>>,
}

impl WorkflowService {
    pub fn new(
        db: Arc<SqlitePool>,
        agent_service: Arc<Mutex<agent_service::AgentService>>,
    ) -> Self {
        Self {
            db,
            engine: WorkflowEngine::new(),
            agent_service,
        }
    }

    /// Handle user approval actions
    pub async fn handle_user_action(
        &self,
        workflow_run_id: &str,
        action: UserAction,
        notes: Option<String>,
    ) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, action = %action.as_str(), notes = ?notes, "Handling user action");

        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
            .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Retrieved current workflow stage");

        let transition =
            workflow_user_action::resolve_user_action_transition(&current_stage, &action, notes)?;
        self.transition_stage(
            workflow_run_id,
            current_stage,
            transition.to_stage,
            transition.trigger,
            transition.notes,
        )
        .await?;
        if let Some(stage) = transition.execute_stage {
            self.execute_stage(workflow_run_id, stage).await?;
        }

        info!(workflow_run_id = %workflow_run_id, action = %action.as_str(), "Successfully handled user action");
        Ok(())
    }

    /// Advance workflow to the next stage
    pub async fn advance_workflow(&self, workflow_run_id: &str) -> Result<(), AppError> {
        workflow_advance::advance_workflow(self.db.as_ref(), &self.engine, workflow_run_id).await
    }

    /// Get workflow run with current status
    pub async fn get_workflow_run(&self, workflow_run_id: &str) -> Result<WorkflowRun, AppError> {
        debug!(workflow_run_id = %workflow_run_id, "Retrieving workflow run");
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        info!(workflow_run_id = %workflow_run_id, current_stage = %workflow_run.current_stage, "Successfully retrieved workflow run");
        Ok(workflow_run)
    }

    /// Get workflow stage history
    pub async fn get_workflow_history(
        &self,
        workflow_run_id: &str,
    ) -> Result<Vec<WorkflowStageHistory>, AppError> {
        debug!(workflow_run_id = %workflow_run_id, "Retrieving workflow history");
        let history = workflow_repo::get_workflow_history(&self.db, workflow_run_id).await?;
        info!(workflow_run_id = %workflow_run_id, history_entries = history.len(), "Successfully retrieved workflow history");
        Ok(history)
    }
}

#[cfg(test)]
mod tests;

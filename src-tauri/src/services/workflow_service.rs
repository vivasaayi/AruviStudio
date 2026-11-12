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

    /// Retry a failed Git delivery after the user has corrected its cause.
    pub async fn retry_git_delivery(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if !crate::services::workflow_git_push::is_retryable_git_delivery(&self.db, workflow_run_id).await? {
            return Err(AppError::Validation("This workflow does not have a retryable failed Git delivery".to_string()));
        }
        let mut tx = self.db.begin().await?;
        let changed = sqlx::query("UPDATE workflow_runs SET status='running',current_stage='git_push',error_message=NULL,ended_at=NULL,retry_count=retry_count+1,updated_at=datetime('now') WHERE id=? AND status='failed' AND current_stage='failed' AND rowid=(SELECT MAX(rowid) FROM workflow_runs WHERE work_item_id=(SELECT work_item_id FROM workflow_runs WHERE id=?)) AND (SELECT from_stage FROM workflow_stage_history WHERE workflow_run_id=? AND to_stage='failed' ORDER BY rowid DESC LIMIT 1)='git_push'")
            .bind(workflow_run_id).bind(workflow_run_id).bind(workflow_run_id).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 { return Err(AppError::Validation("Workflow changed; refresh before retrying delivery".to_string())); }
        sqlx::query("INSERT INTO workflow_stage_history (id,workflow_run_id,from_stage,to_stage,trigger,notes,transitioned_at) VALUES (?,?,'failed','git_push','manual_override','Git delivery retry requested',datetime('now'))")
            .bind(uuid::Uuid::new_v4().to_string()).bind(workflow_run_id).execute(&mut *tx).await?;
        tx.commit().await?;
        if let Err(error) = self.execute_stage(workflow_run_id, WorkflowStage::GitPush).await {
            let current = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
            if current.status == "running" {
                crate::services::workflow_failure::mark_workflow_failed(&self.db, workflow_run_id, &WorkflowStage::GitPush, &error).await?;
            }
            return Err(error);
        }
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

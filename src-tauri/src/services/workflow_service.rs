use crate::domain::agent::AgentRunStatus;
use crate::domain::work_item::WorkItemStatus;
use crate::domain::workflow::{
    TransitionTrigger, UserAction, WorkflowRun, WorkflowStage, WorkflowStageHistory,
};
use crate::error::AppError;
use crate::persistence::{agent_repo, work_item_repo, workflow_repo};
use crate::services::agent_service;
use crate::services::{
    workflow_agent_stage, workflow_approval_gate, workflow_coordinator_policy, workflow_git_push,
    workflow_repository_guard, workflow_stage_transition, workflow_user_action,
};
use crate::workflows::engine::WorkflowEngine;
use sqlx::SqlitePool;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid;

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

    async fn ensure_repository_ready_for_stage(
        &self,
        workflow_run_id: &str,
        stage: &WorkflowStage,
    ) -> Result<(), AppError> {
        workflow_repository_guard::ensure_repository_ready_for_stage(
            self.db.as_ref(),
            workflow_run_id,
            stage,
        )
        .await
    }

    /// Start a workflow for a work item
    pub async fn start_work_item_workflow(
        &self,
        work_item_id: &str,
    ) -> Result<WorkflowRun, AppError> {
        info!(work_item_id = %work_item_id, "Starting workflow for work item");

        // Recovery: close coordinator-review runs that are orphaned due to invalid/missing coordinators.
        let recovered = workflow_repo::close_orphaned_coordinator_reviews(&self.db).await?;
        if recovered > 0 {
            warn!(
                recovered_runs = recovered,
                "Closed orphaned coordinator review workflow runs before start"
            );
        }

        // Validate work item exists and is in correct state
        let work_item = work_item_repo::get_work_item(&self.db, work_item_id).await?;
        if work_item.status != WorkItemStatus::Approved {
            return Err(AppError::Validation(
                "Work item must be approved before starting workflow".to_string(),
            ));
        }

        if let Some(active) =
            workflow_repo::find_active_workflow_for_work_item(&self.db, work_item_id).await?
        {
            warn!(
                workflow_run_id = %active.id,
                stage = %active.current_stage,
                "Active workflow already exists for work item; attempting to resume"
            );

            let stage_failed = agent_repo::list_agent_runs_for_workflow(&self.db, &active.id)
                .await?
                .iter()
                .rev()
                .any(|run| {
                    run.stage == active.current_stage && run.status == AgentRunStatus::Failed
                });
            if stage_failed {
                warn!(
                    workflow_run_id = %active.id,
                    stage = %active.current_stage,
                    "Detected failed agent run for active stage; closing stale workflow before restart"
                );
                let stage_for_failure =
                    WorkflowStage::from_str(&active.current_stage).unwrap_or(WorkflowStage::Draft);
                self.mark_workflow_failed(
                    &active.id,
                    &stage_for_failure,
                    &AppError::Validation(format!(
                        "Detected failed agent run while workflow remained active in {}",
                        active.current_stage
                    )),
                )
                .await?;
            } else {
                let current_stage =
                    WorkflowStage::from_str(&active.current_stage).unwrap_or(WorkflowStage::Draft);

                match current_stage {
                    WorkflowStage::Draft => {
                        self.transition_stage(
                            &active.id,
                            WorkflowStage::Draft,
                            WorkflowStage::RequirementAnalysis,
                            TransitionTrigger::Automatic,
                            "Resumed existing workflow from draft".to_string(),
                        )
                        .await?;
                        self.execute_stage(&active.id, WorkflowStage::RequirementAnalysis)
                            .await?;
                    }
                    WorkflowStage::CoordinatorReview => {
                        if active.pending_stage_name.is_none() {
                            workflow_repo::update_workflow_lifecycle(
                                &self.db,
                                &active.id,
                                "failed",
                                Some(
                                    "Auto-closed invalid coordinator_review run without pending stage",
                                ),
                                true,
                            )
                            .await?;
                        } else {
                            self.execute_stage(&active.id, WorkflowStage::CoordinatorReview)
                                .await?;
                        }
                    }
                    WorkflowStage::PendingTaskApproval
                    | WorkflowStage::PendingPlanApproval
                    | WorkflowStage::PendingTestReview
                    | WorkflowStage::Done
                    | WorkflowStage::Failed
                    | WorkflowStage::Cancelled
                    | WorkflowStage::Blocked => {
                        // Gate/terminal states are intentionally not auto-executed.
                    }
                    other => {
                        self.execute_stage(&active.id, other).await?;
                    }
                }

                return workflow_repo::get_workflow_run(&self.db, &active.id).await;
            }
        }

        // Create workflow run
        let workflow_run_id = uuid::Uuid::new_v4().to_string();
        let workflow_run =
            workflow_repo::create_workflow_run(&self.db, &workflow_run_id, work_item_id).await?;
        info!(workflow_run_id = %workflow_run.id, work_item_id = %work_item_id, "Created workflow run for work item");

        let assigned_team = agent_repo::resolve_team_for_work_item(&self.db, &work_item).await?;
        let coordinator = match &assigned_team {
            Some(team) => agent_repo::find_team_coordinator(&self.db, &team.id).await?,
            None => None,
        };
        if let Some(team) = &assigned_team {
            let active_count =
                agent_repo::count_active_workflows_for_team(&self.db, &team.id).await?;
            if active_count >= i64::from(team.max_concurrent_workflows) {
                return Err(AppError::Validation(format!(
                    "Team {} is at capacity ({}/{})",
                    team.name, active_count, team.max_concurrent_workflows
                )));
            }
        }
        workflow_repo::set_workflow_assignment(
            &self.db,
            &workflow_run.id,
            assigned_team.as_ref().map(|team| team.id.as_str()),
            coordinator.as_ref().map(|agent| agent.id.as_str()),
        )
        .await?;
        info!(
            workflow_run_id = %workflow_run.id,
            assigned_team = ?assigned_team.as_ref().map(|team| team.name.as_str()),
            coordinator = ?coordinator.as_ref().map(|agent| agent.name.as_str()),
            "Resolved team ownership for workflow"
        );

        // Work item approval already happened at the work-item level. Move directly into execution.
        self.transition_stage(
            &workflow_run.id,
            WorkflowStage::Draft,
            WorkflowStage::RequirementAnalysis,
            TransitionTrigger::Automatic,
            "Workflow started from approved work item".to_string(),
        )
        .await?;
        self.execute_stage(&workflow_run.id, WorkflowStage::RequirementAnalysis)
            .await?;

        Ok(workflow_run)
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

    /// Execute a workflow stage
    fn execute_stage<'a>(
        &'a self,
        workflow_run_id: &'a str,
        stage: WorkflowStage,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        self.execute_stage_internal(workflow_run_id, stage, false)
    }

    fn execute_stage_internal<'a>(
        &'a self,
        workflow_run_id: &'a str,
        stage: WorkflowStage,
        bypass_coordinator_review: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            info!(
                "Executing stage {} for workflow run {}",
                stage.as_str(),
                workflow_run_id
            );

            self.ensure_repository_ready_for_stage(workflow_run_id, &stage)
                .await?;

            if !bypass_coordinator_review
                && workflow_coordinator_policy::requires_coordinator_review(
                    self.db.as_ref(),
                    &stage,
                )
                .await?
            {
                if workflow_coordinator_policy::has_active_coordinator_for_workflow(
                    self.db.as_ref(),
                    workflow_run_id,
                )
                .await?
                {
                    self.route_through_coordinator_review(workflow_run_id, stage.clone())
                        .await?;
                    return Ok(());
                }
                warn!(
                    workflow_run_id = %workflow_run_id,
                    stage = %stage.as_str(),
                    "Coordinator review required by policy but no active coordinator is available; bypassing coordinator stage"
                );
            }

            let execution_result = match stage.clone() {
                WorkflowStage::CoordinatorReview => {
                    self.execute_coordinator_review(workflow_run_id).await
                }
                WorkflowStage::RequirementAnalysis => {
                    self.execute_requirement_analysis(workflow_run_id).await
                }
                WorkflowStage::Planning => self.execute_planning(workflow_run_id).await,
                WorkflowStage::Coding => self.execute_coding(workflow_run_id).await,
                WorkflowStage::UnitTestGeneration => {
                    self.execute_unit_test_generation(workflow_run_id).await
                }
                WorkflowStage::IntegrationTestGeneration => {
                    self.execute_integration_test_generation(workflow_run_id)
                        .await
                }
                WorkflowStage::UiTestPlanning => {
                    self.execute_ui_test_planning(workflow_run_id).await
                }
                WorkflowStage::DockerTestExecution => {
                    self.execute_docker_test_execution(workflow_run_id).await
                }
                WorkflowStage::QaValidation => self.execute_qa_validation(workflow_run_id).await,
                WorkflowStage::SecurityReview => {
                    self.execute_security_review(workflow_run_id).await
                }
                WorkflowStage::PerformanceReview => {
                    self.execute_performance_review(workflow_run_id).await
                }
                WorkflowStage::PushPreparation => {
                    self.execute_push_preparation(workflow_run_id).await
                }
                WorkflowStage::GitPush => self.execute_git_push(workflow_run_id).await,
                _ => {
                    warn!("Stage {} does not require execution", stage.as_str());
                    Ok(())
                }
            };

            if let Err(stage_error) = execution_result {
                error!(
                    workflow_run_id = %workflow_run_id,
                    stage = %stage.as_str(),
                    error = %stage_error,
                    "Stage execution failed"
                );
                if let Err(mark_error) = self
                    .mark_workflow_failed(workflow_run_id, &stage, &stage_error)
                    .await
                {
                    error!(
                        workflow_run_id = %workflow_run_id,
                        stage = %stage.as_str(),
                        error = %mark_error,
                        "Failed to persist workflow failure state"
                    );
                }
                return Err(stage_error);
            }

            Ok(())
        })
    }

    async fn mark_workflow_failed(
        &self,
        workflow_run_id: &str,
        stage: &WorkflowStage,
        cause: &AppError,
    ) -> Result<(), AppError> {
        let reason = format!("Stage {} failed: {}", stage.as_str(), cause);
        let current = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;

        if current.current_stage != WorkflowStage::Failed.as_str() {
            workflow_repo::update_workflow_stage(
                &self.db,
                workflow_run_id,
                WorkflowStage::Failed.as_str(),
            )
            .await?;

            let transition_id = uuid::Uuid::new_v4().to_string();
            workflow_repo::record_stage_transition(
                &self.db,
                &transition_id,
                workflow_run_id,
                &current.current_stage,
                WorkflowStage::Failed.as_str(),
                TransitionTrigger::Automatic.as_str(),
                &reason,
            )
            .await?;
        }

        workflow_repo::update_workflow_lifecycle(
            &self.db,
            workflow_run_id,
            "failed",
            Some(&reason),
            true,
        )
        .await?;
        Ok(())
    }

    async fn route_through_coordinator_review(
        &self,
        workflow_run_id: &str,
        target_stage: WorkflowStage,
    ) -> Result<(), AppError> {
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
            .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;

        workflow_repo::set_pending_stage_name(
            &self.db,
            workflow_run_id,
            Some(target_stage.as_str()),
        )
        .await?;
        self.transition_stage(
            workflow_run_id,
            current_stage,
            WorkflowStage::CoordinatorReview,
            TransitionTrigger::Automatic,
            format!("Coordinator review before {}", target_stage.as_str()),
        )
        .await?;
        self.execute_stage_internal(workflow_run_id, WorkflowStage::CoordinatorReview, true)
            .await
    }

    async fn execute_coordinator_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing coordinator review stage");
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let pending_stage_name = workflow_run.pending_stage_name.clone().ok_or_else(|| {
            AppError::Validation("Coordinator review requires a pending target stage".to_string())
        })?;
        let target_stage = WorkflowStage::from_str(&pending_stage_name)
            .map_err(|e| AppError::Validation(format!("Invalid pending workflow stage: {}", e)))?;

        if !workflow_coordinator_policy::has_active_coordinator_for_workflow(
            self.db.as_ref(),
            workflow_run_id,
        )
        .await?
        {
            warn!(
                workflow_run_id = %workflow_run_id,
                target_stage = %target_stage.as_str(),
                "No active coordinator found at coordinator review time; bypassing to pending stage"
            );
            workflow_repo::set_pending_stage_name(&self.db, workflow_run_id, None).await?;
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::CoordinatorReview,
                target_stage.clone(),
                TransitionTrigger::Automatic,
                format!(
                    "Coordinator unavailable; bypassed to {}",
                    target_stage.as_str()
                ),
            )
            .await?;
            self.execute_stage_internal(workflow_run_id, target_stage, true)
                .await?;
            return Ok(());
        }

        let agent_service = self.agent_service.lock().await;
        let agent_run = agent_service
            .run_agent_for_stage(workflow_run_id, "coordinator_review")
            .await?;
        drop(agent_service);

        if agent_run.status == AgentRunStatus::Completed {
            workflow_repo::set_pending_stage_name(&self.db, workflow_run_id, None).await?;
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::CoordinatorReview,
                target_stage.clone(),
                TransitionTrigger::AgentCompletion,
                format!("Coordinator review completed for {}", target_stage.as_str()),
            )
            .await?;
            self.execute_stage_internal(workflow_run_id, target_stage, true)
                .await?;
        }

        Ok(())
    }

    async fn execute_requirement_analysis(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing requirement analysis stage");

        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "requirement_analysis",
            WorkflowStage::RequirementAnalysis,
            WorkflowStage::Planning,
            "Requirement analysis completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::Planning)
                .await?;
            info!(workflow_run_id = %workflow_run_id, "Successfully transitioned to planning stage");
        }

        Ok(())
    }

    /// Execute planning stage
    async fn execute_planning(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing planning stage");

        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "planning",
            WorkflowStage::Planning,
            WorkflowStage::PendingPlanApproval,
            "Planning completed",
        )
        .await?
        {
            if workflow_approval_gate::is_auto_plan_approval_enabled(self.db.as_ref()).await? {
                let workflow_run =
                    workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
                workflow_approval_gate::record_plan_approval(
                    self.db.as_ref(),
                    workflow_run_id,
                    &workflow_run.work_item_id,
                    "approved",
                    "Plan auto-approved by workflow setting",
                )
                .await?;
                self.transition_stage(
                    workflow_run_id,
                    WorkflowStage::PendingPlanApproval,
                    WorkflowStage::Coding,
                    TransitionTrigger::Automatic,
                    "Plan auto-approved by workflow setting".to_string(),
                )
                .await?;
                self.execute_stage(workflow_run_id, WorkflowStage::Coding)
                    .await?;
                info!(
                    workflow_run_id = %workflow_run_id,
                    "Planning completed and auto-approved directly into coding"
                );
            } else {
                info!(workflow_run_id = %workflow_run_id, "Successfully transitioned to plan approval stage");
            }
        }

        Ok(())
    }

    /// Execute coding stage
    async fn execute_coding(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing coding stage");

        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "coding",
            WorkflowStage::Coding,
            WorkflowStage::UnitTestGeneration,
            "Coding completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::UnitTestGeneration)
                .await?;
            info!(workflow_run_id = %workflow_run_id, "Successfully transitioned to unit test generation stage");
        }

        Ok(())
    }

    /// Execute unit test generation stage
    async fn execute_unit_test_generation(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "unit_test_generation",
            WorkflowStage::UnitTestGeneration,
            WorkflowStage::IntegrationTestGeneration,
            "Unit test generation completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::IntegrationTestGeneration)
                .await?;
        }

        Ok(())
    }

    /// Execute integration test generation stage
    async fn execute_integration_test_generation(
        &self,
        workflow_run_id: &str,
    ) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "integration_test_generation",
            WorkflowStage::IntegrationTestGeneration,
            WorkflowStage::UiTestPlanning,
            "Integration test generation completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::UiTestPlanning)
                .await?;
        }

        Ok(())
    }

    /// Execute UI test planning stage
    async fn execute_ui_test_planning(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "ui_test_planning",
            WorkflowStage::UiTestPlanning,
            WorkflowStage::DockerTestExecution,
            "UI test planning completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::DockerTestExecution)
                .await?;
        }

        Ok(())
    }

    /// Execute Docker test execution stage
    async fn execute_docker_test_execution(&self, workflow_run_id: &str) -> Result<(), AppError> {
        // TODO: Implement Docker test execution
        // For now, simulate success
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        self.transition_stage(
            workflow_run_id,
            WorkflowStage::DockerTestExecution,
            WorkflowStage::QaValidation,
            TransitionTrigger::Automatic,
            "Test execution completed".to_string(),
        )
        .await?;
        self.execute_stage(workflow_run_id, WorkflowStage::QaValidation)
            .await?;

        Ok(())
    }

    /// Execute QA validation stage
    async fn execute_qa_validation(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "qa_validation",
            WorkflowStage::QaValidation,
            WorkflowStage::SecurityReview,
            "QA validation completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::SecurityReview)
                .await?;
        }

        Ok(())
    }

    /// Execute security review stage
    async fn execute_security_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "security_review",
            WorkflowStage::SecurityReview,
            WorkflowStage::PerformanceReview,
            "Security review completed",
        )
        .await?
        {
            self.execute_stage(workflow_run_id, WorkflowStage::PerformanceReview)
                .await?;
        }

        Ok(())
    }

    /// Execute performance review stage
    async fn execute_performance_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if workflow_agent_stage::run_agent_stage_and_transition(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
            "performance_review",
            WorkflowStage::PerformanceReview,
            WorkflowStage::PendingTestReview,
            "Performance review completed",
        )
        .await?
            && workflow_approval_gate::is_auto_test_review_enabled(self.db.as_ref()).await?
        {
            let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
            workflow_approval_gate::record_test_review_approval(
                self.db.as_ref(),
                workflow_run_id,
                &workflow_run.work_item_id,
                "approved",
                "Test review auto-approved by workflow setting",
            )
            .await?;
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::PendingTestReview,
                WorkflowStage::PushPreparation,
                TransitionTrigger::Automatic,
                "Test review auto-approved by workflow setting".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::PushPreparation)
                .await?;
        }

        Ok(())
    }

    /// Execute push preparation stage
    async fn execute_push_preparation(&self, workflow_run_id: &str) -> Result<(), AppError> {
        // TODO: Implement push preparation (validation, etc.)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        self.transition_stage(
            workflow_run_id,
            WorkflowStage::PushPreparation,
            WorkflowStage::GitPush,
            TransitionTrigger::Automatic,
            "Push preparation completed".to_string(),
        )
        .await?;
        self.execute_stage(workflow_run_id, WorkflowStage::GitPush)
            .await?;

        Ok(())
    }

    /// Execute Git push stage
    async fn execute_git_push(&self, workflow_run_id: &str) -> Result<(), AppError> {
        workflow_git_push::execute_git_push_stage(self.db.as_ref(), workflow_run_id).await?;
        self.transition_stage(
            workflow_run_id,
            WorkflowStage::GitPush,
            WorkflowStage::Done,
            TransitionTrigger::Automatic,
            "Git push completed".to_string(),
        )
        .await?;

        Ok(())
    }

    /// Transition workflow to a new stage
    async fn transition_stage(
        &self,
        workflow_run_id: &str,
        from: WorkflowStage,
        to: WorkflowStage,
        trigger: TransitionTrigger,
        notes: String,
    ) -> Result<(), AppError> {
        workflow_stage_transition::transition_stage(
            self.db.as_ref(),
            workflow_run_id,
            from,
            to,
            trigger,
            notes,
        )
        .await
    }

    /// Advance workflow to the next stage
    pub async fn advance_workflow(&self, workflow_run_id: &str) -> Result<(), AppError> {
        debug!(workflow_run_id = %workflow_run_id, "Advancing workflow");

        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
            .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Retrieved current workflow stage");

        // Check if we can advance
        if !current_stage.is_terminal() {
            if let Some(next_stage) = self.engine.next_stage(&current_stage) {
                debug!(workflow_run_id = %workflow_run_id, next_stage = %next_stage.as_str(), "Determined next workflow stage");

                // Handle special stages that need additional processing
                if next_stage == WorkflowStage::GitPush {
                    self.handle_git_push(&workflow_run).await?;
                }

                // Update the workflow stage
                workflow_repo::update_workflow_stage(
                    &self.db,
                    workflow_run_id,
                    next_stage.as_str(),
                )
                .await?;
                workflow_repo::record_stage_transition(
                    &self.db,
                    &uuid::Uuid::new_v4().to_string(),
                    workflow_run_id,
                    current_stage.as_str(),
                    next_stage.as_str(),
                    "automatic",
                    "Workflow advancement",
                )
                .await?;
                info!(workflow_run_id = %workflow_run_id, from_stage = %current_stage.as_str(), to_stage = %next_stage.as_str(), "Successfully advanced workflow stage");
            } else {
                debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "No next stage available, workflow is complete");
            }
        } else {
            debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Workflow is in terminal state, cannot advance");
        }

        Ok(())
    }

    /// Handle Git push operations
    async fn handle_git_push(&self, workflow_run: &WorkflowRun) -> Result<(), AppError> {
        workflow_git_push::push_workflow_changes(self.db.as_ref(), workflow_run).await
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

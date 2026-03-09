use crate::domain::agent::AgentRunStatus;
use crate::domain::work_item::WorkItemStatus;
use crate::domain::workflow::{
    TransitionTrigger, UserAction, WorkflowRun, WorkflowStage, WorkflowStageHistory,
};
use crate::error::AppError;
use crate::persistence::{agent_repo, approval_repo, settings_repo, work_item_repo, workflow_repo};
use crate::services::agent_service;
use crate::workflows::engine::WorkflowEngine;
use crate::workflows::transitions;
use sqlx::SqlitePool;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid;

const AUTO_APPROVE_PLAN_KEY: &str = "workflow.auto_approve_plan";
const AUTO_APPROVE_TEST_REVIEW_KEY: &str = "workflow.auto_approve_test_review";

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

    fn is_coordinator_role(role: &str) -> bool {
        matches!(
            role.to_ascii_lowercase().as_str(),
            "manager" | "team_lead" | "coordinator"
        )
    }

    async fn is_auto_plan_approval_enabled(&self) -> Result<bool, AppError> {
        settings_repo::get_bool_setting(&self.db, AUTO_APPROVE_PLAN_KEY, true).await
    }

    async fn is_auto_test_review_enabled(&self) -> Result<bool, AppError> {
        settings_repo::get_bool_setting(&self.db, AUTO_APPROVE_TEST_REVIEW_KEY, true).await
    }

    async fn record_plan_approval(
        &self,
        workflow_run_id: &str,
        work_item_id: &str,
        status: &str,
        notes: &str,
    ) -> Result<(), AppError> {
        let approval_id = uuid::Uuid::new_v4().to_string();
        approval_repo::create_approval(
            &self.db,
            &approval_id,
            work_item_id,
            Some(workflow_run_id),
            "plan_approval",
            status,
            notes,
        )
        .await?;
        Ok(())
    }

    async fn record_test_review_approval(
        &self,
        workflow_run_id: &str,
        work_item_id: &str,
        status: &str,
        notes: &str,
    ) -> Result<(), AppError> {
        let approval_id = uuid::Uuid::new_v4().to_string();
        approval_repo::create_approval(
            &self.db,
            &approval_id,
            work_item_id,
            Some(workflow_run_id),
            "test_review",
            status,
            notes,
        )
        .await?;
        Ok(())
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

        match (&current_stage, &action) {
            (WorkflowStage::PendingTaskApproval, UserAction::Approve) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::RequirementAnalysis,
                    TransitionTrigger::UserApproval,
                    notes.unwrap_or("Work item approved".to_string()),
                )
                .await?;
                // Start requirement analysis
                self.execute_stage(workflow_run_id, WorkflowStage::RequirementAnalysis)
                    .await?;
            }
            (WorkflowStage::PendingTaskApproval, UserAction::Reject) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::Cancelled,
                    TransitionTrigger::UserRejection,
                    notes.unwrap_or("Work item rejected".to_string()),
                )
                .await?;
            }
            (WorkflowStage::PendingPlanApproval, UserAction::Approve) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::Coding,
                    TransitionTrigger::UserApproval,
                    notes.unwrap_or("Plan approved".to_string()),
                )
                .await?;
                // Start coding
                self.execute_stage(workflow_run_id, WorkflowStage::Coding)
                    .await?;
            }
            (WorkflowStage::PendingPlanApproval, UserAction::Reject) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::RequirementAnalysis,
                    TransitionTrigger::UserRejection,
                    notes.unwrap_or("Plan rejected, restarting analysis".to_string()),
                )
                .await?;
                // Restart requirement analysis
                self.execute_stage(workflow_run_id, WorkflowStage::RequirementAnalysis)
                    .await?;
            }
            (WorkflowStage::PendingTestReview, UserAction::Approve) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::PushPreparation,
                    TransitionTrigger::UserApproval,
                    notes.unwrap_or("Tests approved".to_string()),
                )
                .await?;
                // Start push preparation
                self.execute_stage(workflow_run_id, WorkflowStage::PushPreparation)
                    .await?;
            }
            (WorkflowStage::PendingTestReview, UserAction::Reject) => {
                self.transition_stage(
                    workflow_run_id,
                    current_stage.clone(),
                    WorkflowStage::Coding,
                    TransitionTrigger::UserRejection,
                    notes.unwrap_or("Tests rejected, restarting coding".to_string()),
                )
                .await?;
                // Restart coding
                self.execute_stage(workflow_run_id, WorkflowStage::Coding)
                    .await?;
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "Invalid action {} for stage {}",
                    action.as_str(),
                    current_stage.as_str()
                )))
            }
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

            if !bypass_coordinator_review && self.requires_coordinator_review(&stage).await? {
                if self
                    .has_active_coordinator_for_workflow(workflow_run_id)
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

    async fn requires_coordinator_review(&self, stage: &WorkflowStage) -> Result<bool, AppError> {
        if let Some(policy) =
            agent_repo::get_workflow_stage_policy(&self.db, stage.as_str()).await?
        {
            return Ok(policy.coordinator_required);
        }
        Ok(stage.requires_coordinator_review())
    }

    async fn execute_coordinator_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing coordinator review stage");
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let pending_stage_name = workflow_run.pending_stage_name.clone().ok_or_else(|| {
            AppError::Validation("Coordinator review requires a pending target stage".to_string())
        })?;
        let target_stage = WorkflowStage::from_str(&pending_stage_name)
            .map_err(|e| AppError::Validation(format!("Invalid pending workflow stage: {}", e)))?;

        if !self
            .has_active_coordinator_for_workflow(workflow_run_id)
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

    async fn has_active_coordinator_for_workflow(
        &self,
        workflow_run_id: &str,
    ) -> Result<bool, AppError> {
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;

        if let Some(coordinator_id) = workflow_run.coordinator_agent_id.as_deref() {
            if let Ok(agent) = agent_repo::get_agent_definition(&self.db, coordinator_id).await {
                if agent.enabled
                    && agent.employment_status == "active"
                    && Self::is_coordinator_role(&agent.role)
                {
                    return Ok(true);
                }
            }
        }

        if let Some(team_id) = workflow_run.assigned_team_id.as_deref() {
            return Ok(agent_repo::find_team_coordinator(&self.db, team_id)
                .await?
                .is_some());
        }

        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;
        if let Some(team) = agent_repo::resolve_team_for_work_item(&self.db, &work_item).await? {
            return Ok(agent_repo::find_team_coordinator(&self.db, &team.id)
                .await?
                .is_some());
        }

        Ok(false)
    }
    async fn execute_requirement_analysis(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing requirement analysis stage");

        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "requirement_analysis")
                .await?
        };
        debug!(workflow_run_id = %workflow_run_id, agent_run_id = %agent_run.id, agent_run_status = ?agent_run.status, "Completed agent run for requirement analysis");

        // On completion, transition to planning
        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::RequirementAnalysis,
                WorkflowStage::Planning,
                TransitionTrigger::AgentCompletion,
                "Requirement analysis completed".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::Planning)
                .await?;
            info!(workflow_run_id = %workflow_run_id, "Successfully transitioned to planning stage");
        } else {
            debug!(workflow_run_id = %workflow_run_id, agent_run_status = ?agent_run.status, "Agent run not completed, staying in requirement analysis stage");
        }

        Ok(())
    }

    /// Execute planning stage
    async fn execute_planning(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing planning stage");

        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "planning")
                .await?
        };
        debug!(workflow_run_id = %workflow_run_id, agent_run_id = %agent_run.id, agent_run_status = ?agent_run.status, "Completed agent run for planning");

        // On completion, transition to plan approval
        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::Planning,
                WorkflowStage::PendingPlanApproval,
                TransitionTrigger::AgentCompletion,
                "Planning completed".to_string(),
            )
            .await?;
            if self.is_auto_plan_approval_enabled().await? {
                let workflow_run =
                    workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
                self.record_plan_approval(
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
        } else {
            debug!(workflow_run_id = %workflow_run_id, agent_run_status = ?agent_run.status, "Agent run not completed, staying in planning stage");
        }

        Ok(())
    }

    /// Execute coding stage
    async fn execute_coding(&self, workflow_run_id: &str) -> Result<(), AppError> {
        info!(workflow_run_id = %workflow_run_id, "Executing coding stage");

        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "coding")
                .await?
        };
        debug!(workflow_run_id = %workflow_run_id, agent_run_id = %agent_run.id, agent_run_status = ?agent_run.status, "Completed agent run for coding");

        // On completion, transition to unit test generation
        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::Coding,
                WorkflowStage::UnitTestGeneration,
                TransitionTrigger::AgentCompletion,
                "Coding completed".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::UnitTestGeneration)
                .await?;
            info!(workflow_run_id = %workflow_run_id, "Successfully transitioned to unit test generation stage");
        } else {
            debug!(workflow_run_id = %workflow_run_id, agent_run_status = ?agent_run.status, "Agent run not completed, staying in coding stage");
        }

        Ok(())
    }

    /// Execute unit test generation stage
    async fn execute_unit_test_generation(&self, workflow_run_id: &str) -> Result<(), AppError> {
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "unit_test_generation")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::UnitTestGeneration,
                WorkflowStage::IntegrationTestGeneration,
                TransitionTrigger::AgentCompletion,
                "Unit test generation completed".to_string(),
            )
            .await?;
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
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "integration_test_generation")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::IntegrationTestGeneration,
                WorkflowStage::UiTestPlanning,
                TransitionTrigger::AgentCompletion,
                "Integration test generation completed".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::UiTestPlanning)
                .await?;
        }

        Ok(())
    }

    /// Execute UI test planning stage
    async fn execute_ui_test_planning(&self, workflow_run_id: &str) -> Result<(), AppError> {
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "ui_test_planning")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::UiTestPlanning,
                WorkflowStage::DockerTestExecution,
                TransitionTrigger::AgentCompletion,
                "UI test planning completed".to_string(),
            )
            .await?;
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
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "qa_validation")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::QaValidation,
                WorkflowStage::SecurityReview,
                TransitionTrigger::AgentCompletion,
                "QA validation completed".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::SecurityReview)
                .await?;
        }

        Ok(())
    }

    /// Execute security review stage
    async fn execute_security_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "security_review")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::SecurityReview,
                WorkflowStage::PerformanceReview,
                TransitionTrigger::AgentCompletion,
                "Security review completed".to_string(),
            )
            .await?;
            self.execute_stage(workflow_run_id, WorkflowStage::PerformanceReview)
                .await?;
        }

        Ok(())
    }

    /// Execute performance review stage
    async fn execute_performance_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        let agent_run = {
            let agent_service = self.agent_service.lock().await;
            agent_service
                .run_agent_for_stage(workflow_run_id, "performance_review")
                .await?
        };

        if agent_run.status == AgentRunStatus::Completed {
            self.transition_stage(
                workflow_run_id,
                WorkflowStage::PerformanceReview,
                WorkflowStage::PendingTestReview,
                TransitionTrigger::AgentCompletion,
                "Performance review completed".to_string(),
            )
            .await?;
            if self.is_auto_test_review_enabled().await? {
                let workflow_run =
                    workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
                self.record_test_review_approval(
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
        use crate::execution::git_ops::GitOperations;

        // Get workflow run and work item info
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;

        if let Some(repo_id) = &work_item.active_repo_id {
            let repo =
                crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?;
            let branch_name = work_item
                .branch_name
                .clone()
                .unwrap_or_else(|| "main".to_string());

            let commit_message = format!("Implement work item: {}", work_item.title);
            match GitOperations::create_commit(&repo.local_path, &commit_message) {
                Ok(commit_id) => {
                    GitOperations::push_to_remote(&repo.local_path, "origin", &branch_name)?;
                    info!(
                        "Successfully pushed commit {} for work item {}",
                        commit_id, work_item.id
                    );
                }
                Err(AppError::Internal(message)) if message.contains("No changes to commit") => {
                    warn!(
                        work_item_id = %work_item.id,
                        repo_id = %repo.id,
                        "Skipping git push because no repository changes were produced for this work item"
                    );
                }
                Err(error) => return Err(error),
            }
        }

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
        let notes = self
            .annotate_transition_notes(workflow_run_id, &to, notes)
            .await?;
        info!(
            "Transitioning workflow {} from {} to {} ({})",
            workflow_run_id,
            from.as_str(),
            to.as_str(),
            notes
        );

        // Validate transition
        if !transitions::is_valid_transition(&from, &to) {
            return Err(AppError::Validation(format!(
                "Invalid transition from {} to {}",
                from.as_str(),
                to.as_str()
            )));
        }

        // Update workflow run
        workflow_repo::update_workflow_stage(&self.db, workflow_run_id, &to.as_str()).await?;
        let (status, error_message, mark_ended) = match to {
            WorkflowStage::Done => ("completed", None, true),
            WorkflowStage::Failed => ("failed", Some(notes.as_str()), true),
            WorkflowStage::Cancelled => ("cancelled", Some(notes.as_str()), true),
            _ => ("running", None, false),
        };
        workflow_repo::update_workflow_lifecycle(
            &self.db,
            workflow_run_id,
            status,
            error_message,
            mark_ended,
        )
        .await?;

        // Record transition history
        let transition_id = uuid::Uuid::new_v4().to_string();
        workflow_repo::record_stage_transition(
            &self.db,
            &transition_id,
            workflow_run_id,
            &from.as_str(),
            &to.as_str(),
            &trigger.as_str(),
            &notes,
        )
        .await?;

        // Emit event for UI updates
        // TODO: Implement event emission

        Ok(())
    }

    async fn annotate_transition_notes(
        &self,
        workflow_run_id: &str,
        to_stage: &WorkflowStage,
        notes: String,
    ) -> Result<String, AppError> {
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let team_name = match workflow_run.assigned_team_id.as_deref() {
            Some(team_id) => agent_repo::get_agent_team(&self.db, team_id)
                .await
                .ok()
                .map(|team| team.name),
            None => None,
        };
        let coordinator_name = match workflow_run.coordinator_agent_id.as_deref() {
            Some(agent_id) => agent_repo::get_agent_definition(&self.db, agent_id)
                .await
                .ok()
                .map(|agent| agent.name),
            None => None,
        };

        let mut annotated = notes;
        if let Some(team_name) = team_name {
            annotated.push_str(&format!(" | Team: {team_name}"));
        }
        if let Some(coordinator_name) = coordinator_name {
            let label = if to_stage.is_approval_gate() {
                "Approval owner"
            } else {
                "Coordinator"
            };
            annotated.push_str(&format!(" | {label}: {coordinator_name}"));
        }

        Ok(annotated)
    }

    /// Advance workflow to the next stage
    pub async fn advance_workflow(&self, workflow_run_id: &str) -> Result<(), AppError> {
        debug!(workflow_run_id = %workflow_run_id, "Advancing workflow");

        let mut workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
            .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Retrieved current workflow stage");

        // Check if we can advance
        if !current_stage.is_terminal() {
            if let Some(next_stage) = self.engine.next_stage(&current_stage) {
                debug!(workflow_run_id = %workflow_run_id, next_stage = %next_stage.as_str(), "Determined next workflow stage");

                // Handle special stages that need additional processing
                match next_stage {
                    WorkflowStage::GitPush => {
                        self.handle_git_push(&workflow_run).await?;
                    }
                    _ => {}
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
                    &current_stage.as_str(),
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
        use crate::execution::git_ops::GitOperations;

        // Get work item and repository info
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;
        if let Some(repo_id) = &work_item.active_repo_id {
            let repo =
                crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?;

            // Stage all changes
            GitOperations::stage_all_changes(&repo.local_path)?;

            // Create commit
            let commit_message = format!("Implement work item: {}", work_item.title);
            let commit_id = GitOperations::create_commit(&repo.local_path, &commit_message)?;

            // Push to remote
            GitOperations::push_to_remote(
                &repo.local_path,
                "origin",
                &work_item.branch_name.unwrap_or_else(|| "main".to_string()),
            )?;

            info!(
                "Successfully pushed commit {} for work item {}",
                commit_id, work_item.id
            );
        }

        Ok(())
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
mod tests {
    use super::WorkflowService;
    use crate::domain::workflow::UserAction;
    use crate::persistence::{
        agent_repo, artifact_repo, db as db_service, model_repo, product_repo, repository_repo,
        settings_repo, work_item_repo,
    };
    use crate::services::{agent_service::AgentService, model_service::ModelService};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{Arc, OnceLock};
    use tokio::sync::{Mutex, OwnedMutexGuard};
    use tokio::time::{sleep, Duration, Instant};

    fn workflow_test_lock() -> Arc<Mutex<()>> {
        static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
        LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
    }

    async fn acquire_workflow_test_lock() -> OwnedMutexGuard<()> {
        workflow_test_lock().lock_owned().await
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aruvi_workflow_service_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    #[tokio::test]
    async fn plan_approval_continues_to_coding_and_records_history_and_artifacts() {
        let _test_guard = acquire_workflow_test_lock().await;
        let temp_root = make_temp_dir("plan_approval");
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");

        let repo_dir = temp_root.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
        std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
            .expect("failed to seed repository file");

        let repository = repository_repo::create_repository(
            &pool,
            "workflow-test-repo",
            "Workflow Test Repo",
            &repo_dir.to_string_lossy(),
            "",
            "main",
        )
        .await
        .expect("failed to create repository");

        let product = product_repo::create_product(
            &pool,
            "workflow-product",
            "Workflow Product",
            "desc",
            "vision",
            "[]",
            "[]",
        )
        .await
        .expect("failed to create product");
        let module = product_repo::create_module(
            &pool,
            "workflow-module",
            &product.id,
            "Delivery Module",
            "desc",
            "purpose",
        )
        .await
        .expect("failed to create module");

        let work_item = work_item_repo::create_work_item(
            &pool,
            "workflow-work-item",
            &product.id,
            Some(&module.id),
            None,
            None,
            "Implement workflow continuation",
            "problem",
            "description",
            "acceptance",
            "constraints",
            "feature",
            "medium",
            "medium",
        )
        .await
        .expect("failed to create work item");

        sqlx::query("UPDATE work_items SET status='approved', active_repo_id=? WHERE id=?")
            .bind(&repository.id)
            .bind(&work_item.id)
            .execute(&pool)
            .await
            .expect("failed to prepare approved work item");

        let db_arc = Arc::new(pool.clone());
        let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
        let artifact_dir = temp_root.join("artifacts");
        let workspace_dir = temp_root.join("workspaces");
        std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
        std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

        let agent_service = AgentService::new(
            Arc::clone(&db_arc),
            Arc::clone(&model_service),
            artifact_dir.clone(),
            workspace_dir.clone(),
        );
        let workflow_service =
            WorkflowService::new(Arc::clone(&db_arc), Arc::new(Mutex::new(agent_service)));

        settings_repo::set_setting(&pool, super::AUTO_APPROVE_PLAN_KEY, "false")
            .await
            .expect("failed to disable auto plan approval for manual-gate test");
        settings_repo::set_setting(&pool, super::AUTO_APPROVE_TEST_REVIEW_KEY, "false")
            .await
            .expect("failed to disable auto test review for manual-gate test");

        AgentService::set_test_model_outputs_for_any_workflow(vec![
            "requirement analysis complete".to_string(),
            "planning complete".to_string(),
            json!({
                "type": "tool_call",
                "tool": "repo.write_file",
                "reason": "implement approved plan",
                "arguments": {
                    "path": "src/generated.rs",
                    "content": "pub fn generated() -> &'static str { \"ok\" }\n"
                }
            })
            .to_string(),
            json!({
                "type": "final",
                "summary": "coding complete",
                "result": "implemented"
            })
            .to_string(),
            "unit test generation complete".to_string(),
            "integration test generation complete".to_string(),
            "ui test planning complete".to_string(),
            "qa validation complete".to_string(),
            "security review complete".to_string(),
            "performance review complete".to_string(),
        ]);

        let workflow_run = workflow_service
            .start_work_item_workflow(&work_item.id)
            .await
            .expect("failed to start workflow");

        let at_plan_gate = workflow_service
            .get_workflow_run(&workflow_run.id)
            .await
            .expect("failed to refresh workflow after start");
        assert_eq!(at_plan_gate.current_stage, "pending_plan_approval");

        let artifacts_before_approval =
            artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
                .await
                .expect("failed to list artifacts before plan approval");
        assert!(
            artifacts_before_approval
                .iter()
                .any(|artifact| artifact.artifact_type == "planning_prompt"),
            "missing planning_prompt artifact before plan approval"
        );
        assert!(
            artifacts_before_approval
                .iter()
                .any(|artifact| artifact.artifact_type == "planning_output"),
            "missing planning_output artifact before plan approval"
        );

        workflow_service
            .handle_user_action(
                &workflow_run.id,
                UserAction::Approve,
                Some("approve test plan".to_string()),
            )
            .await
            .expect("failed to approve plan and continue workflow");

        let post_approval = workflow_service
            .get_workflow_run(&workflow_run.id)
            .await
            .expect("failed to refresh workflow after approval");
        assert_eq!(post_approval.current_stage, "pending_test_review");

        let history = workflow_service
            .get_workflow_history(&workflow_run.id)
            .await
            .expect("failed to load workflow history");
        assert!(
            history.iter().any(|entry| {
                entry.from_stage == "planning" && entry.to_stage == "pending_plan_approval"
            }),
            "missing planning -> pending_plan_approval transition"
        );
        assert!(
            history.iter().any(|entry| {
                entry.from_stage == "pending_plan_approval" && entry.to_stage == "coding"
            }),
            "missing pending_plan_approval -> coding transition"
        );

        let artifacts_after_approval =
            artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
                .await
                .expect("failed to list artifacts after plan approval");
        assert!(
            artifacts_after_approval
                .iter()
                .any(|artifact| artifact.artifact_type == "coding_tool_trace"),
            "missing coding_tool_trace artifact after approval-driven coding"
        );
        assert!(
            artifacts_after_approval
                .iter()
                .any(|artifact| artifact.artifact_type == "coding_output"),
            "missing coding_output artifact after approval-driven coding"
        );

        let generated = std::fs::read_to_string(repo_dir.join("src/generated.rs"))
            .expect("expected generated file from coding stage");
        assert!(
            generated.contains("generated"),
            "generated file content did not match expected coding output"
        );

        AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[tokio::test]
    async fn planning_auto_approval_continues_directly_to_coding_by_default() {
        let _test_guard = acquire_workflow_test_lock().await;
        let temp_root = make_temp_dir("auto_plan_approval");
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");

        let repo_dir = temp_root.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
        std::fs::write(repo_dir.join("README.md"), "# workflow repo\n")
            .expect("failed to seed repository file");

        let repository = repository_repo::create_repository(
            &pool,
            "workflow-auto-approval-repo",
            "Workflow Auto Approval Repo",
            &repo_dir.to_string_lossy(),
            "",
            "main",
        )
        .await
        .expect("failed to create repository");

        let product = product_repo::create_product(
            &pool,
            "workflow-auto-approval-product",
            "Workflow Product",
            "desc",
            "vision",
            "[]",
            "[]",
        )
        .await
        .expect("failed to create product");
        let module = product_repo::create_module(
            &pool,
            "workflow-auto-approval-module",
            &product.id,
            "Delivery Module",
            "desc",
            "purpose",
        )
        .await
        .expect("failed to create module");

        let work_item = work_item_repo::create_work_item(
            &pool,
            "workflow-auto-approval-work-item",
            &product.id,
            Some(&module.id),
            None,
            None,
            "Implement workflow continuation",
            "problem",
            "description",
            "acceptance",
            "constraints",
            "feature",
            "medium",
            "medium",
        )
        .await
        .expect("failed to create work item");

        sqlx::query("UPDATE work_items SET status='approved', active_repo_id=? WHERE id=?")
            .bind(&repository.id)
            .bind(&work_item.id)
            .execute(&pool)
            .await
            .expect("failed to prepare approved work item");

        let db_arc = Arc::new(pool.clone());
        let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
        let artifact_dir = temp_root.join("artifacts");
        let workspace_dir = temp_root.join("workspaces");
        std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
        std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

        let agent_service = AgentService::new(
            Arc::clone(&db_arc),
            Arc::clone(&model_service),
            artifact_dir.clone(),
            workspace_dir.clone(),
        );
        let workflow_service =
            WorkflowService::new(Arc::clone(&db_arc), Arc::new(Mutex::new(agent_service)));

        settings_repo::set_setting(&pool, super::AUTO_APPROVE_TEST_REVIEW_KEY, "false")
            .await
            .expect("failed to disable auto test review for auto-plan test");

        AgentService::set_test_model_outputs_for_any_workflow(vec![
            "requirement analysis complete".to_string(),
            "planning complete".to_string(),
            json!({
                "type": "tool_call",
                "tool": "repo.write_file",
                "reason": "implement approved plan",
                "arguments": {
                    "path": "src/generated.rs",
                    "content": "pub fn generated() -> &'static str { \"ok\" }\n"
                }
            })
            .to_string(),
            json!({
                "type": "final",
                "summary": "coding complete",
                "result": "implemented"
            })
            .to_string(),
            "unit test generation complete".to_string(),
            "integration test generation complete".to_string(),
            "ui test planning complete".to_string(),
            "qa validation complete".to_string(),
            "security review complete".to_string(),
            "performance review complete".to_string(),
        ]);

        let workflow_run = workflow_service
            .start_work_item_workflow(&work_item.id)
            .await
            .expect("failed to start workflow");

        let after_planning = workflow_service
            .get_workflow_run(&workflow_run.id)
            .await
            .expect("failed to load workflow after auto plan approval");
        assert_eq!(after_planning.current_stage, "pending_test_review");

        let approvals = crate::persistence::approval_repo::list_approvals(&pool, &work_item.id)
            .await
            .expect("failed to list approvals");
        assert!(
            approvals.iter().any(|approval| {
                approval.approval_type.to_string() == "plan_approval"
                    && approval.status.to_string() == "approved"
                    && approval.notes.contains("auto-approved")
            }),
            "expected auto-approved plan approval record"
        );

        let history = workflow_service
            .get_workflow_history(&workflow_run.id)
            .await
            .expect("failed to load workflow history");
        assert!(
            history.iter().any(|entry| {
                entry.from_stage == "pending_plan_approval"
                    && entry.to_stage == "coding"
                    && entry.notes.contains("auto-approved")
            }),
            "missing auto-approved pending_plan_approval -> coding transition"
        );

        AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[tokio::test]
    async fn auto_test_review_continues_to_done_by_default() {
        let _test_guard = acquire_workflow_test_lock().await;
        let temp_root = make_temp_dir("auto_test_review");
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");

        let repo_dir = temp_root.join("repo");
        let remote_bare_dir = temp_root.join("origin.git");
        create_empty_calculator_test_repo(&repo_dir, &remote_bare_dir)
            .expect("failed to create repo with bare remote");

        let repository = repository_repo::create_repository(
            &pool,
            "workflow-auto-test-review-repo",
            "Workflow Auto Test Review Repo",
            &repo_dir.to_string_lossy(),
            &remote_bare_dir.to_string_lossy(),
            "main",
        )
        .await
        .expect("failed to create repository");

        let product = product_repo::create_product(
            &pool,
            "workflow-auto-test-review-product",
            "Workflow Product",
            "desc",
            "vision",
            "[]",
            "[]",
        )
        .await
        .expect("failed to create product");
        let module = product_repo::create_module(
            &pool,
            "workflow-auto-test-review-module",
            &product.id,
            "Delivery Module",
            "desc",
            "purpose",
        )
        .await
        .expect("failed to create module");

        let work_item = work_item_repo::create_work_item(
            &pool,
            "workflow-auto-test-review-work-item",
            &product.id,
            Some(&module.id),
            None,
            None,
            "Implement workflow continuation",
            "problem",
            "description",
            "acceptance",
            "constraints",
            "feature",
            "medium",
            "medium",
        )
        .await
        .expect("failed to create work item");

        sqlx::query("UPDATE work_items SET status='approved', active_repo_id=? WHERE id=?")
            .bind(&repository.id)
            .bind(&work_item.id)
            .execute(&pool)
            .await
            .expect("failed to prepare approved work item");

        let db_arc = Arc::new(pool.clone());
        let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
        let artifact_dir = temp_root.join("artifacts");
        let workspace_dir = temp_root.join("workspaces");
        std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
        std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

        let agent_service = AgentService::new(
            Arc::clone(&db_arc),
            Arc::clone(&model_service),
            artifact_dir.clone(),
            workspace_dir.clone(),
        );
        let workflow_service =
            WorkflowService::new(Arc::clone(&db_arc), Arc::new(Mutex::new(agent_service)));

        AgentService::set_test_model_outputs_for_any_workflow(vec![
            "requirement analysis complete".to_string(),
            "planning complete".to_string(),
            json!({
                "type": "tool_call",
                "tool": "repo.write_file",
                "reason": "implement approved plan",
                "arguments": {
                    "path": "src/generated.rs",
                    "content": "pub fn generated() -> &'static str { \"ok\" }\n"
                }
            })
            .to_string(),
            json!({
                "type": "final",
                "summary": "coding complete",
                "result": "implemented"
            })
            .to_string(),
            "unit test generation complete".to_string(),
            "integration test generation complete".to_string(),
            "ui test planning complete".to_string(),
            "qa validation complete".to_string(),
            "security review complete".to_string(),
            "performance review complete".to_string(),
        ]);

        let workflow_run = workflow_service
            .start_work_item_workflow(&work_item.id)
            .await
            .expect("failed to start workflow");

        let final_state = workflow_service
            .get_workflow_run(&workflow_run.id)
            .await
            .expect("failed to load workflow after auto test review");
        assert_eq!(final_state.current_stage, "done");

        let approvals = crate::persistence::approval_repo::list_approvals(&pool, &work_item.id)
            .await
            .expect("failed to list approvals");
        assert!(
            approvals.iter().any(|approval| {
                approval.approval_type.to_string() == "test_review"
                    && approval.status.to_string() == "approved"
                    && approval.notes.contains("auto-approved")
            }),
            "expected auto-approved test review record"
        );

        let history = workflow_service
            .get_workflow_history(&workflow_run.id)
            .await
            .expect("failed to load workflow history");
        assert!(
            history.iter().any(|entry| {
                entry.from_stage == "pending_test_review"
                    && entry.to_stage == "push_preparation"
                    && entry.notes.contains("auto-approved")
            }),
            "missing auto-approved pending_test_review -> push_preparation transition"
        );

        AgentService::set_test_model_outputs_for_any_workflow(Vec::new());
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[tokio::test]
    #[ignore = "requires live model provider configuration and can take significant time"]
    async fn live_calculator_iterative_workflow_smoke() {
        let _test_guard = acquire_workflow_test_lock().await;
        let temp_root = make_temp_dir("live_calculator");
        println!("LIVE_TEST_ROOT={}", temp_root.display());
        let db_path = temp_root.join("aruvi-live.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");

        let repo_dir = temp_root.join("calculator-test-repo");
        let remote_bare_dir = temp_root.join("calculator-origin.git");
        create_empty_calculator_test_repo(&repo_dir, &remote_bare_dir)
            .expect("failed to create empty calculator test repository");
        let repository = repository_repo::create_repository(
            &pool,
            "calculator-repo",
            "Calculator React Test",
            &repo_dir.to_string_lossy(),
            &remote_bare_dir.to_string_lossy(),
            "main",
        )
        .await
        .expect("failed to register calculator repository");

        configure_live_model_bindings(&pool)
            .await
            .expect("failed to bind live model for agents");

        let product = product_repo::create_product(
            &pool,
            "calculator-product",
            "Calculator",
            "Iterative calculator delivery with full workflow enforcement.",
            "Deliver calculator outcomes in small, validated increments.",
            "[]",
            "[\"react\",\"calculator\",\"agentic\"]",
        )
        .await
        .expect("failed to create Calculator product");

        let module = product_repo::create_module(
            &pool,
            "calculator-module",
            &product.id,
            "Calculator Engine",
            "Core capability delivery module for calculator behavior.",
            "Implement and validate calculator functionality end-to-end.",
        )
        .await
        .expect("failed to create calculator module");

        let bootstrap_work_item = work_item_repo::create_work_item(
            &pool,
            "work-item-bootstrap-initialize-repo",
            &product.id,
            Some(&module.id),
            None,
            None,
            "Initialize repository and test folder",
            "Initialize an empty calculator repository baseline before capability outcomes start shipping.",
            "Create the baseline repository structure (including an empty tests folder), commit the setup, and keep follow-up outcomes focused on incremental functional changes.",
            "Repository baseline is committed, tests folder exists, and subsequent outcomes can commit changes without re-initializing the project.",
            "Do not implement calculator features in this bootstrap outcome.",
            "feature",
            "high",
            "low",
        )
        .await
        .expect("failed to create bootstrap work item");
        sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
            .bind(&repository.id)
            .bind(&bootstrap_work_item.id)
            .execute(&pool)
            .await
            .expect("failed to assign active repository to bootstrap work item");

        let capability_specs: [(&str, &[&str]); 5] = [
            (
                "Simple Math",
                &["Addition", "Subtraction", "Multiplication", "Division"],
            ),
            ("Scientific", &["Sin", "Cos", "Tan"]),
            ("Exponents", &["Square", "Cube", "Power of X"]),
            ("Roots", &["Square", "Cube"]),
            ("Programming", &["ASCII", "HEX"]),
        ];

        let mut ordered_work_item_ids: Vec<String> = vec![bootstrap_work_item.id];
        for (capability_name, outcomes) in capability_specs {
            let capability_slug = capability_name.to_ascii_lowercase().replace(' ', "-");
            let capability = product_repo::create_capability(
                &pool,
                &format!("capability-{capability_slug}"),
                &module.id,
                None,
                capability_name,
                &format!("{capability_name} capability for calculator outcomes"),
                &format!("{capability_name} outcomes: {}", outcomes.join(", ")),
                "medium",
                "low",
                "Build in iterative outcomes with full test gates.",
            )
            .await
            .expect("failed to create capability");

            for outcome in outcomes {
                let outcome_slug = outcome.to_ascii_lowercase().replace(' ', "-");
                let outcome_capability = product_repo::create_capability(
                    &pool,
                    &format!("capability-{capability_slug}-{outcome_slug}"),
                    &module.id,
                    Some(&capability.id),
                    outcome,
                    &format!("{outcome} outcome for {capability_name}"),
                    &format!("Calculator supports {outcome} for {capability_name}."),
                    "medium",
                    "low",
                    "Deliver as a focused outcome with full workflow validation.",
                )
                .await
                .expect("failed to create outcome capability");

                let work_item_id = format!("work-item-{}-{}", capability_slug, outcome_slug);
                let work_item = work_item_repo::create_work_item(
                    &pool,
                    &work_item_id,
                    &product.id,
                    Some(&module.id),
                    Some(&outcome_capability.id),
                    None,
                    &format!("{capability_name}: {outcome}"),
                    &format!("Implement {outcome} behavior for {capability_name}."),
                    &format!(
                        "Deliver the {outcome} outcome under {capability_name} in the React calculator with iterative commits and review gates."
                    ),
                    "Component behavior, unit tests, integration tests, and UI tests pass.",
                    "Stay inside React codebase and calculator scope.",
                    "feature",
                    "medium",
                    "medium",
                )
                .await
                .expect("failed to create work item");

                sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
                    .bind(&repository.id)
                    .bind(&work_item.id)
                    .execute(&pool)
                    .await
                    .expect("failed to assign active repository");

                ordered_work_item_ids.push(work_item.id);
            }
        }

        let db_arc = Arc::new(pool.clone());
        let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
        let artifact_dir = temp_root.join("artifacts");
        let workspace_dir = temp_root.join("workspaces");
        std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
        std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

        let agent_service = AgentService::new(
            Arc::clone(&db_arc),
            Arc::clone(&model_service),
            artifact_dir.clone(),
            workspace_dir.clone(),
        );
        let workflow_service =
            WorkflowService::new(Arc::clone(&db_arc), Arc::new(Mutex::new(agent_service)));

        settings_repo::set_setting(&pool, super::AUTO_APPROVE_PLAN_KEY, "false")
            .await
            .expect("failed to disable auto plan approval for live smoke test");
        settings_repo::set_setting(&pool, super::AUTO_APPROVE_TEST_REVIEW_KEY, "false")
            .await
            .expect("failed to disable auto test review for live smoke test");

        let max_iterations = std::env::var("ARUVI_LIVE_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let approval_timeout = Duration::from_secs(
            std::env::var("ARUVI_LIVE_STAGE_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1800),
        );
        let complete_to_done = std::env::var("ARUVI_LIVE_COMPLETE_TO_DONE")
            .ok()
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(true);
        let keep_temp = std::env::var("ARUVI_LIVE_KEEP_TEMP")
            .ok()
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        for work_item_id in ordered_work_item_ids.into_iter().take(max_iterations) {
            work_item_repo::update_work_item(
                &pool,
                &work_item_id,
                None,
                None,
                Some("approved"),
                None,
                None,
                None,
            )
            .await
            .expect("failed to approve work item for workflow start");

            let workflow_run = workflow_service
                .start_work_item_workflow(&work_item_id)
                .await
                .expect("failed to start workflow for live iteration");

            wait_for_stage(
                &workflow_service,
                &workflow_run.id,
                "pending_plan_approval",
                approval_timeout,
            )
            .await
            .expect("workflow never reached pending_plan_approval");

            workflow_service
                .handle_user_action(
                    &workflow_run.id,
                    UserAction::Approve,
                    Some("Auto-approved plan for live iterative test".to_string()),
                )
                .await
                .expect("failed to approve plan in live iteration");

            wait_for_stage(
                &workflow_service,
                &workflow_run.id,
                "pending_test_review",
                approval_timeout,
            )
            .await
            .expect("workflow never reached pending_test_review after plan approval");

            let artifacts = artifact_repo::list_work_item_artifacts(&pool, &work_item_id)
                .await
                .expect("failed to list artifacts for live iteration");
            assert!(
                artifacts
                    .iter()
                    .any(|artifact| artifact.artifact_type == "coding_tool_trace"),
                "expected coding_tool_trace artifact for work item {}",
                work_item_id
            );

            if complete_to_done {
                workflow_service
                    .handle_user_action(
                        &workflow_run.id,
                        UserAction::Approve,
                        Some("Auto-approved test review for live iterative test".to_string()),
                    )
                    .await
                    .expect("failed to approve test review in live iteration");
                wait_for_stage(
                    &workflow_service,
                    &workflow_run.id,
                    "done",
                    approval_timeout,
                )
                .await
                .expect("workflow never reached done after test review approval");
            }
        }

        if keep_temp {
            println!("LIVE_TEST_ROOT_PRESERVED={}", temp_root.display());
        } else {
            let _ = std::fs::remove_dir_all(temp_root);
        }
    }

    fn create_empty_calculator_test_repo(
        repo_dir: &PathBuf,
        remote_bare_dir: &PathBuf,
    ) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(repo_dir)?;
        std::fs::create_dir_all(remote_bare_dir)?;
        std::fs::create_dir_all(repo_dir.join("tests"))?;

        std::fs::write(
            repo_dir.join("README.md"),
            "# Calculator Pressure Test\n\nThis repository is intentionally initialized empty for outcome-driven agent delivery.\n",
        )?;
        std::fs::write(
            repo_dir.join(".gitignore"),
            "node_modules/\ndist/\nbuild/\ncoverage/\n",
        )?;
        std::fs::write(repo_dir.join("tests/.gitkeep"), "")?;

        run_git_command(
            repo_dir,
            &["init", "-b", "main"],
            "initialize git repository",
        )?;
        run_git_command(
            repo_dir,
            &["config", "user.name", "Aruvi Pressure Runner"],
            "set git user.name",
        )?;
        run_git_command(
            repo_dir,
            &["config", "user.email", "aruvi-pressure@example.com"],
            "set git user.email",
        )?;

        run_git_command(
            remote_bare_dir,
            &["init", "--bare"],
            "initialize bare remote repository",
        )?;

        run_git_command(
            repo_dir,
            &[
                "remote",
                "add",
                "origin",
                remote_bare_dir.to_string_lossy().as_ref(),
            ],
            "add origin remote",
        )?;
        run_git_command(repo_dir, &["add", "."], "stage bootstrap files")?;
        run_git_command(
            repo_dir,
            &[
                "commit",
                "-m",
                "chore: bootstrap empty calculator pressure repo",
            ],
            "create bootstrap commit",
        )?;
        run_git_command(
            repo_dir,
            &["push", "-u", "origin", "main"],
            "push bootstrap commit",
        )?;
        Ok(())
    }

    fn run_git_command(cwd: &PathBuf, args: &[&str], context: &str) -> Result<(), std::io::Error> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()?;
        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "git command failed while trying to {}: git {} | stdout: {} | stderr: {}",
                context,
                args.join(" "),
                stdout.trim(),
                stderr.trim()
            ),
        ))
    }

    async fn configure_live_model_bindings(pool: &sqlx::SqlitePool) -> Result<(), String> {
        let live_base_url = std::env::var("ARUVI_LIVE_BASE_URL")
            .map_err(|_| "ARUVI_LIVE_BASE_URL is required for live test".to_string())?;
        let live_model_name = std::env::var("ARUVI_LIVE_MODEL")
            .map_err(|_| "ARUVI_LIVE_MODEL is required for live test".to_string())?;
        let live_api_key = std::env::var("ARUVI_LIVE_API_KEY").ok();

        let provider_id = "live-provider";
        let model_id = "live-model";

        model_repo::create_provider(
            pool,
            provider_id,
            "Live Provider",
            "openai_compatible",
            &live_base_url,
            live_api_key.as_deref(),
        )
        .await
        .map_err(|error| format!("create_provider failed: {error}"))?;
        model_repo::create_model_definition(
            pool,
            model_id,
            provider_id,
            &live_model_name,
            Some(128000),
        )
        .await
        .map_err(|error| format!("create_model_definition failed: {error}"))?;

        let agents = agent_repo::list_agent_definitions(pool)
            .await
            .map_err(|error| format!("list_agent_definitions failed: {error}"))?;
        for agent in agents {
            agent_repo::create_agent_model_binding(
                pool,
                &uuid::Uuid::new_v4().to_string(),
                &agent.id,
                model_id,
                0,
            )
            .await
            .map_err(|error| {
                format!(
                    "create_agent_model_binding failed for {}: {error}",
                    agent.id
                )
            })?;
        }

        Ok(())
    }

    async fn wait_for_stage(
        service: &WorkflowService,
        workflow_run_id: &str,
        target_stage: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        let start = Instant::now();
        loop {
            let run = service
                .get_workflow_run(workflow_run_id)
                .await
                .map_err(|error| format!("get_workflow_run failed: {error}"))?;
            if run.current_stage == target_stage {
                return Ok(());
            }
            if ["failed", "cancelled", "done"].contains(&run.current_stage.as_str())
                && run.current_stage != target_stage
            {
                return Err(format!(
                    "workflow reached terminal stage {} before {}",
                    run.current_stage, target_stage
                ));
            }
            if start.elapsed() > timeout {
                return Err(format!(
                    "timeout waiting for stage {} (last stage: {})",
                    target_stage, run.current_stage
                ));
            }
            sleep(Duration::from_secs(2)).await;
        }
    }
}

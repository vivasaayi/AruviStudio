use super::WorkflowService;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::services::{
    workflow_coordinator_policy, workflow_coordinator_review, workflow_failure,
    workflow_repository_guard, workflow_stage_transition,
};
use std::future::Future;
use std::pin::Pin;
use tracing::{error, info, warn};

impl WorkflowService {
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

    /// Execute a workflow stage.
    pub(super) fn execute_stage<'a>(
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
                if let Err(mark_error) = workflow_failure::mark_workflow_failed(
                    self.db.as_ref(),
                    workflow_run_id,
                    &stage,
                    &stage_error,
                )
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

    async fn route_through_coordinator_review(
        &self,
        workflow_run_id: &str,
        target_stage: WorkflowStage,
    ) -> Result<(), AppError> {
        workflow_coordinator_review::prepare_coordinator_review(
            self.db.as_ref(),
            workflow_run_id,
            target_stage,
        )
        .await?;
        self.execute_stage_internal(workflow_run_id, WorkflowStage::CoordinatorReview, true)
            .await
    }

    async fn execute_coordinator_review(&self, workflow_run_id: &str) -> Result<(), AppError> {
        if let Some(target_stage) = workflow_coordinator_review::execute_coordinator_review(
            self.db.as_ref(),
            &self.agent_service,
            workflow_run_id,
        )
        .await?
        {
            self.execute_stage_internal(workflow_run_id, target_stage, true)
                .await?;
        }

        Ok(())
    }

    /// Transition workflow to a new stage.
    pub(super) async fn transition_stage(
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
}

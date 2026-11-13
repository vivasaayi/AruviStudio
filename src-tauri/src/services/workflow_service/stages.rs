use super::WorkflowService;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::services::{
    workflow_coordinator_policy, workflow_coordinator_review, workflow_failure,
    workflow_repository_guard, workflow_stage_transition,
};
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

    /// Execute automatic stage transitions without recursively polling prior stages.
    pub(super) async fn execute_stage(
        &self,
        workflow_run_id: &str,
        mut stage: WorkflowStage,
    ) -> Result<(), AppError> {
        let mut bypass_coordinator_review = false;
        loop {
            info!(
                "Executing stage {} for workflow run {}",
                stage.as_str(),
                workflow_run_id
            );
            if let Err(stage_error) = self
                .ensure_repository_ready_for_stage(workflow_run_id, &stage)
                .await
            {
                if let Err(mark_error) = workflow_failure::mark_workflow_failed(
                    self.db.as_ref(),
                    workflow_run_id,
                    &stage,
                    &stage_error,
                )
                .await
                {
                    error!(workflow_run_id = %workflow_run_id, stage = %stage.as_str(), error = %mark_error, "Failed to persist workflow failure state");
                }
                return Err(stage_error);
            }

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
                    workflow_coordinator_review::prepare_coordinator_review(
                        self.db.as_ref(),
                        workflow_run_id,
                        stage,
                    )
                    .await?;
                    stage = WorkflowStage::CoordinatorReview;
                    bypass_coordinator_review = true;
                    continue;
                }
                warn!(workflow_run_id = %workflow_run_id, stage = %stage.as_str(), "Coordinator review required by policy but no active coordinator is available; bypassing coordinator stage");
            }

            let execution_result = match stage {
                WorkflowStage::CoordinatorReview => {
                    workflow_coordinator_review::execute_coordinator_review(
                        self.db.as_ref(),
                        &self.agent_service,
                        workflow_run_id,
                    )
                    .await
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
                    Ok(None)
                }
            };
            match execution_result {
                Ok(Some(next_stage)) => {
                    bypass_coordinator_review = stage == WorkflowStage::CoordinatorReview;
                    stage = next_stage;
                }
                Ok(None) => return Ok(()),
                Err(stage_error) => {
                    error!(workflow_run_id = %workflow_run_id, stage = %stage.as_str(), error = %stage_error, "Stage execution failed");
                    if let Err(mark_error) = workflow_failure::mark_workflow_failed(
                        self.db.as_ref(),
                        workflow_run_id,
                        &stage,
                        &stage_error,
                    )
                    .await
                    {
                        error!(workflow_run_id = %workflow_run_id, stage = %stage.as_str(), error = %mark_error, "Failed to persist workflow failure state");
                    }
                    return Err(stage_error);
                }
            }
        }
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

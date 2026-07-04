use super::WorkflowService;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::workflow_repo;
use crate::services::{workflow_agent_stage, workflow_approval_gate};
use tracing::info;

impl WorkflowService {
    pub(super) async fn execute_planning(&self, workflow_run_id: &str) -> Result<(), AppError> {
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

    pub(super) async fn execute_performance_review(
        &self,
        workflow_run_id: &str,
    ) -> Result<(), AppError> {
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
}

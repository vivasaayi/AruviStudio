use super::WorkflowService;
use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::services::workflow_agent_stage;
use tracing::info;

impl WorkflowService {
    pub(super) async fn execute_requirement_analysis(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::Planning));
        }

        Ok(None)
    }

    pub(super) async fn execute_coding(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::UnitTestGeneration));
        }

        Ok(None)
    }

    pub(super) async fn execute_unit_test_generation(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::IntegrationTestGeneration));
        }

        Ok(None)
    }

    pub(super) async fn execute_integration_test_generation(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::UiTestPlanning));
        }

        Ok(None)
    }

    pub(super) async fn execute_ui_test_planning(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::DockerTestExecution));
        }

        Ok(None)
    }

    pub(super) async fn execute_qa_validation(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::SecurityReview));
        }

        Ok(None)
    }

    pub(super) async fn execute_security_review(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowStage>, AppError> {
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
            return Ok(Some(WorkflowStage::PerformanceReview));
        }

        Ok(None)
    }
}

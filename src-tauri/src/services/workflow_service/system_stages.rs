use super::WorkflowService;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::services::workflow_git_push;

impl WorkflowService {
    pub(super) async fn execute_docker_test_execution(
        &self,
        workflow_run_id: &str,
    ) -> Result<(), AppError> {
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

    pub(super) async fn execute_push_preparation(
        &self,
        workflow_run_id: &str,
    ) -> Result<(), AppError> {
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

    pub(super) async fn execute_git_push(&self, workflow_run_id: &str) -> Result<(), AppError> {
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
}

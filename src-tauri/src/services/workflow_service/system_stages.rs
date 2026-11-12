use super::WorkflowService;
use crate::domain::workflow::{TransitionTrigger, WorkflowStage};
use crate::error::AppError;
use crate::persistence::{artifact_repo, repository_repo, settings_repo, work_item_repo, workflow_repo};
use crate::services::{workflow_git_push, workflow_test_runner};

impl WorkflowService {
    pub(super) async fn execute_docker_test_execution(
        &self,
        workflow_run_id: &str,
    ) -> Result<(), AppError> {
        let workflow = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let result_key = format!("workflow.test_result.{workflow_run_id}");
        settings_repo::set_setting(&self.db, &result_key, "running").await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow.work_item_id).await?;
        let (_, args) = workflow_test_runner::resolve_command(&self.db, work_item.product_id.as_deref().unwrap_or("")).await?;
        let (outcome, report) = if !args.is_empty() {
            let repository = match work_item.active_repo_id.as_deref() {
                Some(id) => Some(repository_repo::get_repository(&self.db, id).await?),
                None => repository_repo::resolve_repository_for_work_item(&self.db, &work_item.id).await?,
            }.ok_or_else(|| AppError::Validation("A repository is required to run tests".into()))?;
            match workflow_test_runner::run_command(std::path::Path::new(&repository.local_path), &args).await {
                Ok(result) => {
                    let outcome = if result.success { "passed" } else { "failed" };
                    (outcome, serde_json::json!({"status":outcome,"exit_code":result.exit_code,"timed_out":result.timed_out,"stdout":result.stdout,"stderr":result.stderr}))
                }
                Err(error) => ("failed", serde_json::json!({"status":"failed", "error":error.to_string()})),
            }
        } else {
            ("skipped", serde_json::json!({"status":"skipped", "reason":"No workflow test command is configured; manual review is required."}))
        };
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let artifact_base = self.agent_service.lock().await.artifact_base_path().to_owned();
        let dir = artifact_base.join("workflow-tests").join(workflow_run_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{artifact_id}.json"));
        tokio::fs::write(&path, serde_json::to_vec_pretty(&report)?).await?;
        let summary = match outcome {
            "passed" => "Configured test command passed",
            "failed" => "Configured test command failed",
            _ => "Tests were not run; manual test review is required",
        };
        artifact_repo::create_artifact(self.db.as_ref(), artifact_repo::CreateArtifactInput {
            id: &artifact_id, work_item_id: &workflow.work_item_id, workflow_run_id: Some(workflow_run_id),
            agent_run_id: None, artifact_type: "workflow_test_result", summary, storage_path: &path.to_string_lossy(),
        }).await?;
        settings_repo::set_setting(&self.db, &result_key, outcome).await?;
        if outcome == "failed" {
            return Err(AppError::Validation("Configured test command failed; inspect the workflow test result artifact".into()));
        }

        self.transition_stage(
            workflow_run_id,
            WorkflowStage::DockerTestExecution,
            WorkflowStage::QaValidation,
            TransitionTrigger::Automatic,
            summary.to_string(),
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

use super::WorkflowService;
use crate::domain::agent::AgentRunStatus;
use crate::domain::work_item::WorkItemStatus;
use crate::domain::workflow::{TransitionTrigger, WorkflowRun, WorkflowStage};
use crate::error::AppError;
use crate::persistence::{agent_repo, work_item_repo, workflow_repo};
use crate::services::workflow_failure;
use std::str::FromStr;
use tracing::{info, warn};

impl WorkflowService {
    /// Start a workflow for a work item.
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
                workflow_failure::mark_workflow_failed(
                    self.db.as_ref(),
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
}

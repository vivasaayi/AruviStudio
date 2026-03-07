// Workflow engine - manages the 20-state workflow state machine
// Handles state transitions, guard conditions, and rework loops
use crate::domain::workflow::WorkflowStage;

pub struct WorkflowEngine;

impl WorkflowEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn next_stage(&self, current: &WorkflowStage) -> Option<WorkflowStage> {
        match current {
            WorkflowStage::Draft => Some(WorkflowStage::PendingTaskApproval),
            WorkflowStage::PendingTaskApproval => Some(WorkflowStage::RequirementAnalysis),
            WorkflowStage::CoordinatorReview => Some(WorkflowStage::RequirementAnalysis),
            WorkflowStage::RequirementAnalysis => Some(WorkflowStage::Planning),
            WorkflowStage::Planning => Some(WorkflowStage::PendingPlanApproval),
            WorkflowStage::PendingPlanApproval => Some(WorkflowStage::Coding),
            WorkflowStage::Coding => Some(WorkflowStage::UnitTestGeneration),
            WorkflowStage::UnitTestGeneration => Some(WorkflowStage::IntegrationTestGeneration),
            WorkflowStage::IntegrationTestGeneration => Some(WorkflowStage::UiTestPlanning),
            WorkflowStage::UiTestPlanning => Some(WorkflowStage::DockerTestExecution),
            WorkflowStage::DockerTestExecution => Some(WorkflowStage::QaValidation),
            WorkflowStage::QaValidation => Some(WorkflowStage::SecurityReview),
            WorkflowStage::SecurityReview => Some(WorkflowStage::PerformanceReview),
            WorkflowStage::PerformanceReview => Some(WorkflowStage::PendingTestReview),
            WorkflowStage::PendingTestReview => Some(WorkflowStage::PushPreparation),
            WorkflowStage::PushPreparation => Some(WorkflowStage::GitPush),
            WorkflowStage::GitPush => Some(WorkflowStage::Done),
            WorkflowStage::Done
            | WorkflowStage::Failed
            | WorkflowStage::Cancelled
            | WorkflowStage::Blocked => None,
        }
    }
}

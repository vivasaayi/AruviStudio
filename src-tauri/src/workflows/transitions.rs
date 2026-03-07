// Workflow transition guards and validation rules

use crate::domain::workflow::WorkflowStage;

pub fn is_valid_transition(from: &WorkflowStage, to: &WorkflowStage) -> bool {
    match (from, to) {
        // Initial transitions
        (WorkflowStage::Draft, WorkflowStage::PendingTaskApproval) => true,
        (WorkflowStage::Draft, WorkflowStage::RequirementAnalysis) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::RequirementAnalysis) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::Planning) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::Coding) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::UnitTestGeneration) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::IntegrationTestGeneration) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::UiTestPlanning) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::QaValidation) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::SecurityReview) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::PerformanceReview) => true,
        (WorkflowStage::CoordinatorReview, WorkflowStage::PushPreparation) => true,

        // Approval flows
        (WorkflowStage::PendingTaskApproval, WorkflowStage::RequirementAnalysis) => true,
        (WorkflowStage::PendingTaskApproval, WorkflowStage::Cancelled) => true,
        (WorkflowStage::PendingPlanApproval, WorkflowStage::Coding) => true,
        (WorkflowStage::PendingPlanApproval, WorkflowStage::RequirementAnalysis) => true, // Rework
        (WorkflowStage::PendingTestReview, WorkflowStage::PushPreparation) => true,
        (WorkflowStage::PendingTestReview, WorkflowStage::Coding) => true, // Rework

        // Main workflow progression
        (WorkflowStage::RequirementAnalysis, WorkflowStage::Planning) => true,
        (WorkflowStage::RequirementAnalysis, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::Planning, WorkflowStage::PendingPlanApproval) => true,
        (WorkflowStage::Planning, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::Coding, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::Coding, WorkflowStage::UnitTestGeneration) => true,
        (WorkflowStage::UnitTestGeneration, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::UnitTestGeneration, WorkflowStage::IntegrationTestGeneration) => true,
        (WorkflowStage::IntegrationTestGeneration, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::IntegrationTestGeneration, WorkflowStage::UiTestPlanning) => true,
        (WorkflowStage::UiTestPlanning, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::UiTestPlanning, WorkflowStage::DockerTestExecution) => true,
        (WorkflowStage::DockerTestExecution, WorkflowStage::QaValidation) => true,
        (WorkflowStage::QaValidation, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::QaValidation, WorkflowStage::SecurityReview) => true,
        (WorkflowStage::SecurityReview, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::SecurityReview, WorkflowStage::PerformanceReview) => true,
        (WorkflowStage::PerformanceReview, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::PerformanceReview, WorkflowStage::PendingTestReview) => true,
        (WorkflowStage::PushPreparation, WorkflowStage::CoordinatorReview) => true,
        (WorkflowStage::PushPreparation, WorkflowStage::GitPush) => true,
        (WorkflowStage::GitPush, WorkflowStage::Done) => true,

        // Rework loops - can go back to coding from various stages
        (WorkflowStage::DockerTestExecution, WorkflowStage::Coding) => true, // Test failure
        (WorkflowStage::QaValidation, WorkflowStage::Coding) => true,        // QA failure
        (WorkflowStage::SecurityReview, WorkflowStage::Coding) => true,      // Security issue
        (WorkflowStage::PerformanceReview, WorkflowStage::Coding) => true,   // Performance issue

        // Terminal states - no transitions out
        (WorkflowStage::Done, _) => false,
        (WorkflowStage::Failed, _) => false,
        (WorkflowStage::Cancelled, _) => false,
        (WorkflowStage::Blocked, _) => false,

        // Invalid transitions
        _ => false,
    }
}

pub fn can_transition_on_agent_completion(from: &WorkflowStage) -> bool {
    matches!(
        from,
        WorkflowStage::RequirementAnalysis
            | WorkflowStage::Planning
            | WorkflowStage::Coding
            | WorkflowStage::UnitTestGeneration
            | WorkflowStage::IntegrationTestGeneration
            | WorkflowStage::UiTestPlanning
            | WorkflowStage::QaValidation
            | WorkflowStage::SecurityReview
            | WorkflowStage::PerformanceReview
    )
}

pub fn can_transition_on_user_approval(from: &WorkflowStage) -> bool {
    matches!(
        from,
        WorkflowStage::PendingTaskApproval
            | WorkflowStage::PendingPlanApproval
            | WorkflowStage::PendingTestReview
    )
}

pub fn can_transition_on_test_results(from: &WorkflowStage) -> bool {
    matches!(from, WorkflowStage::DockerTestExecution)
}

pub fn can_transition_on_push_completion(from: &WorkflowStage) -> bool {
    matches!(from, WorkflowStage::GitPush)
}

pub fn is_rework_transition(from: &WorkflowStage, to: &WorkflowStage) -> bool {
    matches!(
        (from, to),
        (WorkflowStage::DockerTestExecution, WorkflowStage::Coding)
            | (WorkflowStage::QaValidation, WorkflowStage::Coding)
            | (WorkflowStage::SecurityReview, WorkflowStage::Coding)
            | (WorkflowStage::PerformanceReview, WorkflowStage::Coding)
            | (
                WorkflowStage::PendingPlanApproval,
                WorkflowStage::RequirementAnalysis
            )
            | (WorkflowStage::PendingTestReview, WorkflowStage::Coding)
    )
}

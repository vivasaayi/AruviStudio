use crate::domain::workflow::{TransitionTrigger, UserAction, WorkflowStage};
use crate::error::AppError;

pub(crate) struct WorkflowUserActionTransition {
    pub(crate) to_stage: WorkflowStage,
    pub(crate) trigger: TransitionTrigger,
    pub(crate) notes: String,
    pub(crate) execute_stage: Option<WorkflowStage>,
}

pub(crate) fn resolve_user_action_transition(
    current_stage: &WorkflowStage,
    action: &UserAction,
    notes: Option<String>,
) -> Result<WorkflowUserActionTransition, AppError> {
    let transition = match (current_stage, action) {
        (WorkflowStage::PendingTaskApproval, UserAction::Approve) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::RequirementAnalysis,
            trigger: TransitionTrigger::UserApproval,
            notes: notes.unwrap_or_else(|| "Work item approved".to_string()),
            execute_stage: Some(WorkflowStage::RequirementAnalysis),
        },
        (WorkflowStage::PendingTaskApproval, UserAction::Reject) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::Cancelled,
            trigger: TransitionTrigger::UserRejection,
            notes: notes.unwrap_or_else(|| "Work item rejected".to_string()),
            execute_stage: None,
        },
        (WorkflowStage::PendingPlanApproval, UserAction::Approve) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::Coding,
            trigger: TransitionTrigger::UserApproval,
            notes: notes.unwrap_or_else(|| "Plan approved".to_string()),
            execute_stage: Some(WorkflowStage::Coding),
        },
        (WorkflowStage::PendingPlanApproval, UserAction::Reject) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::RequirementAnalysis,
            trigger: TransitionTrigger::UserRejection,
            notes: notes.unwrap_or_else(|| "Plan rejected, restarting analysis".to_string()),
            execute_stage: Some(WorkflowStage::RequirementAnalysis),
        },
        (WorkflowStage::PendingTestReview, UserAction::Approve) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::PushPreparation,
            trigger: TransitionTrigger::UserApproval,
            notes: notes.unwrap_or_else(|| "Tests approved".to_string()),
            execute_stage: Some(WorkflowStage::PushPreparation),
        },
        (WorkflowStage::PendingTestReview, UserAction::Reject) => WorkflowUserActionTransition {
            to_stage: WorkflowStage::Coding,
            trigger: TransitionTrigger::UserRejection,
            notes: notes.unwrap_or_else(|| "Tests rejected, restarting coding".to_string()),
            execute_stage: Some(WorkflowStage::Coding),
        },
        _ => {
            return Err(AppError::Validation(format!(
                "Invalid action {} for stage {}",
                action.as_str(),
                current_stage.as_str()
            )));
        }
    };
    Ok(transition)
}

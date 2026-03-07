use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStage {
    Draft,
    PendingTaskApproval,
    CoordinatorReview,
    RequirementAnalysis,
    Planning,
    PendingPlanApproval,
    Coding,
    UnitTestGeneration,
    IntegrationTestGeneration,
    UiTestPlanning,
    DockerTestExecution,
    QaValidation,
    SecurityReview,
    PerformanceReview,
    PendingTestReview,
    PushPreparation,
    GitPush,
    Done,
    Blocked,
    Failed,
    Cancelled,
}

impl WorkflowStage {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Draft => "draft",
            Self::PendingTaskApproval => "pending_task_approval",
            Self::CoordinatorReview => "coordinator_review",
            Self::RequirementAnalysis => "requirement_analysis",
            Self::Planning => "planning",
            Self::PendingPlanApproval => "pending_plan_approval",
            Self::Coding => "coding",
            Self::UnitTestGeneration => "unit_test_generation",
            Self::IntegrationTestGeneration => "integration_test_generation",
            Self::UiTestPlanning => "ui_test_planning",
            Self::DockerTestExecution => "docker_test_execution",
            Self::QaValidation => "qa_validation",
            Self::SecurityReview => "security_review",
            Self::PerformanceReview => "performance_review",
            Self::PendingTestReview => "pending_test_review",
            Self::PushPreparation => "push_preparation",
            Self::GitPush => "git_push",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    pub fn is_approval_gate(&self) -> bool {
        matches!(
            self,
            Self::PendingTaskApproval | Self::PendingPlanApproval | Self::PendingTestReview
        )
    }

    pub fn requires_agent(&self) -> bool {
        matches!(
            self,
            Self::RequirementAnalysis
                | Self::Planning
                | Self::Coding
                | Self::UnitTestGeneration
                | Self::IntegrationTestGeneration
                | Self::UiTestPlanning
                | Self::QaValidation
                | Self::SecurityReview
                | Self::PerformanceReview
        )
    }

    pub fn requires_coordinator_review(&self) -> bool {
        matches!(
            self,
            Self::RequirementAnalysis
                | Self::Planning
                | Self::Coding
                | Self::UnitTestGeneration
                | Self::IntegrationTestGeneration
                | Self::UiTestPlanning
                | Self::QaValidation
                | Self::SecurityReview
                | Self::PerformanceReview
                | Self::PushPreparation
        )
    }
}

impl std::fmt::Display for WorkflowStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for WorkflowStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "pending_task_approval" => Ok(Self::PendingTaskApproval),
            "coordinator_review" => Ok(Self::CoordinatorReview),
            "requirement_analysis" => Ok(Self::RequirementAnalysis),
            "planning" => Ok(Self::Planning),
            "pending_plan_approval" => Ok(Self::PendingPlanApproval),
            "coding" => Ok(Self::Coding),
            "unit_test_generation" => Ok(Self::UnitTestGeneration),
            "integration_test_generation" => Ok(Self::IntegrationTestGeneration),
            "ui_test_planning" => Ok(Self::UiTestPlanning),
            "docker_test_execution" => Ok(Self::DockerTestExecution),
            "qa_validation" => Ok(Self::QaValidation),
            "security_review" => Ok(Self::SecurityReview),
            "performance_review" => Ok(Self::PerformanceReview),
            "pending_test_review" => Ok(Self::PendingTestReview),
            "push_preparation" => Ok(Self::PushPreparation),
            "git_push" => Ok(Self::GitPush),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Unknown workflow stage: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkflowRun {
    pub id: String,
    pub work_item_id: String,
    pub workflow_version: String,
    pub status: String,
    pub current_stage: String,
    pub assigned_team_id: Option<String>,
    pub coordinator_agent_id: Option<String>,
    pub pending_stage_name: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkflowStageHistory {
    pub id: String,
    pub workflow_run_id: String,
    pub from_stage: String,
    pub to_stage: String,
    pub trigger: String,
    pub notes: String,
    pub transitioned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTrigger {
    Automatic,
    UserApproval,
    UserRejection,
    AgentCompletion,
    Rework,
    ManualOverride,
}

impl TransitionTrigger {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Automatic => "automatic",
            Self::UserApproval => "user_approval",
            Self::UserRejection => "user_rejection",
            Self::AgentCompletion => "agent_completion",
            Self::Rework => "rework",
            Self::ManualOverride => "manual_override",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserAction {
    Approve,
    Reject,
    Pause,
    Resume,
    Cancel,
}

impl UserAction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Cancel => "cancel",
        }
    }
}

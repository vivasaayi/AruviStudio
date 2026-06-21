use crate::error::AppError;
use crate::persistence::{approval_repo, settings_repo};
use sqlx::SqlitePool;

pub(crate) const AUTO_APPROVE_PLAN_KEY: &str = "workflow.auto_approve_plan";
pub(crate) const AUTO_APPROVE_TEST_REVIEW_KEY: &str = "workflow.auto_approve_test_review";

pub(crate) async fn is_auto_plan_approval_enabled(db: &SqlitePool) -> Result<bool, AppError> {
    settings_repo::get_bool_setting(db, AUTO_APPROVE_PLAN_KEY, true).await
}

pub(crate) async fn is_auto_test_review_enabled(db: &SqlitePool) -> Result<bool, AppError> {
    settings_repo::get_bool_setting(db, AUTO_APPROVE_TEST_REVIEW_KEY, true).await
}

pub(crate) async fn record_plan_approval(
    db: &SqlitePool,
    workflow_run_id: &str,
    work_item_id: &str,
    status: &str,
    notes: &str,
) -> Result<(), AppError> {
    record_workflow_approval(
        db,
        workflow_run_id,
        work_item_id,
        "plan_approval",
        status,
        notes,
    )
    .await
}

pub(crate) async fn record_test_review_approval(
    db: &SqlitePool,
    workflow_run_id: &str,
    work_item_id: &str,
    status: &str,
    notes: &str,
) -> Result<(), AppError> {
    record_workflow_approval(
        db,
        workflow_run_id,
        work_item_id,
        "test_review",
        status,
        notes,
    )
    .await
}

async fn record_workflow_approval(
    db: &SqlitePool,
    workflow_run_id: &str,
    work_item_id: &str,
    approval_type: &str,
    status: &str,
    notes: &str,
) -> Result<(), AppError> {
    let approval_id = uuid::Uuid::new_v4().to_string();
    approval_repo::create_approval(
        db,
        &approval_id,
        work_item_id,
        Some(workflow_run_id),
        approval_type,
        status,
        notes,
    )
    .await?;
    Ok(())
}

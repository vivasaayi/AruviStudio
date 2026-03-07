use crate::domain::workflow::{WorkflowRun, WorkflowStageHistory};
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_workflow_run(
    pool: &SqlitePool,
    id: &str,
    work_item_id: &str,
) -> Result<WorkflowRun, AppError> {
    sqlx::query_as::<_, WorkflowRun>("INSERT INTO workflow_runs (id,work_item_id,workflow_version,status,current_stage,retry_count,max_retries,started_at,updated_at) VALUES (?,?,?,'running','draft',?,?,'now','now') RETURNING id,work_item_id,workflow_version,status,current_stage,assigned_team_id,coordinator_agent_id,pending_stage_name,retry_count,max_retries,error_message,started_at,ended_at,updated_at")
        .bind(id).bind(work_item_id).bind("v1").bind(0).bind(3)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn get_workflow_run(pool: &SqlitePool, id: &str) -> Result<WorkflowRun, AppError> {
    sqlx::query_as::<_, WorkflowRun>("SELECT id,work_item_id,workflow_version,status,current_stage,assigned_team_id,coordinator_agent_id,pending_stage_name,retry_count,max_retries,error_message,started_at,ended_at,updated_at FROM workflow_runs WHERE id=?")
        .bind(id)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn get_latest_workflow_run_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Option<WorkflowRun>, AppError> {
    sqlx::query_as::<_, WorkflowRun>(
        "SELECT id,work_item_id,workflow_version,status,current_stage,assigned_team_id,coordinator_agent_id,pending_stage_name,retry_count,max_retries,error_message,started_at,ended_at,updated_at
         FROM workflow_runs
         WHERE work_item_id=?
         ORDER BY started_at DESC, updated_at DESC
         LIMIT 1",
    )
    .bind(work_item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn find_active_workflow_for_work_item(
    pool: &SqlitePool,
    work_item_id: &str,
) -> Result<Option<WorkflowRun>, AppError> {
    sqlx::query_as::<_, WorkflowRun>(
        "SELECT id,work_item_id,workflow_version,status,current_stage,assigned_team_id,coordinator_agent_id,pending_stage_name,retry_count,max_retries,error_message,started_at,ended_at,updated_at
         FROM workflow_runs
         WHERE work_item_id=?
           AND status='running'
           AND current_stage NOT IN ('done','failed','cancelled')
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(work_item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn close_orphaned_coordinator_reviews(pool: &SqlitePool) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE workflow_runs
         SET status='failed',
             error_message='Auto-closed orphaned coordinator review run',
             ended_at=datetime('now'),
             updated_at=datetime('now')
         WHERE status='running'
           AND current_stage='coordinator_review'
           AND (
               coordinator_agent_id IS NULL
               OR coordinator_agent_id NOT IN (
                   SELECT id
                   FROM agent_definitions
                   WHERE enabled=1
                     AND employment_status='active'
                     AND lower(role) IN ('manager','team_lead','coordinator')
               )
           )",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_workflow_stage(
    pool: &SqlitePool,
    id: &str,
    stage: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE workflow_runs SET current_stage=?,updated_at=datetime('now') WHERE id=?")
        .bind(stage)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_workflow_lifecycle(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    error_message: Option<&str>,
    mark_ended: bool,
) -> Result<(), AppError> {
    let ended_at = if mark_ended { Some("now") } else { None };
    sqlx::query(
        "UPDATE workflow_runs
         SET status=?,
             error_message=?,
             ended_at=CASE WHEN ? IS NOT NULL THEN datetime('now') ELSE ended_at END,
             updated_at=datetime('now')
         WHERE id=?",
    )
    .bind(status)
    .bind(error_message)
    .bind(ended_at)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_workflow_assignment(
    pool: &SqlitePool,
    id: &str,
    assigned_team_id: Option<&str>,
    coordinator_agent_id: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE workflow_runs SET assigned_team_id=?, coordinator_agent_id=?, updated_at=datetime('now') WHERE id=?")
        .bind(assigned_team_id)
        .bind(coordinator_agent_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_pending_stage_name(
    pool: &SqlitePool,
    id: &str,
    pending_stage_name: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE workflow_runs SET pending_stage_name=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(pending_stage_name)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_stage_transition(
    pool: &SqlitePool,
    id: &str,
    workflow_run_id: &str,
    from: &str,
    to: &str,
    trigger: &str,
    notes: &str,
) -> Result<WorkflowStageHistory, AppError> {
    sqlx::query_as::<_, WorkflowStageHistory>("INSERT INTO workflow_stage_history (id,workflow_run_id,from_stage,to_stage,trigger,notes,transitioned_at) VALUES (?,?,?,?,?,?,datetime('now')) RETURNING id,workflow_run_id,from_stage,to_stage,trigger,notes,transitioned_at")
        .bind(id).bind(workflow_run_id).bind(from).bind(to).bind(trigger).bind(notes)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn get_workflow_history(
    pool: &SqlitePool,
    workflow_run_id: &str,
) -> Result<Vec<WorkflowStageHistory>, AppError> {
    sqlx::query_as::<_, WorkflowStageHistory>("SELECT id,workflow_run_id,from_stage,to_stage,trigger,notes,transitioned_at FROM workflow_stage_history WHERE workflow_run_id=? ORDER BY transitioned_at ASC")
        .bind(workflow_run_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

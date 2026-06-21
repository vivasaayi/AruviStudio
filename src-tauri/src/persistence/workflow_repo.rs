use crate::domain::workflow::{WorkflowRun, WorkflowStageHistory};
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_workflow_run(
    pool: &SqlitePool,
    id: &str,
    work_item_id: &str,
) -> Result<WorkflowRun, AppError> {
    sqlx::query_as::<_, WorkflowRun>("INSERT INTO workflow_runs (id,work_item_id,workflow_version,status,current_stage,retry_count,max_retries,started_at,updated_at) VALUES (?,?,?,'running','draft',?,?,datetime('now'),datetime('now')) RETURNING id,work_item_id,workflow_version,status,current_stage,assigned_team_id,coordinator_agent_id,pending_stage_name,retry_count,max_retries,error_message,started_at,ended_at,updated_at")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{agent_repo, db as db_service, product_repo, work_item_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_workflow_repo_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ))
    }

    async fn create_test_pool(name: &str) -> SqlitePool {
        let temp_root = make_temp_dir(name);
        std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
        let db_path = temp_root.join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());
        db_service::create_pool(&database_url)
            .await
            .expect("test database should be created")
    }

    async fn create_test_work_item(pool: &SqlitePool, product_id: &str, work_item_id: &str) {
        product_repo::create_product(
            pool,
            product_repo::CreateProductInput {
                id: product_id,
                name: "Workflow Product",
                description: "",
                vision: "",
                goals: "[]",
                tags: "[]",
                lifecycle: Some("active"),
                health: Some("healthy"),
                owner_label: None,
                investment_status: Some("invest"),
                roadmap: None,
                evidence: None,
            },
        )
        .await
        .expect("product should be created");

        work_item_repo::create_work_item(
            pool,
            work_item_repo::CreateWorkItemInput {
                id: work_item_id,
                product_id,
                product_area_id: None,
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: "Workflow work item",
                problem_statement: "",
                description: "",
                acceptance_criteria: "",
                constraints: "",
                work_item_type: "story",
                priority: "medium",
                complexity: "medium",
            },
        )
        .await
        .expect("work item should be created");
    }

    async fn create_test_agent(pool: &SqlitePool, id: &str, name: &str, role: &str) {
        agent_repo::create_agent_definition(
            pool,
            agent_repo::CreateAgentDefinitionInput {
                id,
                name,
                role,
                description: "",
                prompt_template_ref: "",
                allowed_tools: "[]",
                skill_tags: "[]",
                boundaries: "{}",
                enabled: true,
                employment_status: "active",
            },
        )
        .await
        .expect("agent should be created");
    }

    #[tokio::test]
    async fn create_and_update_workflow_run_persists_lifecycle_assignment_and_pending_stage() {
        let pool = create_test_pool("workflow_lifecycle").await;
        create_test_work_item(&pool, "product-workflow-lifecycle", "work-item-lifecycle").await;
        create_test_agent(&pool, "coordinator-active", "Coordinator", "coordinator").await;

        let workflow = create_workflow_run(&pool, "workflow-lifecycle", "work-item-lifecycle")
            .await
            .expect("workflow run should be created");

        assert_eq!(workflow.status, "running");
        assert_eq!(workflow.current_stage, "draft");
        assert_eq!(workflow.retry_count, 0);
        assert_eq!(workflow.max_retries, 3);
        assert!(workflow.ended_at.is_none());

        update_workflow_stage(&pool, "workflow-lifecycle", "planning")
            .await
            .expect("stage should update");
        set_workflow_assignment(
            &pool,
            "workflow-lifecycle",
            None,
            Some("coordinator-active"),
        )
        .await
        .expect("assignment should update");
        set_pending_stage_name(&pool, "workflow-lifecycle", Some("coding"))
            .await
            .expect("pending stage should update");
        update_workflow_lifecycle(
            &pool,
            "workflow-lifecycle",
            "failed",
            Some("planner failed"),
            true,
        )
        .await
        .expect("lifecycle should update");

        let updated = get_workflow_run(&pool, "workflow-lifecycle")
            .await
            .expect("workflow run should load");

        assert_eq!(updated.current_stage, "planning");
        assert_eq!(
            updated.coordinator_agent_id.as_deref(),
            Some("coordinator-active")
        );
        assert_eq!(updated.pending_stage_name.as_deref(), Some("coding"));
        assert_eq!(updated.status, "failed");
        assert_eq!(updated.error_message.as_deref(), Some("planner failed"));
        assert!(updated.ended_at.is_some());
    }

    #[tokio::test]
    async fn find_active_workflow_returns_latest_non_terminal_running_workflow() {
        let pool = create_test_pool("workflow_active").await;
        create_test_work_item(&pool, "product-workflow-active", "work-item-active").await;

        sqlx::query(
            "INSERT INTO workflow_runs (
                id, work_item_id, workflow_version, status, current_stage, retry_count, max_retries,
                started_at, updated_at
             ) VALUES (
                ?, ?, 'v1', 'running', 'done', 0, 3, datetime('now', '-10 minutes'), datetime('now', '-10 minutes')
             )",
        )
        .bind("workflow-terminal-stage")
        .bind("work-item-active")
        .execute(&pool)
        .await
        .expect("terminal-stage workflow should be inserted");
        sqlx::query(
            "INSERT INTO workflow_runs (
                id, work_item_id, workflow_version, status, current_stage, retry_count, max_retries,
                started_at, updated_at
             ) VALUES (
                ?, ?, 'v1', 'paused', 'coding', 0, 3, datetime('now', '-5 minutes'), datetime('now', '-5 minutes')
             )",
        )
        .bind("workflow-paused")
        .bind("work-item-active")
        .execute(&pool)
        .await
        .expect("paused workflow should be inserted");
        sqlx::query(
            "INSERT INTO workflow_runs (
                id, work_item_id, workflow_version, status, current_stage, retry_count, max_retries,
                started_at, updated_at
             ) VALUES (
                ?, ?, 'v1', 'running', 'planning', 0, 3, datetime('now', '-1 minutes'), datetime('now', '-1 minutes')
             )",
        )
        .bind("workflow-active-latest")
        .bind("work-item-active")
        .execute(&pool)
        .await
        .expect("active workflow should be inserted");

        let active = find_active_workflow_for_work_item(&pool, "work-item-active")
            .await
            .expect("active workflow lookup should succeed")
            .expect("an active workflow should be returned");
        let latest = get_latest_workflow_run_for_work_item(&pool, "work-item-active")
            .await
            .expect("latest workflow lookup should succeed")
            .expect("a latest workflow should be returned");

        assert_eq!(active.id, "workflow-active-latest");
        assert_eq!(active.current_stage, "planning");
        assert_eq!(latest.id, "workflow-active-latest");
    }

    #[tokio::test]
    async fn close_orphaned_coordinator_reviews_only_closes_invalid_running_reviews() {
        let pool = create_test_pool("workflow_orphans").await;
        create_test_work_item(&pool, "product-workflow-orphans", "work-item-orphans").await;
        create_test_agent(
            &pool,
            "coordinator-valid",
            "Valid Coordinator",
            "coordinator",
        )
        .await;
        create_test_agent(&pool, "developer-invalid", "Developer", "developer").await;

        for (id, coordinator_agent_id, status, stage) in [
            (
                "workflow-orphan-null",
                None,
                "running",
                "coordinator_review",
            ),
            (
                "workflow-orphan-role",
                Some("developer-invalid"),
                "running",
                "coordinator_review",
            ),
            (
                "workflow-valid-review",
                Some("coordinator-valid"),
                "running",
                "coordinator_review",
            ),
            (
                "workflow-nonreview",
                Some("developer-invalid"),
                "running",
                "planning",
            ),
            (
                "workflow-paused-review",
                None,
                "paused",
                "coordinator_review",
            ),
        ] {
            sqlx::query(
                "INSERT INTO workflow_runs (
                    id, work_item_id, workflow_version, status, current_stage, coordinator_agent_id,
                    retry_count, max_retries, started_at, updated_at
                 ) VALUES (
                    ?, ?, 'v1', ?, ?, ?, 0, 3, datetime('now'), datetime('now')
                 )",
            )
            .bind(id)
            .bind("work-item-orphans")
            .bind(status)
            .bind(stage)
            .bind(coordinator_agent_id)
            .execute(&pool)
            .await
            .expect("workflow run should be inserted");
        }

        let affected = close_orphaned_coordinator_reviews(&pool)
            .await
            .expect("orphan cleanup should succeed");

        assert_eq!(affected, 2);

        let null_review = get_workflow_run(&pool, "workflow-orphan-null")
            .await
            .expect("null-review workflow should load");
        let invalid_role_review = get_workflow_run(&pool, "workflow-orphan-role")
            .await
            .expect("invalid-role workflow should load");
        let valid_review = get_workflow_run(&pool, "workflow-valid-review")
            .await
            .expect("valid review workflow should load");
        let non_review = get_workflow_run(&pool, "workflow-nonreview")
            .await
            .expect("non-review workflow should load");
        let paused_review = get_workflow_run(&pool, "workflow-paused-review")
            .await
            .expect("paused review workflow should load");

        assert_eq!(null_review.status, "failed");
        assert_eq!(
            null_review.error_message.as_deref(),
            Some("Auto-closed orphaned coordinator review run")
        );
        assert!(null_review.ended_at.is_some());
        assert_eq!(invalid_role_review.status, "failed");
        assert_eq!(valid_review.status, "running");
        assert_eq!(non_review.status, "running");
        assert_eq!(paused_review.status, "paused");
    }
}

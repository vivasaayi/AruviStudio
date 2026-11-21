use crate::domain::workflow::WorkflowStage;
use crate::error::AppError;
use crate::persistence::workflow_repo;
use crate::services::{workflow_delivery_lock, workflow_failure, workflow_git_push};
use crate::workflows::engine::WorkflowEngine;
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::{debug, info};

pub(crate) async fn advance_workflow(
    db: &SqlitePool,
    engine: &WorkflowEngine,
    workflow_run_id: &str,
) -> Result<(), AppError> {
    debug!(workflow_run_id = %workflow_run_id, "Advancing workflow");

    let _delivery_guard = workflow_delivery_lock::acquire(db, workflow_run_id).await?;
    let workflow_run = workflow_repo::get_workflow_run(db, workflow_run_id).await?;
    let current_stage = WorkflowStage::from_str(&workflow_run.current_stage)
        .map_err(|e| AppError::Validation(format!("Invalid workflow stage: {}", e)))?;
    debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Retrieved current workflow stage");

    if current_stage.is_terminal() {
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "Workflow is in terminal state, cannot advance");
        return Ok(());
    }
    if workflow_run.status != "running" {
        return Err(AppError::Validation("Workflow is no longer running; refresh before continuing".into()));
    }

    let Some(next_stage) = engine.next_stage(&current_stage) else {
        debug!(workflow_run_id = %workflow_run_id, current_stage = %current_stage.as_str(), "No next stage available, workflow is complete");
        return Ok(());
    };
    debug!(workflow_run_id = %workflow_run_id, next_stage = %next_stage.as_str(), "Determined next workflow stage");

    if next_stage == WorkflowStage::GitPush || current_stage == WorkflowStage::GitPush {
        if next_stage == WorkflowStage::GitPush {
            workflow_repo::commit_workflow_transition(db, workflow_run_id, current_stage.as_str(),
                WorkflowStage::GitPush.as_str(), "running", None, false, "automatic", "Workflow advancement").await?;
        }
        if let Err(error) = workflow_git_push::execute_git_push_stage(db, workflow_run_id).await {
            if let Err(mark_error) = workflow_failure::mark_workflow_failed(db, workflow_run_id, &WorkflowStage::GitPush, &error).await {
                tracing::error!(workflow_run_id, error = %mark_error, "Failed to record Git delivery failure");
            }
            return Err(error);
        }
        workflow_repo::commit_workflow_transition(db, workflow_run_id, WorkflowStage::GitPush.as_str(),
            WorkflowStage::Done.as_str(), "completed", None, true, "automatic", "Git push completed").await?;
        return Ok(());
    }

    let completed = next_stage == WorkflowStage::Done;
    workflow_repo::commit_workflow_transition(
        db,
        workflow_run_id,
        current_stage.as_str(),
        next_stage.as_str(),
        if completed { "completed" } else { "running" },
        None,
        completed,
        "automatic",
        "Workflow advancement",
    )
    .await?;
    info!(workflow_run_id = %workflow_run_id, from_stage = %current_stage.as_str(), to_stage = %next_stage.as_str(), "Successfully advanced workflow stage");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{db as db_service, product_repo, work_item_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_workflow_advance_{}_{}",
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
                name: "Workflow Advance Product",
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
                title: "Workflow advance work item",
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

    #[tokio::test]
    async fn advance_workflow_moves_non_terminal_stage_and_records_history() {
        let pool = create_test_pool("moves_stage").await;
        create_test_work_item(&pool, "product-advance", "work-item-advance").await;
        workflow_repo::create_workflow_run(&pool, "workflow-advance", "work-item-advance")
            .await
            .expect("workflow should be created");
        workflow_repo::update_workflow_stage(&pool, "workflow-advance", "planning")
            .await
            .expect("stage should update");

        advance_workflow(&pool, &WorkflowEngine::new(), "workflow-advance")
            .await
            .expect("workflow should advance");

        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-advance")
            .await
            .expect("workflow should load");
        assert_eq!(workflow.current_stage, "pending_plan_approval");

        let history = workflow_repo::get_workflow_history(&pool, "workflow-advance")
            .await
            .expect("history should load");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_stage, "planning");
        assert_eq!(history[0].to_stage, "pending_plan_approval");
        assert_eq!(history[0].trigger, "automatic");
        assert_eq!(history[0].notes, "Workflow advancement");
    }

    #[tokio::test]
    async fn advance_workflow_ignores_terminal_stage() {
        let pool = create_test_pool("terminal_stage").await;
        create_test_work_item(&pool, "product-terminal", "work-item-terminal").await;
        workflow_repo::create_workflow_run(&pool, "workflow-terminal", "work-item-terminal")
            .await
            .expect("workflow should be created");
        workflow_repo::update_workflow_stage(&pool, "workflow-terminal", "done")
            .await
            .expect("stage should update");

        advance_workflow(&pool, &WorkflowEngine::new(), "workflow-terminal")
            .await
            .expect("terminal workflow should be ignored");

        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-terminal")
            .await
            .expect("workflow should load");
        assert_eq!(workflow.current_stage, "done");

        let history = workflow_repo::get_workflow_history(&pool, "workflow-terminal")
            .await
            .expect("history should load");
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn advance_from_git_push_does_not_complete_without_delivery() {
        let pool = create_test_pool("complete_lifecycle").await;
        create_test_work_item(&pool, "product-complete", "work-item-complete").await;
        workflow_repo::create_workflow_run(&pool, "workflow-complete", "work-item-complete").await.unwrap();
        workflow_repo::update_workflow_stage(&pool, "workflow-complete", "git_push").await.unwrap();
        assert!(advance_workflow(&pool, &WorkflowEngine::new(), "workflow-complete").await.is_err());
        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-complete").await.unwrap();
        assert_eq!(workflow.current_stage, "failed");
        assert_eq!(workflow.status, "failed");
        assert!(workflow.ended_at.is_some());
        assert_eq!(workflow_repo::get_workflow_history(&pool, "workflow-complete").await.unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_advances_deliver_one_commit_and_complete_once() {
        use crate::services::workflow_delivery_manifest::{self, DeliveryManifest};
        use std::{collections::BTreeMap, path::Path, sync::Arc};

        let root = make_temp_dir("git_delivery");
        let work = root.join("work");
        let bare = root.join("origin.git");
        std::fs::create_dir_all(&work).unwrap();
        git2::Repository::init_bare(&bare).unwrap();
        let base = {
            let repo = git2::Repository::init(&work).unwrap();
            repo.set_head("refs/heads/main").unwrap();
            repo.remote("origin", bare.to_str().unwrap()).unwrap();
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Fixture").unwrap();
            config.set_str("user.email", "fixture@example.invalid").unwrap();
            std::fs::write(work.join("base.txt"), "base\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("base.txt")).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let signature = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap().to_string()
        };
        std::fs::write(work.join("generated.txt"), "workflow output\n").unwrap();
        let hash = workflow_delivery_manifest::file_digest(&work, Path::new("generated.txt")).unwrap();
        let pool = create_test_pool("concurrent_delivery").await;
        create_test_work_item(&pool, "product-delivery", "item-delivery").await;
        crate::persistence::repository_repo::create_repository(&pool, "repo-delivery", "Fixture", work.to_str().unwrap(), "", "main").await.unwrap();
        sqlx::query("UPDATE work_items SET active_repo_id=?, branch_name=? WHERE id=?")
            .bind("repo-delivery").bind("main").bind("item-delivery").execute(&pool).await.unwrap();
        workflow_repo::create_workflow_run(&pool, "run-delivery", "item-delivery").await.unwrap();
        workflow_repo::update_workflow_stage(&pool, "run-delivery", "push_preparation").await.unwrap();
        workflow_delivery_manifest::save(&pool, "run-delivery", &DeliveryManifest {
            repository_id: "repo-delivery".into(), base_head: Some(base.clone()),
            files: BTreeMap::from([("generated.txt".into(), hash)]), commit: None,
        }).await.unwrap();
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let mut tasks = Vec::new();
        for _ in 0..2 {
            let pool = pool.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                advance_workflow(&pool, &WorkflowEngine::new(), "run-delivery").await
            }));
        }
        for task in tasks { task.await.unwrap().unwrap(); }
        let run = workflow_repo::get_workflow_run(&pool, "run-delivery").await.unwrap();
        assert_eq!((run.current_stage.as_str(), run.status.as_str()), ("done", "completed"));
        let history = workflow_repo::get_workflow_history(&pool, "run-delivery").await.unwrap();
        assert_eq!(history.len(), 2);
        let commit = workflow_delivery_manifest::load(&pool, "run-delivery").await.unwrap().unwrap().commit.unwrap();
        let remote = git2::Repository::open_bare(&bare).unwrap();
        let delivered = remote.find_commit(git2::Oid::from_str(&commit).unwrap()).unwrap();
        assert_eq!(remote.find_reference("refs/heads/main").unwrap().target().unwrap().to_string(), commit);
        assert_eq!(delivered.parent_id(0).unwrap().to_string(), base);
        assert!(delivered.tree().unwrap().get_name("generated.txt").is_some());
        drop(delivered);
        drop(remote);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn delivery_lock_child_process() {
        let Ok(url) = std::env::var("ARUVI_DELIVERY_CHILD_DB") else { return; };
        let marker = std::path::PathBuf::from(std::env::var("ARUVI_DELIVERY_CHILD_MARKER").unwrap());
        let pool = db_service::create_pool(&url).await.unwrap();
        std::fs::write(marker.with_extension("ready"), "ready").unwrap();
        let _guard = workflow_delivery_lock::acquire(&pool, "run-lock").await.unwrap();
        std::fs::write(marker, "acquired").unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn delivery_lock_coordinates_separate_processes() {
        use std::time::{Duration, Instant};
        let pool = create_test_pool("delivery_process_lock").await;
        create_test_work_item(&pool, "product-lock", "item-lock").await;
        workflow_repo::create_workflow_run(&pool, "run-lock", "item-lock").await.unwrap();
        let held = workflow_delivery_lock::acquire(&pool, "run-lock").await.unwrap();
        let dbs: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list").fetch_all(&pool).await.unwrap();
        let path = &dbs.iter().find(|(_, name, _)| name == "main").unwrap().2;
        let marker = std::path::Path::new(path).with_extension("delivery-acquired");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "services::workflow_advance::tests::delivery_lock_child_process", "--nocapture"])
            .env("ARUVI_DELIVERY_CHILD_DB", format!("sqlite:{path}"))
            .env("ARUVI_DELIVERY_CHILD_MARKER", &marker)
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !marker.with_extension("ready").exists() {
            if let Some(status) = child.try_wait().unwrap() { panic!("child exited before attempting lock: {status}"); }
            if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); panic!("child startup timed out"); }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!marker.exists(), "child acquired a held delivery lock");
        drop(held);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() { assert!(status.success(), "child lock test failed"); break; }
            if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); panic!("child lock test timed out"); }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(marker.exists());
    }

    #[tokio::test]
    async fn advance_rejects_non_running_workflow() {
        let pool = create_test_pool("non_running").await;
        create_test_work_item(&pool, "product-stopped", "work-item-stopped").await;
        workflow_repo::create_workflow_run(&pool, "workflow-stopped", "work-item-stopped").await.unwrap();
        workflow_repo::update_workflow_lifecycle(&pool, "workflow-stopped", "failed", Some("stopped"), true).await.unwrap();
        assert!(advance_workflow(&pool, &WorkflowEngine::new(), "workflow-stopped").await.is_err());
        let workflow = workflow_repo::get_workflow_run(&pool, "workflow-stopped").await.unwrap();
        assert_eq!((workflow.current_stage.as_str(), workflow.status.as_str()), ("draft", "failed"));
        assert!(workflow_repo::get_workflow_history(&pool, "workflow-stopped").await.unwrap().is_empty());
    }
}

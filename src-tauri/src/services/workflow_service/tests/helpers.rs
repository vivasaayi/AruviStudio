use super::super::WorkflowService;
use crate::persistence::{agent_repo, model_repo, product_repo, work_item_repo};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};
use tokio::time::{sleep, Duration, Instant};

fn workflow_test_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

pub(super) async fn acquire_workflow_test_lock() -> OwnedMutexGuard<()> {
    workflow_test_lock().lock_owned().await
}

pub(super) fn make_temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "aruvi_workflow_service_{}_{}",
        name,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

pub(super) fn test_work_item_input<'a>(
    id: &'a str,
    product_id: &'a str,
    product_area_id: Option<&'a str>,
    title: &'a str,
) -> work_item_repo::CreateWorkItemInput<'a> {
    work_item_repo::CreateWorkItemInput {
        id,
        product_id,
        product_area_id,
        capability_id: None,
        source_node_id: None,
        source_node_type: None,
        parent_work_item_id: None,
        title,
        problem_statement: "problem",
        description: "description",
        acceptance_criteria: "acceptance",
        constraints: "constraints",
        work_item_type: "story",
        priority: "medium",
        complexity: "medium",
    }
}

pub(super) async fn create_test_product(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
) -> crate::domain::product::Product {
    product_repo::create_product(
        pool,
        product_repo::CreateProductInput {
            id,
            name,
            description: "desc",
            vision: "vision",
            goals: "[]",
            tags: "[]",
            lifecycle: None,
            health: None,
            owner_label: None,
            investment_status: None,
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("failed to create product")
}

pub(super) async fn create_test_product_area(
    pool: &sqlx::SqlitePool,
    id: &str,
    product_id: &str,
    name: &str,
) -> crate::domain::product::ProductArea {
    product_repo::create_product_area(
        pool,
        product_repo::CreateProductAreaInput {
            id,
            product_id,
            name,
            description: "desc",
            purpose: "purpose",
            node_kind: None,
            explanation: "",
            examples: "",
            implementation_notes: "",
            test_guidance: "",
        },
    )
    .await
    .expect("failed to create product_area")
}

pub(super) fn create_empty_calculator_test_repo(
    repo_dir: &PathBuf,
    remote_bare_dir: &PathBuf,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(repo_dir)?;
    std::fs::create_dir_all(remote_bare_dir)?;
    std::fs::create_dir_all(repo_dir.join("tests"))?;

    std::fs::write(
        repo_dir.join("README.md"),
        "# Calculator Pressure Test\n\nThis repository is intentionally initialized empty for outcome-driven agent delivery.\n",
    )?;
    std::fs::write(
        repo_dir.join(".gitignore"),
        "node_product_areas/\ndist/\nbuild/\ncoverage/\n",
    )?;
    std::fs::write(repo_dir.join("tests/.gitkeep"), "")?;

    run_git_command(
        repo_dir,
        &["init", "-b", "main"],
        "initialize git repository",
    )?;
    run_git_command(
        repo_dir,
        &["config", "user.name", "Aruvi Pressure Runner"],
        "set git user.name",
    )?;
    run_git_command(
        repo_dir,
        &["config", "user.email", "aruvi-pressure@example.com"],
        "set git user.email",
    )?;

    run_git_command(
        remote_bare_dir,
        &["init", "--bare"],
        "initialize bare remote repository",
    )?;

    run_git_command(
        repo_dir,
        &[
            "remote",
            "add",
            "origin",
            remote_bare_dir.to_string_lossy().as_ref(),
        ],
        "add origin remote",
    )?;
    run_git_command(repo_dir, &["add", "."], "stage bootstrap files")?;
    run_git_command(
        repo_dir,
        &[
            "commit",
            "-m",
            "chore: bootstrap empty calculator pressure repo",
        ],
        "create bootstrap commit",
    )?;
    run_git_command(
        repo_dir,
        &["push", "-u", "origin", "main"],
        "push bootstrap commit",
    )?;
    Ok(())
}

fn run_git_command(cwd: &PathBuf, args: &[&str], context: &str) -> Result<(), std::io::Error> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(std::io::Error::other(format!(
        "git command failed while trying to {}: git {} | stdout: {} | stderr: {}",
        context,
        args.join(" "),
        stdout.trim(),
        stderr.trim()
    )))
}

pub(super) async fn configure_live_model_bindings(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let live_base_url = std::env::var("ARUVI_LIVE_BASE_URL")
        .map_err(|_| "ARUVI_LIVE_BASE_URL is required for live test".to_string())?;
    let live_model_name = std::env::var("ARUVI_LIVE_MODEL")
        .map_err(|_| "ARUVI_LIVE_MODEL is required for live test".to_string())?;
    let live_api_key = std::env::var("ARUVI_LIVE_API_KEY").ok();

    let provider_id = "live-provider";
    let model_id = "live-model";

    model_repo::create_provider(
        pool,
        provider_id,
        "Live Provider",
        "openai_compatible",
        &live_base_url,
        live_api_key.as_deref(),
    )
    .await
    .map_err(|error| format!("create_provider failed: {error}"))?;
    model_repo::create_model_definition(
        pool,
        model_id,
        provider_id,
        &live_model_name,
        Some(128000),
        None,
        None,
    )
    .await
    .map_err(|error| format!("create_model_definition failed: {error}"))?;

    let agents = agent_repo::list_agent_definitions(pool)
        .await
        .map_err(|error| format!("list_agent_definitions failed: {error}"))?;
    for agent in agents {
        agent_repo::create_agent_model_binding(
            pool,
            &uuid::Uuid::new_v4().to_string(),
            &agent.id,
            model_id,
            0,
        )
        .await
        .map_err(|error| {
            format!(
                "create_agent_model_binding failed for {}: {error}",
                agent.id
            )
        })?;
    }

    Ok(())
}

pub(super) async fn wait_for_stage(
    service: &WorkflowService,
    workflow_run_id: &str,
    target_stage: &str,
    timeout: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        let run = service
            .get_workflow_run(workflow_run_id)
            .await
            .map_err(|error| format!("get_workflow_run failed: {error}"))?;
        if run.current_stage == target_stage {
            return Ok(());
        }
        if ["failed", "cancelled", "done"].contains(&run.current_stage.as_str())
            && run.current_stage != target_stage
        {
            return Err(format!(
                "workflow reached terminal stage {} before {}",
                run.current_stage, target_stage
            ));
        }
        if start.elapsed() > timeout {
            return Err(format!(
                "timeout waiting for stage {} (last stage: {})",
                target_stage, run.current_stage
            ));
        }
        sleep(Duration::from_secs(2)).await;
    }
}

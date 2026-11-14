use super::AgentService;
use crate::persistence::{
    agent_repo, artifact_repo, db as db_service, model_repo, product_repo, repository_repo,
    work_item_repo, workflow_repo,
};
use crate::services::model_service::ModelService;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

fn make_temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "aruvi_agent_service_{name}_{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

#[tokio::test]
async fn agent_model_calls_revalidate_selected_model_and_provider() {
    use crate::domain::agent::{AgentRun, AgentRunStatus};
    let temp = tempfile::tempdir().unwrap();
    let pool = db_service::create_pool(&format!("sqlite:{}", temp.path().join("models.db").display())).await.unwrap();
    model_repo::create_provider(&pool, "run-provider", "Fixture", "openai_compatible", "http://example.invalid", None).await.unwrap();
    let model = model_repo::create_model_definition(&pool, "run-model", "run-provider", "fixture", Some(8192), None, None).await.unwrap();
    let db = Arc::new(pool.clone());
    let service = AgentService::new(Arc::clone(&db), Arc::new(ModelService::new(Arc::clone(&db))), temp.path().join("artifacts"), temp.path().join("workspaces"));
    let run = AgentRun { id: "run".into(), workflow_run_id: "model-revalidation-fixture".into(), agent_id: "agent".into(), stage: "coding".into(), status: AgentRunStatus::Running,
        prompt_snapshot_path: None, output_snapshot_path: None, token_count_input: None, token_count_output: None, duration_ms: None, error_message: None,
        started_at: "".into(), ended_at: None, created_at: "".into() };
    AgentService::set_test_model_outputs_for_workflow(&run.workflow_run_id, vec!["first".into(), "second".into()]);
    assert_eq!(service.execute_agent_run(&run, &model, "prompt", 10, false).await.unwrap().content, "first");
    sqlx::query("UPDATE model_definitions SET enabled=0 WHERE id='run-model'").execute(&pool).await.unwrap();
    assert!(matches!(service.execute_agent_run(&run, &model, "prompt", 10, false).await, Err(crate::error::AppError::Validation(message)) if message.contains("model is disabled")));
    sqlx::query("UPDATE model_definitions SET enabled=1 WHERE id='run-model'").execute(&pool).await.unwrap();
    sqlx::query("UPDATE model_providers SET enabled=0 WHERE id='run-provider'").execute(&pool).await.unwrap();
    assert!(matches!(service.execute_agent_run(&run, &model, "prompt", 10, false).await, Err(crate::error::AppError::Validation(message)) if message.contains("provider is disabled")));
    sqlx::query("UPDATE model_providers SET enabled=1 WHERE id='run-provider'").execute(&pool).await.unwrap();
    sqlx::query("UPDATE model_definitions SET name='changed' WHERE id='run-model'").execute(&pool).await.unwrap();
    assert!(matches!(service.execute_agent_run(&run, &model, "prompt", 10, false).await, Err(crate::error::AppError::Validation(message)) if message.contains("model changed")));
    sqlx::query("UPDATE model_definitions SET name='fixture', context_window=4096 WHERE id='run-model'").execute(&pool).await.unwrap();
    assert!(matches!(service.execute_agent_run(&run, &model, "prompt", 10, false).await, Err(crate::error::AppError::Validation(message)) if message.contains("model changed")));
    sqlx::query("UPDATE model_definitions SET context_window=8192 WHERE id='run-model'").execute(&pool).await.unwrap();
    assert_eq!(service.execute_agent_run(&run, &model, "prompt", 10, false).await.unwrap().content, "second");
}

#[tokio::test]
async fn coding_stage_creates_tool_trace_artifact() {
    let temp_root = make_temp_dir("coding_trace");
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool");

    let provider_id = "test-provider";
    model_repo::create_provider(
        &pool,
        provider_id,
        "Test Provider",
        "openai_compatible",
        "http://example.invalid",
        None,
    )
    .await
    .expect("failed to create model provider");

    let model_id = "test-model";
    model_repo::create_model_definition(
        &pool,
        model_id,
        provider_id,
        "test-model",
        Some(8192),
        None,
        None,
    )
    .await
    .expect("failed to create model definition");

    agent_repo::delete_agent_model_bindings_for_agent(&pool, "coding-agent")
        .await
        .expect("failed to clear coding agent model bindings");
    agent_repo::create_agent_model_binding(
        &pool,
        "test-coding-binding",
        "coding-agent",
        model_id,
        0,
    )
    .await
    .expect("failed to bind coding agent to test model");
    sqlx::query(
            "UPDATE agent_definitions
             SET boundaries='{\"max_tool_steps\":15,\"keep_workspace\":false,\"max_file_chars\":4000}',
                 enabled=1,
                 employment_status='active'
             WHERE id='coding-agent'",
        )
        .execute(&pool)
        .await
        .expect("failed to update coding agent boundaries");

    let product = product_repo::create_product(
        &pool,
        product_repo::CreateProductInput {
            id: "test-product",
            name: "Integration Product",
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
    .expect("failed to create product");
    let product_area = product_repo::create_product_area(
        &pool,
        product_repo::CreateProductAreaInput {
            id: "test-product_area",
            product_id: &product.id,
            name: "Core ProductArea",
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
    .expect("failed to create product_area");

    let repo_dir = temp_root.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
    std::fs::write(repo_dir.join("README.md"), "# repo\n")
        .expect("failed to seed local repository");
    let adjacent_line = "function existing(value) { return value + 123456789; }";

    let repository = repository_repo::create_repository(
        &pool,
        "test-repo",
        "Test Repo",
        &repo_dir.to_string_lossy(),
        "",
        "main",
    )
    .await
    .expect("failed to register repository");

    let work_item = work_item_repo::create_work_item(
        &pool,
        work_item_repo::CreateWorkItemInput {
            id: "test-work-item",
            product_id: &product.id,
            product_area_id: Some(&product_area.id),
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title: "Implement tool loop",
            problem_statement: "problem",
            description: "description",
            acceptance_criteria: "acceptance",
            constraints: "constraints",
            work_item_type: "story",
            priority: "medium",
            complexity: "medium",
        },
    )
    .await
    .expect("failed to create work item");

    sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
        .bind(&repository.id)
        .bind(&work_item.id)
        .execute(&pool)
        .await
        .expect("failed to set work item active repository");

    let workflow_run =
        workflow_repo::create_workflow_run(&pool, "test-workflow-run", &work_item.id)
            .await
            .expect("failed to create workflow run");
    AgentService::set_test_model_outputs_for_workflow(
        &workflow_run.id,
        vec![
            "```json\n{\"type\":\"tool_call\",\"tool\":\"repo.write_file\",\"arguments\":{\"path\":\"must-not-exist.txt\",\"content\":\"unsafe\"}}\n```\n```json\n{\"type\":\"final\",\"summary\":\"done\"}\n```\nFile: legacy-must-not-exist.txt\n```\nunsafe\n```".to_string(),
            json!({"type":"final", "summary":"Claimed changes without writing files"}).to_string(),
            json!({"type":"final", "summary":"invalid file payload", "result":{"path":"invalid.txt", "content":"MODEL_CANDIDATE_MARKER"}}).to_string(),
            json!({"type":"tool_call", "tool":"repo.list_tree", "arguments":{"path":"missing-directory"}}).to_string(),
            json!({
                "type": "tool_call",
                "tool": "repo.write_file",
                "reason": "create implementation artifact",
                "arguments": {
                    "path": "hello.txt",
                    "content": "hello from tool loop\n"
                }
            })
            .to_string(),
            json!({"type":"tool_call", "tool":"repo.replace_range", "arguments":{"path":"hello.txt","start_line":1,"end_line":1,"content":"hello from tool loop\nsecond line\n"}}).to_string(),
            json!({"type":"tool_call", "tool":"repo.replace_range", "arguments":{"path":"hello.txt","start_line":1,"end_line":1,"content":"hello from tool loop\nsecond line"}}).to_string(),
            json!({"type":"tool_call", "tool":"repo.write_file", "arguments":{"path":"adjacent.js","content":format!("{adjacent_line}\nold\n")}}).to_string(),
            json!({"type":"tool_call", "tool":"repo.replace_range", "arguments":{"path":"adjacent.js","start_line":2,"end_line":2,"content":format!("{adjacent_line}\nnew\n")}}).to_string(),
            json!({"type":"tool_call", "tool":"repo.replace_range", "arguments":{"path":"adjacent.js","start_line":2,"end_line":2,"content":format!("{adjacent_line}\nnew\n"),"allow_adjacent_duplicate":true}}).to_string(),
            json!({"type":"repo.write_file", "path":"flat-must-not-exist.txt", "content":"not executed"}).to_string(),
            json!({
                "type": "final",
                "summary": "Implemented coding changes",
                "result": "done"
            })
            .to_string(),
        ],
    );

    let db_arc = Arc::new(pool.clone());
    let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
    let artifact_dir = temp_root.join("artifacts");
    let workspace_dir = temp_root.join("workspaces");
    std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
    std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

    let service = AgentService::new(
        Arc::clone(&db_arc),
        Arc::clone(&model_service),
        artifact_dir.clone(),
        workspace_dir.clone(),
    );

    let agent_run = service
        .run_agent_for_stage(&workflow_run.id, "coding")
        .await
        .expect("coding stage run failed");
    assert_eq!(
        agent_run.status,
        crate::domain::agent::AgentRunStatus::Completed
    );

    let artifacts = artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
        .await
        .expect("failed to list artifacts");
    let trace_artifact = artifacts
        .iter()
        .find(|artifact| artifact.artifact_type == "coding_tool_trace")
        .expect("missing coding_tool_trace artifact");
    assert!(
        std::path::Path::new(&trace_artifact.storage_path).exists(),
        "tool trace file does not exist: {}",
        trace_artifact.storage_path
    );
    let trace_content =
        std::fs::read_to_string(&trace_artifact.storage_path).expect("failed to read trace file");
    assert!(trace_content.contains("Response contained multiple actions; no action was executed"));
    assert!(!repo_dir.join("must-not-exist.txt").exists());
    assert!(!repo_dir.join("legacy-must-not-exist.txt").exists());
    assert!(!repo_dir.join("invalid.txt").exists());
    assert!(!repo_dir.join("flat-must-not-exist.txt").exists());
    assert!(trace_content.contains("To retry this attempted action"));
    assert!(trace_content.contains("Previous invalid response (quoted data)"));
    assert!(trace_content.contains("MODEL_CANDIDATE_MARKER"));
    assert!(trace_content.contains("No files were changed"));
    assert!(trace_content.contains("replacement already succeeded"));
    assert!(trace_content.contains("Replacement repeats unchanged neighboring line"));
    assert_eq!(std::fs::read_to_string(repo_dir.join("adjacent.js")).unwrap(), format!("{adjacent_line}\n{adjacent_line}\nnew\n"));
    assert!(trace_content.contains("repo.write_file creates parent directories"));
    assert!(
        trace_content.contains("tool_result"),
        "trace artifact did not include tool_result entry"
    );
    assert!(
        trace_content.contains("repo.write_file"),
        "trace artifact did not include invoked tool name"
    );

    let applied_file = std::fs::read_to_string(repo_dir.join("hello.txt"))
        .expect("expected file written by coding tool loop");
    assert_eq!(applied_file, "hello from tool loop\nsecond line\n");

    let no_change_run = workflow_repo::create_workflow_run(&pool, "test-no-change-run", &work_item.id)
        .await
        .expect("create no-change workflow");
    AgentService::set_test_model_outputs_for_workflow(
        &no_change_run.id,
        vec![json!({"type":"final", "summary":"The requested behavior already exists", "no_changes_needed":true}).to_string()],
    );
    let no_change_agent = service.run_agent_for_stage(&no_change_run.id, "coding").await
        .expect("explicit no-change outcome should complete");
    assert_eq!(no_change_agent.status, crate::domain::agent::AgentRunStatus::Completed);

    // Disable the selected model after the provider receives the first call,
    // so the next tool-loop call must fail before contacting it again.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    sqlx::query("UPDATE model_providers SET base_url=? WHERE id=?")
        .bind(format!("http://{address}/v1")).bind(provider_id).execute(&pool).await.unwrap();
    let fixture_pool = pool.clone();
    let provider = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let header_end = loop {
            let count = socket.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
            if let Some(index) = request.windows(4).position(|part| part == b"\r\n\r\n") { break index + 4; }
        };
        let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
        let size: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
        while request.len() < header_end + size {
            let count = socket.read(&mut buffer).await.unwrap(); assert_ne!(count, 0); request.extend_from_slice(&buffer[..count]);
        }
        sqlx::query("UPDATE model_definitions SET enabled=0 WHERE id='test-model'").execute(&fixture_pool).await.unwrap();
        let content = json!({"type":"tool_call", "tool":"repo.write_file", "arguments":{"path":"suspended.txt", "content":"not delivered"}}).to_string();
        let body = json!({"choices":[{"message":{"content":content},"finish_reason":"stop"}]}).to_string();
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let interrupted = workflow_repo::create_workflow_run(&pool, "test-disabled-mid-run", &work_item.id).await.unwrap();
    let error = service.run_agent_for_stage(&interrupted.id, "coding").await.unwrap_err();
    assert!(error.to_string().contains("selected agent model is disabled"));
    provider.await.unwrap();
    assert!(!repo_dir.join("suspended.txt").exists(), "failed run must not sync its workspace to the repository");
    assert_eq!(std::fs::read_dir(&workspace_dir).unwrap().count(), 0, "keep_workspace=false should remove a failed pre-sync workspace");
    let runs = agent_repo::list_agent_runs_for_workflow(&pool, &interrupted.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, crate::domain::agent::AgentRunStatus::Failed);
    assert!(runs[0].error_message.as_deref().unwrap().contains("selected agent model is disabled"));
    let calls = crate::persistence::model_call_repo::list_model_calls_for_workflow(&pool, &interrupted.id).await.unwrap();
    assert_eq!(calls.len(), 1, "the disabled model must not be called a second time");
    sqlx::query("UPDATE model_definitions SET enabled=1 WHERE id='test-model'").execute(&pool).await.unwrap();
    let retried = workflow_repo::create_workflow_run(&pool, "test-after-reenable", &work_item.id).await.unwrap();
    AgentService::set_test_model_outputs_for_workflow(&retried.id, vec![json!({"type":"final", "summary":"No change required", "no_changes_needed":true}).to_string()]);
    assert_eq!(service.run_agent_for_stage(&retried.id, "coding").await.unwrap().status, crate::domain::agent::AgentRunStatus::Completed);

    // A sync failure can leave a partially updated repository. Preserve the
    // workspace even when normal retention is disabled so it can be inspected.
    let conflict = workflow_repo::create_workflow_run(&pool, "test-sync-conflict", &work_item.id).await.unwrap();
    AgentService::set_test_model_outputs_for_workflow(&conflict.id, vec![
        json!({"type":"tool_call", "tool":"repo.write_file", "arguments":{"path":"README.md", "content":"candidate recovery data"}}).to_string(),
        json!({"type":"final", "summary":"done"}).to_string(),
    ]);
    let error = service.run_agent_for_stage(&conflict.id, "coding").await.unwrap_err();
    assert!(error.to_string().contains("Cannot safely overwrite an existing file"));
    assert_eq!(std::fs::read_to_string(repo_dir.join("README.md")).unwrap(), "# repo\n");
    let workspaces: Vec<_> = std::fs::read_dir(&workspace_dir).unwrap().map(|entry| entry.unwrap().path()).collect();
    assert_eq!(workspaces.len(), 1, "a sync failure must retain the workspace for recovery");
    assert_eq!(std::fs::read_to_string(workspaces[0].join("repo/README.md")).unwrap(), "candidate recovery data");

    sqlx::query("UPDATE agent_definitions SET boundaries='{\"max_tool_steps\":2,\"keep_workspace\":true}' WHERE id='coding-agent'").execute(&pool).await.unwrap();
    sqlx::query("UPDATE model_providers SET base_url='http://127.0.0.1:1/v1' WHERE id='test-provider'").execute(&pool).await.unwrap();
    let retained = workflow_repo::create_workflow_run(&pool, "test-retain-failed-workspace", &work_item.id).await.unwrap();
    assert!(service.run_agent_for_stage(&retained.id, "coding").await.is_err());
    assert_eq!(std::fs::read_dir(&workspace_dir).unwrap().count(), 2, "keep_workspace=true must retain a failed workspace");

    // A preparation failure occurs after the run is created but before a model
    // call. It must be visible as a failed run rather than a perpetual one.
    let preparation = workflow_repo::create_workflow_run(&pool, "test-preparation-failure", &work_item.id).await.unwrap();
    sqlx::query("ALTER TABLE skills RENAME TO unavailable_skills").execute(&pool).await.unwrap();
    let error = service.run_agent_for_stage(&preparation.id, "coding").await.unwrap_err();
    sqlx::query("ALTER TABLE unavailable_skills RENAME TO skills").execute(&pool).await.unwrap();
    let runs = agent_repo::list_agent_runs_for_workflow(&pool, &preparation.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, crate::domain::agent::AgentRunStatus::Failed);
    assert_eq!(runs[0].error_message.as_deref(), Some(error.to_string().as_str()));
    assert!(runs[0].ended_at.is_some());
    assert!(crate::persistence::model_call_repo::list_model_calls_for_workflow(&pool, &preparation.id).await.unwrap().is_empty());

    // A persistence failure after model execution must likewise finish the run.
    sqlx::query("CREATE TRIGGER reject_test_prompt BEFORE INSERT ON artifacts WHEN NEW.artifact_type = 'coding_prompt' BEGIN SELECT RAISE(FAIL, 'prompt storage unavailable'); END")
        .execute(&pool).await.unwrap();
    let output_failure = workflow_repo::create_workflow_run(&pool, "test-output-failure", &work_item.id).await.unwrap();
    AgentService::set_test_model_outputs_for_workflow(&output_failure.id, vec![json!({"type":"final", "summary":"No change required", "no_changes_needed":true}).to_string()]);
    let error = service.run_agent_for_stage(&output_failure.id, "coding").await.unwrap_err();
    assert!(error.to_string().contains("prompt storage unavailable"));
    let runs = agent_repo::list_agent_runs_for_workflow(&pool, &output_failure.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, crate::domain::agent::AgentRunStatus::Failed);
    assert!(runs[0].error_message.as_deref().unwrap().contains("prompt storage unavailable"));
    assert!(runs[0].ended_at.is_some());

    let _ = std::fs::remove_dir_all(temp_root);
}

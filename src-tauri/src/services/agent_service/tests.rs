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
             SET boundaries='{\"max_tool_steps\":4,\"keep_workspace\":false,\"max_file_chars\":4000}',
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
    assert_eq!(applied_file, "hello from tool loop\n");

    let _ = std::fs::remove_dir_all(temp_root);
}

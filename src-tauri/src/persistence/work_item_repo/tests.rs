use super::*;
use crate::domain::product::HierarchyNodeType;
use crate::error::AppError;
use crate::persistence::{db as db_service, product_repo};
use sqlx::SqlitePool;

mod read_queries;

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "aruvi_work_item_repo_{}_{}",
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

async fn create_test_product(pool: &SqlitePool, product_id: &str) {
    product_repo::create_product(
        pool,
        product_repo::CreateProductInput {
            id: product_id,
            name: "Test Product",
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
}

fn test_work_item_input<'a>(
    id: &'a str,
    product_id: &'a str,
    title: &'a str,
) -> CreateWorkItemInput<'a> {
    CreateWorkItemInput {
        id,
        product_id,
        product_area_id: None,
        capability_id: None,
        source_node_id: None,
        source_node_type: None,
        parent_work_item_id: None,
        title,
        problem_statement: "",
        description: "",
        acceptance_criteria: "",
        constraints: "",
        work_item_type: "story",
        priority: "medium",
        complexity: "medium",
    }
}

fn status_patch<'a>(id: &'a str, status: &'a str) -> UpdateWorkItemPatch<'a> {
    UpdateWorkItemPatch {
        id,
        status: Some(status),
        title: None,
        description: None,
        problem_statement: None,
        acceptance_criteria: None,
        constraints: None,
    }
}

async fn create_test_product_area(pool: &SqlitePool, id: &str, product_id: &str, name: &str) {
    product_repo::create_product_area(
        pool,
        product_repo::CreateProductAreaInput {
            id,
            product_id,
            name,
            description: "",
            purpose: "",
            node_kind: Some("product_area"),
            explanation: "",
            examples: "",
            implementation_notes: "",
            test_guidance: "",
        },
    )
    .await
    .expect("product_area should be created");
}

async fn create_test_capability(
    pool: &SqlitePool,
    id: &str,
    product_area_id: &str,
    parent_capability_id: Option<&str>,
    name: &str,
    node_kind: Option<&str>,
) {
    product_repo::create_capability(
        pool,
        product_repo::CreateCapabilityInput {
            id,
            product_area_id,
            parent_capability_id,
            name,
            description: "",
            acceptance_criteria: "",
            priority: "medium",
            risk: "low",
            technical_notes: "",
            node_kind,
            explanation: "",
            examples: "",
            implementation_notes: "",
            test_guidance: "",
        },
    )
    .await
    .expect("capability should be created");
}

async fn explain_query_plan_details(
    pool: &SqlitePool,
    sql: &str,
    bindings: &[&str],
) -> Vec<String> {
    let mut query = sqlx::query_as::<_, (i64, i64, i64, String)>(sql);
    for binding in bindings {
        query = query.bind(*binding);
    }

    query
        .fetch_all(pool)
        .await
        .expect("query plan should load")
        .into_iter()
        .map(|(_, _, _, detail)| detail)
        .collect()
}

async fn assert_query_plan_uses_index(
    pool: &SqlitePool,
    sql: &str,
    bindings: &[&str],
    index_name: &str,
) {
    let details = explain_query_plan_details(pool, sql, bindings).await;
    assert!(
        details.iter().any(|detail| detail.contains(index_name)),
        "expected query plan to use {index_name}; details: {details:#?}"
    );
}

#[tokio::test]
async fn get_sub_work_items_page_returns_child_tasks_for_story() {
    let pool = create_test_pool("sub_work_items").await;
    create_test_product(&pool, "product-sub-work").await;

    create_work_item(
        &pool,
        CreateWorkItemInput {
            priority: "high",
            ..test_work_item_input(
                "story-with-two-tasks",
                "product-sub-work",
                "Story with two tasks",
            )
        },
    )
    .await
    .expect("story should be created");

    for (index, title) in ["First task", "Second task"].iter().enumerate() {
        let task_id = format!("story-child-task-{index}");
        let mut input = test_work_item_input(&task_id, "product-sub-work", title);
        input.parent_work_item_id = Some("story-with-two-tasks");
        input.work_item_type = "task";
        create_work_item(&pool, input)
            .await
            .expect("task should be created");
    }

    let tasks = get_sub_work_items_page(&pool, "story-with-two-tasks", Some(20), Some(0))
        .await
        .expect("child tasks should load");

    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].title, "First task");
    assert_eq!(tasks[1].title, "Second task");
    assert!(tasks
        .iter()
        .all(|task| task.parent_work_item_id.as_deref() == Some("story-with-two-tasks")));
}

#[tokio::test]
async fn create_work_item_resolves_capability_scope_and_story_type() {
    let pool = create_test_pool("normalize_scope").await;
    create_test_product(&pool, "product-normalize").await;
    create_test_product_area(
        &pool,
        "product_area-normalize",
        "product-normalize",
        "Operations",
    )
    .await;
    create_test_capability(
        &pool,
        "capability-normalize",
        "product_area-normalize",
        None,
        "Checkout",
        Some("capability"),
    )
    .await;

    let work_item = create_work_item(
        &pool,
        CreateWorkItemInput {
            capability_id: Some("capability-normalize"),
            ..test_work_item_input(
                "work-item-normalize",
                "product-normalize",
                "Normalized story",
            )
        },
    )
    .await
    .expect("work item should be created");

    assert_eq!(
        work_item.product_area_id.as_deref(),
        Some("product_area-normalize")
    );
    assert_eq!(
        work_item.capability_id.as_deref(),
        Some("capability-normalize")
    );
    assert_eq!(
        work_item.source_node_id.as_deref(),
        Some("capability-normalize")
    );
    assert!(matches!(
        work_item.source_node_type,
        Some(HierarchyNodeType::Capability)
    ));
    assert!(matches!(
        work_item.work_item_type,
        crate::domain::work_item::WorkItemType::Story
    ));
}

#[tokio::test]
async fn create_child_work_item_inherits_parent_source_scope() {
    let pool = create_test_pool("inherit_scope").await;
    create_test_product(&pool, "product-inherit").await;
    create_test_product_area(&pool, "product_area-inherit", "product-inherit", "Platform").await;
    create_test_capability(
        &pool,
        "capability-inherit",
        "product_area-inherit",
        None,
        "Runtime",
        Some("capability"),
    )
    .await;

    create_work_item(
        &pool,
        CreateWorkItemInput {
            capability_id: Some("capability-inherit"),
            priority: "high",
            ..test_work_item_input("parent-story", "product-inherit", "Parent story")
        },
    )
    .await
    .expect("parent work item should be created");

    let child = create_work_item(
        &pool,
        CreateWorkItemInput {
            parent_work_item_id: Some("parent-story"),
            work_item_type: "task",
            complexity: "low",
            ..test_work_item_input("child-task", "product-inherit", "Child task")
        },
    )
    .await
    .expect("child work item should be created");

    assert_eq!(child.parent_work_item_id.as_deref(), Some("parent-story"));
    assert_eq!(
        child.product_area_id.as_deref(),
        Some("product_area-inherit")
    );
    assert_eq!(child.capability_id.as_deref(), Some("capability-inherit"));
    assert_eq!(child.source_node_id.as_deref(), Some("capability-inherit"));
    assert!(matches!(
        child.source_node_type,
        Some(HierarchyNodeType::Capability)
    ));
}

#[tokio::test]
async fn create_work_item_rejects_source_node_id_without_type() {
    let pool = create_test_pool("source_type_validation").await;
    create_test_product(&pool, "product-source-validation").await;

    let error = create_work_item(
        &pool,
        CreateWorkItemInput {
            source_node_id: Some("product_area-missing-type"),
            ..test_work_item_input(
                "work-item-invalid-source",
                "product-source-validation",
                "Invalid source",
            )
        },
    )
    .await
    .expect_err("missing source node type should fail validation");

    assert!(
        matches!(error, AppError::Validation(message) if message == "source_node_type is required when source_node_id is provided.")
    );
}

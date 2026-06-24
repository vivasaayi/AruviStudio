use super::*;
use crate::domain::product::HierarchyNodeKind;
use crate::persistence::db as db_service;
use sqlx::SqlitePool;

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "aruvi_product_repo_{}_{}",
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

async fn create_test_product(pool: &SqlitePool, product_id: &str, name: &str) {
    create_product(
        pool,
        CreateProductInput {
            id: product_id,
            name,
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

#[tokio::test]
async fn get_product_tree_decodes_canonical_catalog_node_kinds() {
    let pool = create_test_pool("canonical_node_kinds").await;
    create_test_product(&pool, "product-canonical-kinds", "Legacy Kinds").await;

    sqlx::query(
            "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('product_area-payments', 'product-canonical-kinds', 'product_area', 'Payments', 0)",
        )
        .execute(&pool)
        .await
        .expect("product area should be inserted");
    sqlx::query(
            "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('product_area-billing', 'product-canonical-kinds', 'product_area', 'Billing', 1)",
        )
        .execute(&pool)
        .await
        .expect("product area should be inserted");
    sqlx::query(
            "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('product_area-operations', 'product-canonical-kinds', 'product_area', 'Operations', 2)",
        )
        .execute(&pool)
        .await
        .expect("product area should be inserted");
    sqlx::query(
        "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES (
                'capability-checkout', 'product_area-payments', NULL, 0, 'capability', 0, 'Checkout'
             )",
    )
    .execute(&pool)
    .await
    .expect("capability should be inserted");
    sqlx::query(
        "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES (
                'capability-runtime', 'product_area-operations', NULL, 0, 'capability', 0, 'Runtime'
             )",
    )
    .execute(&pool)
    .await
    .expect("capability should be inserted");
    sqlx::query(
            "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES (
                'feature-reconciliation', 'product_area-payments', 'capability-checkout', 1, 'feature', 0, 'Reconciliation'
             )",
        )
        .execute(&pool)
        .await
        .expect("feature should be inserted");

    let tree = get_product_tree(&pool, "product-canonical-kinds")
        .await
        .expect("canonical node kinds should decode");

    assert_eq!(tree.product_areas.len(), 3);
    assert_eq!(
        tree.product_areas[0].product_area.node_kind,
        HierarchyNodeKind::ProductArea
    );
    assert_eq!(
        tree.product_areas[1].product_area.node_kind,
        HierarchyNodeKind::ProductArea
    );
    assert_eq!(
        tree.product_areas[2].product_area.node_kind,
        HierarchyNodeKind::ProductArea
    );
    assert_eq!(tree.product_areas[0].features.len(), 1);
    assert_eq!(
        tree.product_areas[0].features[0].capability.node_kind,
        HierarchyNodeKind::Capability
    );
    assert_eq!(tree.product_areas[0].features[0].children.len(), 1);
    assert_eq!(
        tree.product_areas[0].features[0].children[0]
            .capability
            .node_kind,
        HierarchyNodeKind::Feature
    );
    assert_eq!(tree.product_areas[2].features.len(), 1);
    assert_eq!(
        tree.product_areas[2].features[0].capability.node_kind,
        HierarchyNodeKind::Capability
    );

    let summary = summarize_product_tree(&pool, "product-canonical-kinds")
        .await
        .expect("product tree summary should be counted without materializing the tree");
    assert_eq!(summary.product_area_count, 3);
    assert_eq!(summary.capability_count, 3);
    assert_eq!(summary.total_node_count, 6);
    assert_eq!(summary.leaf_node_count, 3);
}

#[tokio::test]
async fn list_capabilities_by_ids_for_product_returns_only_requested_product_capabilities() {
    let pool = create_test_pool("targeted_capabilities").await;
    create_test_product(&pool, "product-targeted", "Targeted Product").await;
    create_test_product(&pool, "product-other", "Other Product").await;

    sqlx::query(
        "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('area-targeted-a', 'product-targeted', 'product_area', 'A', 0)",
    )
    .execute(&pool)
    .await
    .expect("product area should be inserted");
    sqlx::query(
        "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('area-targeted-b', 'product-targeted', 'product_area', 'B', 1)",
    )
    .execute(&pool)
    .await
    .expect("product area should be inserted");
    sqlx::query(
        "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('area-other', 'product-other', 'product_area', 'Other', 0)",
    )
    .execute(&pool)
    .await
    .expect("product area should be inserted");
    for (id, area_id, sort_order, name) in [
        ("cap-targeted-a", "area-targeted-a", 0, "Targeted A"),
        ("cap-targeted-b", "area-targeted-b", 0, "Targeted B"),
        ("cap-unrequested", "area-targeted-a", 1, "Unrequested"),
        ("cap-other", "area-other", 0, "Other"),
    ] {
        sqlx::query(
            "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES (?, ?, NULL, 0, 'capability', ?, ?)",
        )
        .bind(id)
        .bind(area_id)
        .bind(sort_order)
        .bind(name)
        .execute(&pool)
        .await
        .expect("capability should be inserted");
    }

    let requested = list_capabilities_by_ids_for_product(
        &pool,
        "product-targeted",
        &[
            "cap-other".to_string(),
            "cap-unrequested".to_string(),
            "cap-targeted-b".to_string(),
            "cap-targeted-a".to_string(),
        ],
    )
    .await
    .expect("targeted capabilities should load");

    assert_eq!(
        requested
            .iter()
            .map(|capability| capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["cap-targeted-a", "cap-unrequested", "cap-targeted-b"]
    );
}

#[tokio::test]
async fn list_product_capabilities_returns_tree_ordered_capabilities_for_one_product() {
    let pool = create_test_pool("product_capabilities").await;
    create_test_product(&pool, "product-capabilities", "Capability Product").await;
    create_test_product(&pool, "product-other-capabilities", "Other Product").await;

    for (id, product_id, sort_order, name) in [
        ("area-primary", "product-capabilities", 0, "Primary"),
        ("area-secondary", "product-capabilities", 1, "Secondary"),
        (
            "area-other-capabilities",
            "product-other-capabilities",
            0,
            "Other",
        ),
    ] {
        sqlx::query(
            "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES (?, ?, 'product_area', ?, ?)",
        )
        .bind(id)
        .bind(product_id)
        .bind(name)
        .bind(sort_order)
        .execute(&pool)
        .await
        .expect("product area should be inserted");
    }

    for (id, area_id, parent_id, level, node_kind, sort_order, name) in [
        (
            "cap-primary-root",
            "area-primary",
            None,
            0,
            "capability",
            0,
            "Primary Root",
        ),
        (
            "feature-primary-child",
            "area-primary",
            Some("cap-primary-root"),
            1,
            "feature",
            0,
            "Primary Child",
        ),
        (
            "cap-primary-second",
            "area-primary",
            None,
            0,
            "capability",
            1,
            "Primary Second",
        ),
        (
            "cap-secondary-root",
            "area-secondary",
            None,
            0,
            "capability",
            0,
            "Secondary Root",
        ),
        (
            "cap-other-product",
            "area-other-capabilities",
            None,
            0,
            "capability",
            0,
            "Other Product Root",
        ),
    ] {
        sqlx::query(
            "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(area_id)
        .bind(parent_id)
        .bind(level)
        .bind(node_kind)
        .bind(sort_order)
        .bind(name)
        .execute(&pool)
        .await
        .expect("capability should be inserted");
    }

    let capabilities = list_product_capabilities(&pool, "product-capabilities")
        .await
        .expect("product capabilities should load");

    assert_eq!(
        capabilities
            .iter()
            .map(|capability| capability.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "cap-primary-root",
            "feature-primary-child",
            "cap-primary-second",
            "cap-secondary-root"
        ]
    );
    assert!(capabilities
        .iter()
        .all(|capability| capability.product_area_id != "area-other-capabilities"));
}

#[tokio::test]
async fn reset_product_plan_deletes_delivery_and_agent_work_when_requested() {
    let pool = create_test_pool("reset_product_plan").await;
    create_test_product(&pool, "product-reset", "Reset Product").await;

    sqlx::query(
        "INSERT INTO product_areas (id, product_id, node_kind, name, sort_order)
             VALUES ('product_area-reset', 'product-reset', 'product_area', 'Area', 0)",
    )
    .execute(&pool)
    .await
    .expect("product_area should be inserted");
    sqlx::query(
        "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name
             )
             VALUES ('cap-reset', 'product_area-reset', NULL, 0, 'capability', 0, 'Capability')",
    )
    .execute(&pool)
    .await
    .expect("capability should be inserted");
    sqlx::query(
            "INSERT INTO work_items (
                id, product_id, product_area_id, capability_id, title, work_item_type, status
             )
             VALUES ('work-reset', 'product-reset', 'product_area-reset', 'cap-reset', 'Work', 'story', 'draft')",
        )
        .execute(&pool)
        .await
        .expect("work item should be inserted");
    sqlx::query(
        "INSERT INTO agent_work_runs (id, product_id, roadmap_hash)
             VALUES ('run-reset', 'product-reset', 'hash')",
    )
    .execute(&pool)
    .await
    .expect("run should be inserted");
    sqlx::query(
        "INSERT INTO agent_work_items (id, run_id, feature_id, work_item_id, product_area)
             VALUES ('agent-item-reset', 'run-reset', 'feature-reset', 'work-reset', 'Area')",
    )
    .execute(&pool)
    .await
    .expect("agent item should be inserted");
    sqlx::query(
        "INSERT INTO agent_work_events (run_id, event_type, feature_id, work_item_id)
             VALUES ('run-reset', 'imported', 'feature-reset', 'work-reset')",
    )
    .execute(&pool)
    .await
    .expect("agent event should be inserted");
    sqlx::query(
        "INSERT INTO agent_work_evidence (id, run_id, feature_id, work_item_id, evidence_type)
             VALUES ('evidence-reset', 'run-reset', 'feature-reset', 'work-reset', 'test')",
    )
    .execute(&pool)
    .await
    .expect("agent evidence should be inserted");

    let result = reset_product_plan(&pool, "product-reset", true)
        .await
        .expect("reset should succeed");

    assert_eq!(result.product_areas_deleted, 1);
    assert_eq!(result.capabilities_deleted, 1);
    assert_eq!(result.work_items_deleted, 1);
    assert_eq!(result.agent_work_runs_deleted, 1);
    assert_eq!(result.agent_work_items_deleted, 1);
    assert_eq!(result.agent_work_events_deleted, 1);
    assert_eq!(result.agent_work_evidence_deleted, 1);

    let remaining_product_areas: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM product_areas WHERE product_id='product-reset'")
            .fetch_one(&pool)
            .await
            .expect("product_areas should be counted");
    let remaining_work_items: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM work_items WHERE product_id='product-reset'")
            .fetch_one(&pool)
            .await
            .expect("work items should be counted");
    let remaining_runs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_work_runs WHERE product_id='product-reset'")
            .fetch_one(&pool)
            .await
            .expect("runs should be counted");

    assert_eq!(remaining_product_areas, 0);
    assert_eq!(remaining_work_items, 0);
    assert_eq!(remaining_runs, 0);
}

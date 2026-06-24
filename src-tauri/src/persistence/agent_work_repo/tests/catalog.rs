use super::*;
use crate::persistence::{product_repo, work_item_repo};

#[tokio::test]
async fn materialize_catalog_creates_visible_product_tree_and_work_items() {
    let pool = create_test_pool("materialize_catalog").await;
    create_test_product(&pool, "product-mayyam-test", "Mayyam").await;
    create_test_product_area(&pool, "existing-area", "product-mayyam-test", "Payments").await;
    upsert_run(
        &pool,
        UpsertAgentWorkRunInput {
            id: "run-materialize",
            product_id: Some("product-mayyam-test"),
            repository_id: None,
            roadmap_hash: "roadmap-hash",
            status: None,
            last_commit_sha: None,
            current_batch_id: None,
            next_action: Some("Start"),
            metadata: None,
        },
    )
    .await
    .expect("run should be created");
    upsert_item(
        &pool,
        UpsertAgentWorkItemInput {
            run_id: "run-materialize",
            feature_id: "legacy-feature-001",
            work_item_id: None,
            product_area: "Payments",
            service_or_domain: Some("Checkout"),
            priority: Some("P1"),
            release_phase: Some("M1"),
            title: "Add checkout reconciliation",
            description: "Reconcile checkout events against settlements.",
            status: Some("pending"),
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: None,
            metadata: None,
        },
    )
    .await
    .expect("item should be created");

    let result = materialize_catalog(&pool, "run-materialize", None, true)
        .await
        .expect("catalog materialization should succeed");

    assert_eq!(result.total_items, 1);
    assert_eq!(result.product_areas_created, 0);
    assert_eq!(result.product_areas_reused, 1);
    assert_eq!(result.capabilities_created, 1);
    assert_eq!(result.features_upserted, 1);
    assert_eq!(result.work_items_upserted, 1);

    let tree = product_repo::get_product_tree(&pool, "product-mayyam-test")
        .await
        .expect("tree should load");
    assert_eq!(tree.product_areas.len(), 1);
    assert_eq!(tree.product_areas[0].features.len(), 1);
    assert_eq!(tree.product_areas[0].features[0].children.len(), 1);

    let visible_work_items = work_item_repo::list_work_items_page(
        &pool,
        work_item_repo::WorkItemListQuery {
            product_id: Some("product-mayyam-test"),
            limit: Some(20),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("work items should list");
    assert_eq!(visible_work_items.len(), 1);
    assert_eq!(visible_work_items[0].title, "Add checkout reconciliation");

    let linked_item = get_item(&pool, "run-materialize", "legacy-feature-001")
        .await
        .expect("agent work item should load");
    assert_eq!(
        linked_item.work_item_id.as_deref(),
        Some(visible_work_items[0].id.as_str())
    );
}

#[tokio::test]
async fn link_catalog_work_items_reconciles_existing_imported_rows() {
    let pool = create_test_pool("link_catalog_work_items").await;
    create_test_product(&pool, "product-link-test", "Mayyam").await;
    create_test_product_area(&pool, "area-link-test", "product-link-test", "Payments").await;
    sqlx::query(
            "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name, status
             )
             VALUES ('cap-link-test', 'area-link-test', NULL, 0, 'capability', 0, 'Checkout', 'draft')",
        )
        .execute(&pool)
        .await
        .expect("capability should be inserted");
    sqlx::query(
            "INSERT INTO capabilities (
                id, product_area_id, parent_capability_id, level, node_kind, sort_order, name, status,
                technical_notes
             )
             VALUES (
                'mayyam-feature-link-test', 'area-link-test', 'cap-link-test', 1, 'feature', 0,
                'Reconcile checkout', 'draft', 'Roadmap feature id: legacy-feature-link-test.'
             )",
        )
        .execute(&pool)
        .await
        .expect("feature should be inserted");
    sqlx::query(
            "INSERT INTO work_items (
                id, product_id, product_area_id, capability_id, source_node_id, source_node_type,
                title, work_item_type, status
             )
             VALUES (
                'mayyam-work-link-test', 'product-link-test', 'area-link-test', 'mayyam-feature-link-test',
                'mayyam-feature-link-test', 'capability', 'Reconcile checkout', 'story', 'draft'
             )",
        )
        .execute(&pool)
        .await
        .expect("work item should be inserted");
    upsert_run(
        &pool,
        UpsertAgentWorkRunInput {
            id: "run-link-test",
            product_id: Some("product-link-test"),
            repository_id: None,
            roadmap_hash: "roadmap-hash",
            status: None,
            last_commit_sha: None,
            current_batch_id: None,
            next_action: Some("Start"),
            metadata: None,
        },
    )
    .await
    .expect("run should be created");
    upsert_item(
        &pool,
        UpsertAgentWorkItemInput {
            run_id: "run-link-test",
            feature_id: "legacy-feature-link-test",
            work_item_id: None,
            product_area: "Payments",
            service_or_domain: Some("Checkout"),
            priority: Some("P1"),
            release_phase: Some("M1"),
            title: "Reconcile checkout",
            description: "",
            status: Some("committed"),
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: None,
            metadata: None,
        },
    )
    .await
    .expect("item should be created");

    let result = link_catalog_work_items(&pool, "run-link-test", None, true)
        .await
        .expect("linking should succeed");

    assert_eq!(result.total_items, 1);
    assert_eq!(result.already_linked, 0);
    assert_eq!(result.linked_work_items, 1);
    assert_eq!(result.missing_work_items, 0);
    assert_eq!(result.ambiguous_work_items, 0);
    assert_eq!(result.status_synced, 1);

    let linked_item = get_item(&pool, "run-link-test", "legacy-feature-link-test")
        .await
        .expect("agent work item should load");
    assert_eq!(
        linked_item.work_item_id.as_deref(),
        Some("mayyam-work-link-test")
    );

    let work_status: String =
        sqlx::query_scalar("SELECT status FROM work_items WHERE id='mayyam-work-link-test'")
            .fetch_one(&pool)
            .await
            .expect("work status should load");
    assert_eq!(work_status, "done");

    let feature_status: String =
        sqlx::query_scalar("SELECT status FROM capabilities WHERE id='mayyam-feature-link-test'")
            .fetch_one(&pool)
            .await
            .expect("feature status should load");
    assert_eq!(feature_status, "done");

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_events
             WHERE run_id='run-link-test' AND event_type='catalog_work_items_linked'",
    )
    .fetch_one(&pool)
    .await
    .expect("event count should load");
    assert_eq!(event_count, 1);
}

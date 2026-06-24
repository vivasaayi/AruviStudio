use super::*;
use crate::persistence::{db as db_service, product_repo, work_item_repo};

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "aruvi_agent_work_repo_{}_{}",
        name,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

async fn create_test_pool(name: &str) -> SqlitePool {
    let temp_root = make_temp_dir(name);
    let db_path = temp_root.join("aruvi-test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    db_service::create_pool(&db_url)
        .await
        .expect("failed to create database pool")
}

async fn create_test_product(pool: &SqlitePool, product_id: &str, name: &str) {
    product_repo::create_product(
        pool,
        product_repo::CreateProductInput {
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

async fn create_test_product_area(
    pool: &SqlitePool,
    product_area_id: &str,
    product_id: &str,
    name: &str,
) {
    product_repo::create_product_area(
        pool,
        product_repo::CreateProductAreaInput {
            id: product_area_id,
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
    .expect("product area should be created");
}

#[tokio::test]
async fn claim_next_item_holds_conflict_zone_until_commit_releases_it() {
    let pool = create_test_pool("claim_locks").await;

    upsert_run(
        &pool,
        UpsertAgentWorkRunInput {
            id: "run-test",
            product_id: None,
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
            run_id: "run-test",
            feature_id: "01-01",
            work_item_id: None,
            product_area: "calculator-core",
            service_or_domain: None,
            priority: Some("P0"),
            release_phase: None,
            title: "Expression parser",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["product_area:calculator-core"])),
            metadata: None,
        },
    )
    .await
    .expect("first item should be created");
    upsert_item(
        &pool,
        UpsertAgentWorkItemInput {
            run_id: "run-test",
            feature_id: "01-02",
            work_item_id: None,
            product_area: "calculator-core",
            service_or_domain: None,
            priority: Some("P0"),
            release_phase: None,
            title: "Operator precedence",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["product_area:calculator-core"])),
            metadata: None,
        },
    )
    .await
    .expect("second item should be created");

    let first_claim = claim_next_item(
        &pool,
        "run-test",
        "agent-a",
        Some("batch-a"),
        Some("first pass"),
        Some(300),
    )
    .await
    .expect("claim should succeed")
    .expect("an item should be claimed");

    assert_eq!(first_claim.item.feature_id, "01-01");
    assert_eq!(
        first_claim.conflict_zones,
        vec!["product_area:calculator-core".to_string()]
    );

    let blocked_claim = claim_next_item(
        &pool,
        "run-test",
        "agent-b",
        Some("batch-b"),
        Some("same product_area"),
        Some(300),
    )
    .await
    .expect("claim attempt should not error");
    assert!(blocked_claim.is_none());

    link_commit(
        &pool,
        "run-test",
        "batch-a",
        std::slice::from_ref(&first_claim.item.feature_id),
        "abc123",
        Some("agent-a"),
        Some("implemented parser"),
    )
    .await
    .expect("commit should release locks");

    let second_claim = claim_next_item(
        &pool,
        "run-test",
        "agent-b",
        Some("batch-b"),
        Some("after commit"),
        Some(300),
    )
    .await
    .expect("claim should succeed after lock release")
    .expect("second item should be claimable");

    assert_eq!(second_claim.item.feature_id, "01-02");
    assert_eq!(list_active_locks(&pool, "run-test").await.unwrap().len(), 1);
}

#[tokio::test]
async fn lifecycle_updates_heartbeat_status_and_expired_requeue() {
    let pool = create_test_pool("lifecycle").await;

    upsert_run(
        &pool,
        UpsertAgentWorkRunInput {
            id: "run-lifecycle",
            product_id: None,
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
            run_id: "run-lifecycle",
            feature_id: "01-heartbeat",
            work_item_id: None,
            product_area: "calculator-core",
            service_or_domain: None,
            priority: Some("P0"),
            release_phase: None,
            title: "Heartbeat target",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["product_area:calculator-core"])),
            metadata: None,
        },
    )
    .await
    .expect("heartbeat item should be created");
    upsert_item(
        &pool,
        UpsertAgentWorkItemInput {
            run_id: "run-lifecycle",
            feature_id: "02-blocked",
            work_item_id: None,
            product_area: "reporting",
            service_or_domain: None,
            priority: Some("P1"),
            release_phase: None,
            title: "Blocked target",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["product_area:reporting"])),
            metadata: None,
        },
    )
    .await
    .expect("blocked item should be created");

    let heartbeat_claim = claim_next_item(
        &pool,
        "run-lifecycle",
        "agent-a",
        Some("batch-a"),
        Some("claim heartbeat"),
        Some(30),
    )
    .await
    .expect("heartbeat claim should succeed")
    .expect("heartbeat item should be claimed");
    assert_eq!(heartbeat_claim.item.feature_id, "01-heartbeat");

    let heartbeat = heartbeat_item(
        &pool,
        "run-lifecycle",
        "01-heartbeat",
        &heartbeat_claim.claim_token,
        Some(3600),
    )
    .await
    .expect("heartbeat should refresh item and lock lease");
    assert_eq!(heartbeat.status, "claimed");
    assert!(heartbeat.heartbeat_at.is_some());
    assert!(heartbeat.lease_expires_at.is_some());
    let heartbeat_lock = list_active_locks(&pool, "run-lifecycle")
        .await
        .expect("locks should list")
        .into_iter()
        .find(|lock| lock.feature_id.as_deref() == Some("01-heartbeat"))
        .expect("heartbeat lock should remain active");
    assert_eq!(
        Some(heartbeat_lock.lease_expires_at.as_str()),
        heartbeat.lease_expires_at.as_deref()
    );

    sqlx::query(
        "UPDATE agent_work_items
         SET lease_expires_at=datetime('now', '-1 second')
         WHERE run_id='run-lifecycle' AND feature_id='01-heartbeat'",
    )
    .execute(&pool)
    .await
    .expect("item lease should be expired");
    sqlx::query(
        "UPDATE agent_work_locks
         SET lease_expires_at=datetime('now', '-1 second')
         WHERE run_id='run-lifecycle' AND feature_id='01-heartbeat'",
    )
    .execute(&pool)
    .await
    .expect("lock lease should be expired");

    let requeued = requeue_expired_items(
        &pool,
        "run-lifecycle",
        Some("coordinator"),
        Some("lease expired"),
    )
    .await
    .expect("expired claim should be requeued");
    assert_eq!(requeued.len(), 1);
    assert_eq!(requeued[0].feature_id, "01-heartbeat");
    assert_eq!(requeued[0].status, "pending");
    assert!(requeued[0].claim_token.is_none());
    assert!(requeued[0].lease_expires_at.is_none());

    let blocked_claim = claim_next_item(
        &pool,
        "run-lifecycle",
        "agent-b",
        Some("batch-b"),
        Some("claim blocked"),
        Some(300),
    )
    .await
    .expect("blocked claim should succeed")
    .expect("blocked item should be claimed");
    assert_eq!(blocked_claim.item.feature_id, "01-heartbeat");

    let blocked = update_item_status(
        &pool,
        UpdateAgentWorkItemStatusInput {
            run_id: "run-lifecycle",
            feature_id: "01-heartbeat",
            status: "blocked",
            agent: Some("agent-b"),
            batch_id: Some("batch-b"),
            claim_token: Some(&blocked_claim.claim_token),
            commit_sha: None,
            details: Some("blocked by missing dependency"),
        },
    )
    .await
    .expect("terminal status should release locks");
    assert_eq!(blocked.status, "blocked");
    assert_eq!(
        list_active_locks(&pool, "run-lifecycle")
            .await
            .expect("locks should list")
            .len(),
        0
    );

    let status_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_events
         WHERE run_id='run-lifecycle' AND feature_id='01-heartbeat' AND event_type='status' AND status='blocked'",
    )
    .fetch_one(&pool)
    .await
    .expect("status event count should load");
    assert_eq!(status_events, 1);
}

#[tokio::test]
async fn claim_next_item_skips_rows_with_unmet_dependencies() {
    let pool = create_test_pool("claim_dependencies").await;

    upsert_run(
        &pool,
        UpsertAgentWorkRunInput {
            id: "run-deps",
            product_id: None,
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
            run_id: "run-deps",
            feature_id: "01-foundation",
            work_item_id: None,
            product_area: "calculator-core",
            service_or_domain: None,
            priority: Some("P0"),
            release_phase: None,
            title: "Foundation",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["feature:01-foundation"])),
            metadata: None,
        },
    )
    .await
    .expect("foundation should be created");
    upsert_item(
        &pool,
        UpsertAgentWorkItemInput {
            run_id: "run-deps",
            feature_id: "02-dependent",
            work_item_id: None,
            product_area: "calculator-core",
            service_or_domain: None,
            priority: Some("P0"),
            release_phase: None,
            title: "Dependent",
            description: "",
            status: None,
            batch_id: None,
            agent: None,
            commit_sha: None,
            conflict_zones: Some(serde_json::json!(["feature:02-dependent"])),
            metadata: None,
        },
    )
    .await
    .expect("dependent should be created");
    upsert_dependency(
        &pool,
        "run-deps",
        "02-dependent",
        "01-foundation",
        None,
        None,
    )
    .await
    .expect("dependency should be created");

    let ready = list_ready_items(&pool, "run-deps", 10, 0)
        .await
        .expect("ready items should list");
    assert_eq!(
        ready
            .iter()
            .map(|item| item.feature_id.as_str())
            .collect::<Vec<_>>(),
        vec!["01-foundation"]
    );

    let first_claim = claim_next_item(
        &pool,
        "run-deps",
        "agent-a",
        Some("batch-foundation"),
        Some("dependency test"),
        Some(300),
    )
    .await
    .expect("claim should succeed")
    .expect("foundation should be claimed");
    assert_eq!(first_claim.item.feature_id, "01-foundation");

    link_commit(
        &pool,
        "run-deps",
        "batch-foundation",
        &[first_claim.item.feature_id],
        "def456",
        Some("agent-a"),
        Some("foundation complete"),
    )
    .await
    .expect("commit should complete dependency");

    let second_claim = claim_next_item(
        &pool,
        "run-deps",
        "agent-b",
        Some("batch-dependent"),
        Some("dependency ready"),
        Some(300),
    )
    .await
    .expect("claim should succeed")
    .expect("dependent should be claimable");
    assert_eq!(second_claim.item.feature_id, "02-dependent");
}

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

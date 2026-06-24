use super::*;
use std::borrow::Cow;

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aruvi_db_schema_{}_{}", name, uuid::Uuid::new_v4()))
}

async fn create_test_pool(name: &str) -> SqlitePool {
    let temp_root = make_temp_dir(name);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let db_path = temp_root.join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());
    create_pool(&database_url)
        .await
        .expect("test database should be migrated")
}

async fn create_unmigrated_test_pool(name: &str) -> (SqlitePool, String) {
    let temp_root = make_temp_dir(name);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let db_path = temp_root.join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("database URL should parse")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("unmigrated database pool should open");

    (pool, database_url)
}

async fn migrate_through_version(pool: &SqlitePool, version: i64) {
    let all_migrations = sqlx::migrate!("./migrations");
    let legacy_migrations = sqlx::migrate::Migrator {
        migrations: Cow::Owned(
            all_migrations
                .iter()
                .filter(|migration| migration.version <= version)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    legacy_migrations
        .run(pool)
        .await
        .expect("legacy migrations should apply");
}

async fn sqlite_object_exists(pool: &SqlitePool, object_type: &str, name: &str) -> bool {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = ? AND name = ?")
            .bind(object_type)
            .bind(name)
            .fetch_one(pool)
            .await
            .expect("sqlite object lookup should succeed");

    count > 0
}

async fn table_columns(pool: &SqlitePool, table: &str) -> Vec<String> {
    let query = format!("SELECT name FROM pragma_table_info('{table}')");
    sqlx::query_scalar::<_, String>(&query)
        .fetch_all(pool)
        .await
        .expect("table info should be available")
}

async fn sqlite_object_sql(pool: &SqlitePool, object_type: &str, name: &str) -> Option<String> {
    sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = ? AND name = ?")
        .bind(object_type)
        .bind(name)
        .fetch_optional(pool)
        .await
        .expect("sqlite object SQL lookup should succeed")
}

async fn sqlite_object_names_like(
    pool: &SqlitePool,
    object_type: &str,
    pattern: &str,
) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = ? AND name LIKE ? ORDER BY name",
    )
    .bind(object_type)
    .bind(pattern)
    .fetch_all(pool)
    .await
    .expect("sqlite object name lookup should succeed")
}

async fn sqlite_schema_objects(pool: &SqlitePool) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT type, name, COALESCE(sql, '')
         FROM sqlite_master
         WHERE type IN ('table', 'index', 'trigger', 'view')
           AND name NOT LIKE 'sqlite_%'
           AND name != '_sqlx_migrations'
         ORDER BY type, name",
    )
    .fetch_all(pool)
    .await
    .expect("sqlite schema object lookup should succeed")
}

async fn assert_no_legacy_product_area_schema_objects(pool: &SqlitePool) {
    let legacy_sql_fragments = [
        "module_id",
        "references modules",
        "create table modules",
        "scope_type in ('product','module",
        "scope_type in ('module",
    ];
    let violations: Vec<String> = sqlite_schema_objects(pool)
        .await
        .into_iter()
        .filter_map(|(object_type, name, sql)| {
            let normalized_name = name.to_ascii_lowercase();
            let normalized_sql = sql.to_ascii_lowercase();
            let has_legacy_name = normalized_name.contains("module");
            let has_legacy_sql = legacy_sql_fragments
                .iter()
                .any(|fragment| normalized_sql.contains(fragment));
            (has_legacy_name || has_legacy_sql).then(|| format!("{object_type} {name}: {sql}"))
        })
        .collect();

    assert!(
        violations.is_empty(),
        "active product_area schema should not contain legacy module physical names: {violations:#?}"
    );
}

async fn assert_canonical_work_item_summary_indexes(pool: &SqlitePool) {
    let index_names = [
        "idx_work_items_product_area_status_summary",
        "idx_work_items_capability_status_summary",
        "idx_work_items_source_status_summary",
        "idx_work_items_product_status_list",
        "idx_work_items_product_source_list",
        "idx_work_items_product_source_status_list",
    ];

    for index_name in index_names {
        let sql = sqlite_object_sql(pool, "index", index_name)
            .await
            .unwrap_or_else(|| panic!("missing expected index {index_name}"));
        assert!(
            !sql.contains("module_id"),
            "index {index_name} should not use legacy module_id: {sql}"
        );
    }

    let product_area_index =
        sqlite_object_sql(pool, "index", "idx_work_items_product_area_status_summary")
            .await
            .expect("product area summary index should exist");
    assert!(
        product_area_index.contains("product_area_id"),
        "product area summary index should use product_area_id: {product_area_index}"
    );
}

async fn assert_no_product_area_migration_temp_tables(pool: &SqlitePool) {
    let legacy_tables = sqlite_object_names_like(pool, "table", "%_product_area_fk_legacy").await;
    assert!(
        legacy_tables.is_empty(),
        "product_area physical migration should not leave temp tables: {legacy_tables:?}"
    );
}

#[tokio::test]
async fn fresh_migration_uses_product_area_physical_schema() {
    let pool = create_test_pool("product_area_physical_schema").await;

    assert!(sqlite_object_exists(&pool, "table", "product_areas").await);
    assert!(!sqlite_object_exists(&pool, "table", "modules").await);
    assert!(!sqlite_object_exists(&pool, "table", "modules_product_area_fk_legacy").await);
    assert_no_product_area_migration_temp_tables(&pool).await;
    assert_no_legacy_product_area_schema_objects(&pool).await;

    let product_area_columns = table_columns(&pool, "product_areas").await;
    assert!(product_area_columns.contains(&"product_id".to_string()));
    assert!(product_area_columns.contains(&"node_kind".to_string()));

    let capability_columns = table_columns(&pool, "capabilities").await;
    assert!(capability_columns.contains(&"product_area_id".to_string()));
    assert!(!capability_columns.contains(&"module_id".to_string()));

    let work_item_columns = table_columns(&pool, "work_items").await;
    assert!(work_item_columns.contains(&"product_area_id".to_string()));
    assert!(!work_item_columns.contains(&"module_id".to_string()));

    assert!(sqlite_object_exists(&pool, "index", "idx_product_areas_product").await);
    assert!(sqlite_object_exists(&pool, "index", "idx_work_items_product_area").await);
    assert!(sqlite_object_exists(&pool, "index", "idx_work_items_product_area_list").await);
    assert_canonical_work_item_summary_indexes(&pool).await;
}

#[tokio::test]
async fn migration_43_upgrades_legacy_module_physical_schema() {
    let (legacy_pool, database_url) = create_unmigrated_test_pool("legacy_module_upgrade").await;
    migrate_through_version(&legacy_pool, 42).await;

    assert!(sqlite_object_exists(&legacy_pool, "table", "modules").await);
    assert!(table_columns(&legacy_pool, "work_items")
        .await
        .contains(&"module_id".to_string()));
    assert!(table_columns(&legacy_pool, "agent_work_items")
        .await
        .contains(&"module".to_string()));

    sqlx::query(
        "INSERT INTO products (
            id, name, description, vision, goals, tags, status, created_at, updated_at,
            lifecycle, health, owner_label, investment_status, roadmap, evidence
         )
         VALUES (
            'legacy-product', 'Legacy Product', '', '', '[]', '[]', 'active',
            '2026-01-01 00:00:00', '2026-01-01 00:00:00',
            'active', 'healthy', '', 'invest', '', ''
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy product should insert");
    sqlx::query(
        "INSERT INTO modules (
            id, product_id, name, description, purpose, sort_order, created_at, updated_at,
            node_kind, explanation, examples, implementation_notes, test_guidance
         )
         VALUES (
            'legacy-area', 'legacy-product', 'Legacy Area', 'Old module row', '', 0,
            '2026-01-01 00:00:00', '2026-01-01 00:00:00',
            'product_area', '', '', '', ''
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy module should insert");
    sqlx::query(
        "INSERT INTO capabilities (
            id, module_id, parent_capability_id, level, sort_order, name, description,
            acceptance_criteria, priority, risk, status, technical_notes, created_at,
            updated_at, node_kind, explanation, examples, implementation_notes, test_guidance
         )
         VALUES (
            'legacy-capability', 'legacy-area', NULL, 0, 0, 'Legacy Capability', '',
            '', 'medium', 'low', 'draft', '',
            '2026-01-01 00:00:00', '2026-01-01 00:00:00',
            'capability', '', '', '', ''
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy capability should insert");
    sqlx::query(
        "INSERT INTO work_items (
            id, product_id, module_id, capability_id, source_node_id, source_node_type,
            parent_work_item_id, title, problem_statement, description, acceptance_criteria,
            constraints, work_item_type, priority, complexity, status, repo_override_id,
            active_repo_id, branch_name, sort_order, created_at, updated_at
         )
         VALUES (
            'legacy-work', 'legacy-product', 'legacy-area', 'legacy-capability',
            'legacy-capability', 'capability', NULL, 'Legacy Story', '', '', '',
            '', 'story', 'medium', 'medium', 'draft', NULL, NULL, NULL, 0,
            '2026-01-01 00:00:00', '2026-01-01 00:00:00'
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy work item should insert");
    sqlx::query(
        "INSERT INTO agent_work_runs (
            id, product_id, repository_id, roadmap_hash, status, last_commit_sha,
            current_batch_id, next_action, metadata_json, started_at, updated_at
         )
         VALUES (
            'legacy-agent-run', 'legacy-product', NULL, 'hash', 'active', NULL,
            NULL, '', '{}', '2026-01-01 00:00:00', '2026-01-01 00:00:00'
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy agent work run should insert");
    sqlx::query(
        "INSERT INTO agent_work_items (
            id, run_id, feature_id, work_item_id, module, service_or_domain, priority,
            release_phase, title, description, status, batch_id, agent, commit_sha,
            claim_token, lease_expires_at, heartbeat_at, conflict_zones_json, metadata_json,
            created_at, updated_at
         )
         VALUES (
            'legacy-agent-item', 'legacy-agent-run', 'legacy-feature', 'legacy-work',
            'Legacy Area', NULL, 'P1', NULL, 'Legacy Agent Item', '', 'pending',
            NULL, NULL, NULL, NULL, NULL, NULL, '[]', '{}',
            '2026-01-01 00:00:00', '2026-01-01 00:00:00'
         )",
    )
    .execute(&legacy_pool)
    .await
    .expect("legacy agent work item should insert");

    legacy_pool.close().await;
    let upgraded_pool = create_pool(&database_url)
        .await
        .expect("normal app migrator should upgrade legacy physical names");

    assert!(sqlite_object_exists(&upgraded_pool, "table", "product_areas").await);
    assert!(!sqlite_object_exists(&upgraded_pool, "table", "modules").await);
    assert_no_product_area_migration_temp_tables(&upgraded_pool).await;
    assert_no_legacy_product_area_schema_objects(&upgraded_pool).await;
    assert!(!table_columns(&upgraded_pool, "capabilities")
        .await
        .contains(&"module_id".to_string()));
    assert!(!table_columns(&upgraded_pool, "work_items")
        .await
        .contains(&"module_id".to_string()));
    assert!(!table_columns(&upgraded_pool, "agent_work_items")
        .await
        .contains(&"module".to_string()));

    let product_area_name: String =
        sqlx::query_scalar("SELECT name FROM product_areas WHERE id='legacy-area'")
            .fetch_one(&upgraded_pool)
            .await
            .expect("legacy module should become product_area");
    assert_eq!(product_area_name, "Legacy Area");

    let capability_product_area_id: String =
        sqlx::query_scalar("SELECT product_area_id FROM capabilities WHERE id='legacy-capability'")
            .fetch_one(&upgraded_pool)
            .await
            .expect("legacy capability should be preserved");
    assert_eq!(capability_product_area_id, "legacy-area");

    let work_item_product_area_id: String =
        sqlx::query_scalar("SELECT product_area_id FROM work_items WHERE id='legacy-work'")
            .fetch_one(&upgraded_pool)
            .await
            .expect("legacy work item should be preserved");
    assert_eq!(work_item_product_area_id, "legacy-area");

    let agent_work_product_area: String = sqlx::query_scalar(
        "SELECT product_area FROM agent_work_items WHERE id='legacy-agent-item'",
    )
    .fetch_one(&upgraded_pool)
    .await
    .expect("legacy agent work item should be preserved");
    assert_eq!(agent_work_product_area, "Legacy Area");

    assert!(sqlite_object_exists(&upgraded_pool, "index", "idx_product_areas_product").await);
    assert!(sqlite_object_exists(&upgraded_pool, "index", "idx_work_items_product_area").await);
    assert!(
        sqlite_object_exists(&upgraded_pool, "index", "idx_work_items_product_area_list").await
    );
    assert_canonical_work_item_summary_indexes(&upgraded_pool).await;
}

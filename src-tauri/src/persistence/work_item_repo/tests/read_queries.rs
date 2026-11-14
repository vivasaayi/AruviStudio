use super::*;

#[tokio::test]
async fn product_export_includes_all_pages_and_children_without_other_products() {
    let pool = create_test_pool("product_export").await;
    create_test_product(&pool, "book-product").await;
    create_test_product(&pool, "other-product").await;
    sqlx::query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<2101) INSERT INTO work_items (id,product_id,title) SELECT 'book-' || x,'book-product','Story ' || x FROM n")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO work_items (id,product_id,title,parent_work_item_id,work_item_type) VALUES ('book-child','book-product','Child task','book-1','task')")
        .execute(&pool).await.unwrap();
    create_work_item(&pool, test_work_item_input("other-item", "other-product", "Other product")).await.unwrap();
    let items = list_product_export_work_items(&pool, "book-product").await.unwrap();
    assert_eq!(items.len(), 2102);
    assert!(items.iter().any(|item| item.id == "book-child" && item.parent_work_item_id.as_deref() == Some("book-1")));
    assert!(items.iter().all(|item| item.product_id.as_deref() == Some("book-product")));
    assert!(list_product_export_work_items(&pool, "missing-product").await.is_err());
    let snapshot = crate::persistence::product_book_repo::get_product_book_source(&pool, "book-product").await.unwrap();
    assert_eq!(snapshot.work_items.len(), 2102);
    assert!(snapshot.work_items.iter().all(|item| item.product_id.as_deref() == Some("book-product")));
}

#[tokio::test]
async fn product_book_snapshot_is_consistent_and_product_scoped() {
    use crate::persistence::{product_book_repo, product_tree_repo};
    let pool = create_test_pool("book_snapshot").await;
    create_test_product(&pool, "book-product").await;
    create_test_product(&pool, "other-product").await;
    create_test_product_area(&pool, "book-area", "book-product", "Area").await;
    create_test_product_area(&pool, "other-area", "other-product", "Other").await;
    create_test_capability(&pool, "book-cap", "book-area", None, "Capability", Some("capability")).await;
    create_test_capability(&pool, "book-feature", "book-area", Some("book-cap"), "Feature", Some("feature")).await;
    create_test_capability(&pool, "other-cap", "other-area", None, "Other capability", Some("capability")).await;
    sqlx::query("INSERT INTO \"references\" (id, scope_type, scope_id, title) VALUES ('own-product','product','book-product','Own'), ('own-area','product_area','book-area','Area'), ('own-cap','capability','book-cap','Capability'), ('own-feature','feature','book-feature','Feature'), ('wrong-kind','capability','book-feature','Wrong'), ('other-cap-ref','capability','other-cap','Other capability'), ('other-product-ref','product','other-product','Other'), ('other-area-ref','product_area','other-area','Other area')").execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    product_tree_repo::get_product_tree_in_conn(&mut tx, "book-product").await.unwrap();
    create_work_item(&pool, test_work_item_input("late-item", "book-product", "Late")).await.unwrap();
    sqlx::query("INSERT INTO \"references\" (id,scope_type,scope_id,title) VALUES ('late-ref','product','book-product','Late')").execute(&pool).await.unwrap();
    let old = product_book_repo::read_product_book_source(&mut tx, "book-product").await.unwrap();
    assert!(old.work_items.is_empty());
    let mut ids: Vec<_> = old.references.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["own-area", "own-cap", "own-feature", "own-product"]);
    tx.commit().await.unwrap();
    let fresh = product_book_repo::get_product_book_source(&pool, "book-product").await.unwrap();
    assert_eq!(fresh.work_items.len(), 1);
    assert_eq!(fresh.references.len(), 5);
    use crate::services::product_book_renderer::{render_product_book, ProductBookFormat, ProductBookRenderMode, ProductBookTrim};
    let rendered = render_product_book(&fresh, ProductBookFormat::Book, ProductBookRenderMode::Epub, ProductBookTrim::A5).unwrap();
    let html = rendered["html"].as_str().unwrap();
    assert!(html.contains("Late"));
    assert!(html.contains("Feature"));
    assert!(!html.contains("Other area"));
    assert!(rendered["tocItems"].as_array().unwrap().len() > 1);
    assert!(product_book_repo::get_product_book_source(&pool, "missing").await.is_err());
}

#[tokio::test]
async fn list_work_items_page_applies_limit_and_offset() {
    let pool = create_test_pool("list_page").await;
    create_test_product(&pool, "product-work-page").await;

    for index in 0..5 {
        let work_item_id = format!("work-item-{index}");
        let title = format!("Work item {index}");
        create_work_item(
            &pool,
            test_work_item_input(&work_item_id, "product-work-page", &title),
        )
        .await
        .expect("work item should be created");
    }

    let page = list_work_items_page(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-work-page"),
            limit: Some(2),
            offset: Some(1),
            ..Default::default()
        },
    )
    .await
    .expect("page should load");

    assert_eq!(page.len(), 2);
    assert_eq!(page[0].title, "Work item 1");
    assert_eq!(page[1].title, "Work item 2");
}

#[tokio::test]
async fn list_work_items_page_with_metadata_probes_for_next_page() {
    let pool = create_test_pool("list_page_metadata").await;
    create_test_product(&pool, "product-list-page-metadata").await;

    for (id, title) in [
        ("metadata-story-1", "First"),
        ("metadata-story-2", "Second"),
        ("metadata-story-3", "Third"),
    ] {
        create_work_item(
            &pool,
            test_work_item_input(id, "product-list-page-metadata", title),
        )
        .await
        .expect("work item should be created");
    }

    let first_page = list_work_items_page_with_metadata(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-list-page-metadata"),
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("metadata page should load");

    assert_eq!(first_page.limit, 2);
    assert_eq!(first_page.offset, 0);
    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.has_more);

    let second_page = list_work_items_page_with_metadata(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-list-page-metadata"),
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        },
    )
    .await
    .expect("metadata page should load");

    assert_eq!(second_page.items.len(), 1);
    assert!(!second_page.has_more);
}

#[tokio::test]
async fn bounded_work_item_read_helpers_do_not_require_full_product_scan() {
    let pool = create_test_pool("bounded_reads").await;
    create_test_product(&pool, "product-bounded-reads").await;

    for (index, title) in ["Exact Target", "Target One", "Target Two"]
        .iter()
        .enumerate()
    {
        let story_id = format!("bounded-story-{index}");
        create_work_item(
            &pool,
            test_work_item_input(&story_id, "product-bounded-reads", title),
        )
        .await
        .expect("story should be created");
    }
    let mut child_input = test_work_item_input(
        "bounded-child-target",
        "product-bounded-reads",
        "Target Child",
    );
    child_input.parent_work_item_id = Some("bounded-story-0");
    child_input.work_item_type = "task";
    create_work_item(&pool, child_input)
        .await
        .expect("task should be created");

    let search_matches =
        search_work_items_by_title(&pool, Some("product-bounded-reads"), "Exact Target", 2)
            .await
            .expect("title search should load");
    assert_eq!(search_matches[0].id, "bounded-story-0");

    let top_level = list_top_level_work_items_page(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-bounded-reads"),
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("top-level page should load");
    assert_eq!(top_level.len(), 2);
    assert!(top_level
        .iter()
        .all(|item| item.parent_work_item_id.is_none()));

    let child_page = get_sub_work_items_page(&pool, "bounded-story-0", Some(1), Some(0))
        .await
        .expect("child page should load");
    assert_eq!(child_page.len(), 1);
    assert_eq!(child_page[0].id, "bounded-child-target");
}

#[tokio::test]
async fn summarize_work_items_by_product_counts_total_active_and_done() {
    let pool = create_test_pool("summarize_counts").await;
    create_test_product(&pool, "product-summary-counts").await;

    for (index, status) in ["draft", "done", "cancelled", "blocked"].iter().enumerate() {
        let work_item_id = format!("summary-work-item-{index}");
        let title = format!("Summary work item {index}");
        create_work_item(
            &pool,
            test_work_item_input(&work_item_id, "product-summary-counts", &title),
        )
        .await
        .expect("work item should be created");
        update_work_item(&pool, status_patch(&work_item_id, status))
            .await
            .expect("status should update");
    }

    let summaries = summarize_work_items_by_product(&pool)
        .await
        .expect("summaries should load");
    let summary = summaries
        .iter()
        .find(|entry| entry.product_id == "product-summary-counts")
        .expect("product summary should be present");

    assert_eq!(summary.total_count, 4);
    assert_eq!(summary.active_count, 2);
    assert_eq!(summary.done_count, 1);
    assert_eq!(summary.blocked_count, 1);
}

#[tokio::test]
async fn summary_and_paged_reads_handle_50k_product_without_loading_all_rows() {
    let pool = create_test_pool("summarize_50k").await;
    create_test_product(&pool, "product-scale-50k").await;
    create_test_product_area(&pool, "area-scale-50k", "product-scale-50k", "Scale Area").await;
    create_test_capability(
        &pool,
        "capability-scale-50k",
        "area-scale-50k",
        None,
        "Scale Capability",
        Some("capability"),
    )
    .await;
    create_test_capability(
        &pool,
        "feature-scale-50k",
        "area-scale-50k",
        Some("capability-scale-50k"),
        "Scale Feature",
        Some("feature"),
    )
    .await;

    for chunk_start in (0..50_000).step_by(1_000) {
        let mut builder = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
                "INSERT INTO work_items (
                    id, product_id, product_area_id, capability_id, source_node_id, source_node_type,
                    parent_work_item_id, title, work_item_type, priority, complexity, status, sort_order
                ) ",
            );
        builder.push_values(chunk_start..(chunk_start + 1_000), |mut row, index| {
            let status = if index % 10 == 0 {
                "done"
            } else if index % 10 == 1 {
                "blocked"
            } else {
                "in_progress"
            };
            row.push_bind(format!("scale-work-item-{index:05}"))
                .push_bind("product-scale-50k")
                .push_bind("area-scale-50k")
                .push_bind("feature-scale-50k")
                .push_bind("feature-scale-50k")
                .push_bind("capability")
                .push_bind(None::<String>)
                .push_bind(format!("Scale work item {index:05}"))
                .push_bind("story")
                .push_bind("medium")
                .push_bind("medium")
                .push_bind(status)
                .push_bind(index as i64);
        });
        builder
            .build()
            .execute(&pool)
            .await
            .expect("bulk work items should insert");
    }

    assert_query_plan_uses_index(
        &pool,
        "EXPLAIN QUERY PLAN
         SELECT id FROM work_items
         WHERE product_id = ?
         ORDER BY sort_order, created_at DESC
         LIMIT 25 OFFSET 0",
        &["product-scale-50k"],
        "idx_work_items_product_list",
    )
    .await;
    assert_query_plan_uses_index(
        &pool,
        "EXPLAIN QUERY PLAN
         SELECT id FROM work_items
         WHERE product_id = ? AND status = ?
         ORDER BY sort_order, created_at DESC
         LIMIT 25 OFFSET 0",
        &["product-scale-50k", "in_progress"],
        "idx_work_items_product_status_list",
    )
    .await;
    assert_query_plan_uses_index(
        &pool,
        "EXPLAIN QUERY PLAN
         SELECT id FROM work_items
         WHERE product_id = ? AND source_node_type = ? AND source_node_id = ?
         ORDER BY sort_order, created_at DESC
         LIMIT 25 OFFSET 0",
        &["product-scale-50k", "capability", "feature-scale-50k"],
        "idx_work_items_product_source_list",
    )
    .await;
    assert_query_plan_uses_index(
        &pool,
        "EXPLAIN QUERY PLAN
         SELECT id FROM work_items
         WHERE product_id = ? AND source_node_type = ? AND source_node_id = ? AND status = ?
         ORDER BY sort_order, created_at DESC
         LIMIT 25 OFFSET 0",
        &[
            "product-scale-50k",
            "capability",
            "feature-scale-50k",
            "in_progress",
        ],
        "idx_work_items_product_source_status_list",
    )
    .await;

    let product_summaries = summarize_work_items_by_product(&pool)
        .await
        .expect("product summaries should load");
    let product_summary = product_summaries
        .iter()
        .find(|entry| entry.product_id == "product-scale-50k")
        .expect("scale product summary should be present");
    assert_eq!(product_summary.total_count, 50_000);
    assert_eq!(product_summary.done_count, 5_000);
    assert_eq!(product_summary.blocked_count, 5_000);
    assert_eq!(product_summary.active_count, 45_000);

    let scope_summaries = summarize_work_items_by_scope(&pool, Some("product-scale-50k"))
        .await
        .expect("scope summaries should load");
    assert_eq!(scope_summaries.len(), 3);
    assert_eq!(
        scope_summaries
            .iter()
            .map(|entry| entry.total_count)
            .sum::<i64>(),
        50_000
    );
    assert!(scope_summaries.iter().all(|entry| {
        entry.source_node_id.as_deref() == Some("feature-scale-50k")
            && entry.source_node_type.as_deref() == Some("capability")
    }));

    let first_page = list_work_items_page_with_metadata(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-scale-50k"),
            limit: Some(25),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("first Mayyam-scale page should load");
    assert_eq!(first_page.limit, 25);
    assert_eq!(first_page.offset, 0);
    assert_eq!(first_page.items.len(), 25);
    assert!(first_page.has_more);
    assert_eq!(first_page.items[0].id, "scale-work-item-00000");
    assert_eq!(first_page.items[24].id, "scale-work-item-00024");

    let offset_page = list_work_items_page(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-scale-50k"),
            limit: Some(10),
            offset: Some(25),
            ..Default::default()
        },
    )
    .await
    .expect("offset Mayyam-scale page should load");
    assert_eq!(offset_page.len(), 10);
    assert_eq!(offset_page[0].id, "scale-work-item-00025");
    assert_eq!(offset_page[9].id, "scale-work-item-00034");

    let capped_page = list_work_items_page_with_metadata(
        &pool,
        WorkItemListQuery {
            product_id: Some("product-scale-50k"),
            limit: Some(50_000),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("oversized Mayyam-scale page should be capped");
    assert_eq!(capped_page.limit, 2_000);
    assert_eq!(capped_page.items.len(), 2_000);
    assert!(capped_page.has_more);
    assert_eq!(capped_page.items[0].id, "scale-work-item-00000");
    assert_eq!(capped_page.items[1_999].id, "scale-work-item-01999");
}

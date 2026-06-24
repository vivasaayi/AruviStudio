use super::*;
use crate::commands::product_commands;
use crate::commands::test_helpers::make_test_app;
use crate::domain::product::{Capability, Product, ProductArea};
use tauri::test::MockRuntime;
use tauri::Manager;

async fn create_test_product(state: State<'_, AppState>, name: &str) -> Product {
    product_commands::create_product(
        state,
        product_commands::CreateProductCommand {
            name: name.to_string(),
            description: "".to_string(),
            vision: "".to_string(),
            goals: "[]".to_string(),
            tags: "[]".to_string(),
            lifecycle: None,
            health: None,
            owner_label: None,
            investment_status: None,
            roadmap: None,
            evidence: None,
        },
    )
    .await
    .expect("product should be created")
}

async fn create_test_product_area(
    state: State<'_, AppState>,
    product_id: String,
    name: &str,
) -> ProductArea {
    product_commands::create_product_area(
        state,
        product_commands::CreateProductAreaCommand {
            product_id,
            name: name.to_string(),
            description: "".to_string(),
            purpose: "".to_string(),
            node_kind: Some("product_area".to_string()),
            explanation: None,
            examples: None,
            implementation_notes: None,
            test_guidance: None,
        },
    )
    .await
    .expect("product_area should be created")
}

async fn create_test_capability(
    state: State<'_, AppState>,
    product_area_id: String,
    name: &str,
) -> Capability {
    product_commands::create_capability(
        state,
        product_commands::CreateCapabilityCommand {
            product_area_id,
            parent_capability_id: None,
            name: name.to_string(),
            description: "".to_string(),
            acceptance_criteria: "".to_string(),
            priority: "medium".to_string(),
            risk: "low".to_string(),
            technical_notes: "".to_string(),
            node_kind: Some("capability".to_string()),
            explanation: None,
            examples: None,
            implementation_notes: None,
            test_guidance: None,
        },
    )
    .await
    .expect("capability should be created")
}

#[tokio::test]
async fn create_work_item_accepts_canonical_scope_fields() {
    let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_create").await;
    let state = app.state::<AppState>();

    let product = create_test_product(state.clone(), "Work Item Product").await;
    let product_area = create_test_product_area(state.clone(), product.id.clone(), "Area").await;
    let capability =
        create_test_capability(state.clone(), product_area.id.clone(), "Capability").await;

    let work_item = create_work_item(
        state,
        CreateWorkItemCommand {
            product_id: Some(product.id.clone()),
            product_area_id: Some(product_area.id.clone()),
            capability_id: Some(capability.id.clone()),
            source_node_id: Some(capability.id.clone()),
            source_node_type: Some("capability".to_string()),
            parent_work_item_id: None,
            title: "Canonical Work Item".to_string(),
            problem_statement: "Problem from canonical field".to_string(),
            description: "".to_string(),
            acceptance_criteria: "Acceptance from canonical field".to_string(),
            constraints: "".to_string(),
            work_item_type: "story".to_string(),
            priority: "high".to_string(),
            complexity: "medium".to_string(),
        },
    )
    .await
    .expect("work item should be created");

    assert_eq!(work_item.product_id.as_deref(), Some(product.id.as_str()));
    assert_eq!(
        work_item.product_area_id.as_deref(),
        Some(product_area.id.as_str())
    );
    assert_eq!(
        work_item.capability_id.as_deref(),
        Some(capability.id.as_str())
    );
    assert_eq!(work_item.problem_statement, "Problem from canonical field");
    assert_eq!(
        work_item.acceptance_criteria,
        "Acceptance from canonical field"
    );
    assert_eq!(work_item.work_item_type.to_string(), "story");
}

#[tokio::test]
async fn list_work_items_applies_filters_and_pagination_from_command_layer() {
    let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_list").await;
    let state = app.state::<AppState>();

    let product = create_test_product(state.clone(), "List Product").await;

    let mut first_story_id = String::new();
    for (index, status) in ["draft", "done", "draft"].iter().enumerate() {
        let item = create_work_item(
            state.clone(),
            CreateWorkItemCommand {
                product_id: Some(product.id.clone()),
                product_area_id: None,
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: format!("Item {index}"),
                problem_statement: "".to_string(),
                description: "".to_string(),
                acceptance_criteria: "".to_string(),
                constraints: "".to_string(),
                work_item_type: "story".to_string(),
                priority: "medium".to_string(),
                complexity: "medium".to_string(),
            },
        )
        .await
        .expect("work item should be created");

        if index == 0 {
            first_story_id = item.id.clone();
        }
        if *status != "draft" {
            update_work_item(
                state.clone(),
                UpdateWorkItemCommand {
                    id: item.id,
                    title: None,
                    description: None,
                    status: Some((*status).to_string()),
                    problem_statement: None,
                    acceptance_criteria: None,
                    constraints: None,
                },
            )
            .await
            .expect("status should update");
        }
    }
    let page = list_work_items(
        state.clone(),
        ListWorkItemsCommand {
            product_id: Some(product.id.clone()),
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            status: Some("draft".to_string()),
            limit: Some(1),
            offset: Some(1),
            top_level_only: None,
        },
    )
    .await
    .expect("work items should list");

    assert_eq!(page.len(), 1);
    assert_eq!(page[0].title, "Item 2");
    assert_eq!(page[0].status.to_string(), "draft");

    create_work_item(
        state.clone(),
        CreateWorkItemCommand {
            product_id: Some(product.id.clone()),
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: Some(first_story_id.clone()),
            title: "Child task should not appear in top-level page".to_string(),
            problem_statement: "".to_string(),
            description: "".to_string(),
            acceptance_criteria: "".to_string(),
            constraints: "".to_string(),
            work_item_type: "task".to_string(),
            priority: "medium".to_string(),
            complexity: "medium".to_string(),
        },
    )
    .await
    .expect("child task should be created");

    let metadata_page = list_work_items_page(
        state,
        ListWorkItemsCommand {
            product_id: Some(product.id),
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            status: Some("draft".to_string()),
            limit: Some(1),
            offset: Some(0),
            top_level_only: Some(true),
        },
    )
    .await
    .expect("work item metadata page should list");

    assert_eq!(metadata_page.limit, 1);
    assert_eq!(metadata_page.offset, 0);
    assert_eq!(metadata_page.items.len(), 1);
    assert_eq!(metadata_page.items[0].title, "Item 0");
    assert!(metadata_page.items[0].parent_work_item_id.is_none());
    assert!(metadata_page.has_more);
}

#[tokio::test]
async fn legacy_list_work_items_is_bounded_when_limit_is_missing_or_oversized() {
    let app: tauri::App<MockRuntime> = make_test_app("work_item_commands_legacy_list_bounds").await;
    let state = app.state::<AppState>();

    let product = create_test_product(state.clone(), "Legacy List Bounds Product").await;

    for chunk_start in (0..2_100).step_by(300) {
        let chunk_end = (chunk_start + 300).min(2_100);
        let mut builder = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "INSERT INTO work_items (
                    id, product_id, parent_work_item_id, title, work_item_type,
                    priority, complexity, status, sort_order
                ) ",
        );
        builder.push_values(chunk_start..chunk_end, |mut row, index| {
            row.push_bind(format!("legacy-bounded-work-item-{index:04}"))
                .push_bind(&product.id)
                .push_bind(None::<String>)
                .push_bind(format!("Legacy bounded work item {index:04}"))
                .push_bind("story")
                .push_bind("medium")
                .push_bind("medium")
                .push_bind("draft")
                .push_bind(index as i64);
        });
        builder
            .build()
            .execute(&state.db)
            .await
            .expect("legacy bound work items should insert");
    }

    let default_page = list_work_items(
        state.clone(),
        ListWorkItemsCommand {
            product_id: Some(product.id.clone()),
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            status: None,
            limit: None,
            offset: None,
            top_level_only: None,
        },
    )
    .await
    .expect("legacy default bounded page should list");

    assert_eq!(
        default_page.len(),
        usize::try_from(work_item_repo::DEFAULT_LIST_WORK_ITEMS_LIMIT).unwrap()
    );
    assert_eq!(default_page[0].id, "legacy-bounded-work-item-0000");
    assert_eq!(default_page[499].id, "legacy-bounded-work-item-0499");

    let oversized_page = list_work_items(
        state,
        ListWorkItemsCommand {
            product_id: Some(product.id),
            product_area_id: None,
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            status: None,
            limit: Some(50_000),
            offset: None,
            top_level_only: None,
        },
    )
    .await
    .expect("legacy oversized bounded page should list");

    assert_eq!(
        oversized_page.len(),
        usize::try_from(work_item_repo::MAX_LIST_WORK_ITEMS_LIMIT).unwrap()
    );
    assert_eq!(oversized_page[0].id, "legacy-bounded-work-item-0000");
    assert_eq!(oversized_page[1_999].id, "legacy-bounded-work-item-1999");
}

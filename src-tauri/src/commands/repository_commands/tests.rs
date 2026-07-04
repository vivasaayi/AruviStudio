use super::*;
use crate::commands::{product_commands, test_helpers::make_test_app, work_item_commands};
use crate::state::AppState;
use tauri::test::MockRuntime;
use tauri::{Manager, State};

async fn create_work_item_with_product_area(
    state: State<'_, AppState>,
    product_name: &str,
    product_area_name: &str,
    title: &str,
) -> (String, String, String) {
    let product = product_commands::create_product(
        state.clone(),
        product_commands::CreateProductCommand {
            name: product_name.to_string(),
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
    .expect("product should be created");
    let product_area = product_commands::hierarchy::create_product_area(
        state.clone(),
        product_commands::CreateProductAreaCommand {
            product_id: product.id.clone(),
            name: product_area_name.to_string(),
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
    .expect("product_area should be created");
    let work_item = work_item_commands::create_work_item(
        state,
        work_item_commands::CreateWorkItemCommand {
            product_id: Some(product.id.clone()),
            product_area_id: Some(product_area.id.clone()),
            capability_id: None,
            source_node_id: None,
            source_node_type: None,
            parent_work_item_id: None,
            title: title.to_string(),
            problem_statement: "Problem".to_string(),
            description: "Description".to_string(),
            acceptance_criteria: "Acceptance".to_string(),
            constraints: "".to_string(),
            work_item_type: "story".to_string(),
            priority: "medium".to_string(),
            complexity: "medium".to_string(),
        },
    )
    .await
    .expect("work item should be created");

    (product.id, product_area.id, work_item.id)
}

#[tokio::test]
async fn repository_resolution_prefers_product_area_attachment_over_product_attachment() {
    let app: tauri::App<MockRuntime> = make_test_app("repository_commands_resolve").await;
    let state = app.state::<AppState>();
    let (product_id, product_area_id, work_item_id) = create_work_item_with_product_area(
        state.clone(),
        "Repository Product",
        "Repository ProductArea",
        "Repository Work Item",
    )
    .await;

    let product_repo = register_repository(
        state.clone(),
        "Product Repo".to_string(),
        "/tmp/product-repo".to_string(),
        "".to_string(),
        "main".to_string(),
    )
    .await
    .expect("product repo should be created");
    let product_area_repo = register_repository(
        state.clone(),
        "ProductArea Repo".to_string(),
        "/tmp/product_area-repo".to_string(),
        "".to_string(),
        "develop".to_string(),
    )
    .await
    .expect("product_area repo should be created");

    attach_repository(
        state.clone(),
        "product".to_string(),
        product_id.clone(),
        product_repo.id.clone(),
        true,
    )
    .await
    .expect("product attachment should be created");
    attach_repository(
        state.clone(),
        "product_area".to_string(),
        product_area_id.clone(),
        product_area_repo.id.clone(),
        true,
    )
    .await
    .expect("product_area attachment should be created");

    let resolved_for_product =
        resolve_repository_for_scope(state.clone(), Some(product_id.clone()), None)
            .await
            .expect("product scope should resolve")
            .expect("product repo should exist");
    let resolved_for_scope =
        resolve_repository_for_scope(state.clone(), Some(product_id), Some(product_area_id))
            .await
            .expect("product area scope should resolve")
            .expect("product area repo should exist");
    let resolved_for_work_item = resolve_repository_for_work_item(state, work_item_id)
        .await
        .expect("work item repo should resolve")
        .expect("work item repo should exist");

    assert_eq!(resolved_for_product.id, product_repo.id);
    assert_eq!(resolved_for_scope.id, product_area_repo.id);
    assert_eq!(resolved_for_work_item.id, product_area_repo.id);
}

#[tokio::test]
async fn updating_repository_propagates_new_default_branch_to_assigned_work_items() {
    let app: tauri::App<MockRuntime> = make_test_app("repository_commands_update").await;
    let state = app.state::<AppState>();
    let (_, _, work_item_id) = create_work_item_with_product_area(
        state.clone(),
        "Workspace Product",
        "Workspace ProductArea",
        "Workspace Work Item",
    )
    .await;

    let repository = register_repository(
        state.clone(),
        "Workspace Repo".to_string(),
        "/tmp/workspace-repo".to_string(),
        "git@example.com:workspace/repo.git".to_string(),
        "main".to_string(),
    )
    .await
    .expect("repository should be created");

    work_item_commands::assign_work_item_workspace(
        state.clone(),
        work_item_id.clone(),
        Some(repository.id.clone()),
        Some("main".to_string()),
    )
    .await
    .expect("workspace should be assigned");

    let updated_repository = update_repository(
        state.clone(),
        repository.id.clone(),
        "Workspace Repo".to_string(),
        "/tmp/workspace-repo".to_string(),
        "git@example.com:workspace/repo.git".to_string(),
        "develop".to_string(),
    )
    .await
    .expect("repository should update");
    let updated_work_item = work_item_commands::get_work_item(state, work_item_id)
        .await
        .expect("work item should load");

    assert_eq!(updated_repository.default_branch, "develop");
    assert_eq!(
        updated_work_item.active_repo_id.as_deref(),
        Some(repository.id.as_str())
    );
    assert_eq!(updated_work_item.branch_name.as_deref(), Some("develop"));
}

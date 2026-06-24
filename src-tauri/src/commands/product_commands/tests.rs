use super::*;
use crate::commands::test_helpers::make_test_app;
use crate::persistence::settings_repo;
use tauri::test::MockRuntime;
use tauri::Manager;

#[tokio::test]
async fn create_product_and_product_area_accepts_optional_detail_fields() {
    let app: tauri::App<MockRuntime> = make_test_app("product_commands_create_product_area").await;
    let state = app.state::<AppState>();

    let product = create_product(
        state.clone(),
        CreateProductCommand {
            name: "API Product".to_string(),
            description: "Desc".to_string(),
            vision: "Vision".to_string(),
            goals: "[]".to_string(),
            tags: "[]".to_string(),
            lifecycle: Some("active".to_string()),
            health: Some("healthy".to_string()),
            owner_label: Some("Owner".to_string()),
            investment_status: Some("invest".to_string()),
            roadmap: Some("Roadmap".to_string()),
            evidence: Some("Evidence".to_string()),
        },
    )
    .await
    .expect("product should be created");

    let product_area = hierarchy::create_product_area(
        state,
        CreateProductAreaCommand {
            product_id: product.id.clone(),
            name: "Area API".to_string(),
            description: "".to_string(),
            purpose: "".to_string(),
            node_kind: Some("product_area".to_string()),
            explanation: None,
            examples: None,
            implementation_notes: Some("Use camelCase implementation notes".to_string()),
            test_guidance: Some("Use camelCase test guidance".to_string()),
        },
    )
    .await
    .expect("product_area should be created");

    assert_eq!(product_area.node_kind.to_string(), "product_area");
    assert_eq!(
        product_area.implementation_notes,
        "Use camelCase implementation notes"
    );
    assert_eq!(product_area.test_guidance, "Use camelCase test guidance");
}

#[tokio::test]
async fn list_products_hides_example_products_when_setting_enabled() {
    let app: tauri::App<MockRuntime> = make_test_app("product_commands_list_products").await;
    let state = app.state::<AppState>();

    create_product(
        state.clone(),
        CreateProductCommand {
            name: "Custom Product".to_string(),
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
    .expect("custom product should be created");

    settings_repo::set_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, "true")
        .await
        .expect("setting should update");
    let hidden = list_products(state.clone())
        .await
        .expect("products should list");
    assert!(hidden.iter().all(|product| !product.is_example_product()));
    assert!(hidden
        .iter()
        .any(|product| product.name == "Custom Product"));

    settings_repo::set_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, "false")
        .await
        .expect("setting should update");
    let visible = list_products(state).await.expect("products should list");
    assert!(visible.iter().any(|product| product.is_example_product()));
}

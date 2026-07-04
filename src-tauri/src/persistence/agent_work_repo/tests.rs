use super::*;
use crate::persistence::{db as db_service, product_repo};

mod catalog;
mod claim_flows;

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

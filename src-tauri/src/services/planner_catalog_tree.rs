use crate::domain::product::Capability;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::services::planner_catalog::find_product;
use crate::services::planner_service::PlannerTreeNode;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

const PLANNER_TREE_WORK_ITEM_LIMIT: usize = 500;

pub(crate) async fn build_tree_nodes(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<Vec<PlannerTreeNode>, AppError> {
    let products = if let Some(name) = product_name {
        vec![find_product(db, Some(name)).await?]
    } else {
        product_repo::list_products(db).await?
    };

    let mut nodes = vec![];
    for product in products {
        let mut product_items = work_item_repo::list_work_items_page(
            db,
            work_item_repo::WorkItemListQuery {
                product_id: Some(&product.id),
                limit: Some((PLANNER_TREE_WORK_ITEM_LIMIT + 1) as i64),
                offset: Some(0),
                ..Default::default()
            },
        )
        .await?;
        let product_items_truncated = product_items.len() > PLANNER_TREE_WORK_ITEM_LIMIT;
        product_items.truncate(PLANNER_TREE_WORK_ITEM_LIMIT);
        let product_areas = product_repo::list_product_areas(db, &product.id).await?;
        let capability_ids = product_items
            .iter()
            .filter_map(|item| item.capability_id.clone())
            .collect::<Vec<_>>();
        let mut capabilities_by_product_area = group_capabilities_by_product_area(
            product_repo::list_capabilities_by_ids_for_product(db, &product.id, &capability_ids)
                .await?,
        );
        let mut included = HashSet::new();
        let mut product_area_nodes = vec![];
        for product_area in product_areas {
            let mut children = vec![];
            let direct_items = product_items
                .iter()
                .filter(|item| {
                    item.product_area_id.as_deref() == Some(&product_area.id)
                        && item.capability_id.is_none()
                })
                .cloned()
                .collect::<Vec<_>>();
            if !direct_items.is_empty() {
                for item in &direct_items {
                    included.insert(item.id.clone());
                }
                children.push(PlannerTreeNode {
                    id: format!("{}-direct", product_area.id),
                    label: "Direct Work Items".to_string(),
                    meta: None,
                    node_type: Some("group".to_string()),
                    summary: None,
                    source: None,
                    confidence: None,
                    evidence: vec![],
                    children: build_tree_nodes_for_items(&direct_items, None),
                });
            }

            for capability in capabilities_by_product_area
                .remove(&product_area.id)
                .unwrap_or_default()
            {
                let capability_items = product_items
                    .iter()
                    .filter(|item| item.capability_id.as_deref() == Some(&capability.id))
                    .cloned()
                    .collect::<Vec<_>>();
                if capability_items.is_empty() {
                    continue;
                }
                for item in &capability_items {
                    included.insert(item.id.clone());
                }
                children.push(PlannerTreeNode {
                    id: capability.id.clone(),
                    label: capability.name.clone(),
                    meta: None,
                    node_type: Some("capability".to_string()),
                    summary: if capability.description.is_empty() {
                        None
                    } else {
                        Some(capability.description.clone())
                    },
                    source: None,
                    confidence: None,
                    evidence: vec![],
                    children: build_tree_nodes_for_items(&capability_items, None),
                });
            }

            product_area_nodes.push(PlannerTreeNode {
                id: product_area.id.clone(),
                label: product_area.name.clone(),
                meta: None,
                node_type: Some("product_area".to_string()),
                summary: if product_area.description.is_empty() {
                    None
                } else {
                    Some(product_area.description.clone())
                },
                source: None,
                confidence: None,
                evidence: vec![],
                children,
            });
        }

        let unscoped = product_items
            .iter()
            .filter(|item| !included.contains(&item.id) && item.parent_work_item_id.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if !unscoped.is_empty() {
            product_area_nodes.push(PlannerTreeNode {
                id: format!("{}-unscoped", product.id),
                label: "Unscoped".to_string(),
                meta: None,
                node_type: Some("group".to_string()),
                summary: None,
                source: None,
                confidence: None,
                evidence: vec![],
                children: build_tree_nodes_for_items(&unscoped, None),
            });
        }

        if product_items_truncated {
            product_area_nodes.push(PlannerTreeNode {
                id: format!("{}-work-items-truncated", product.id),
                label: format!(
                    "Work item context truncated at {} rows",
                    PLANNER_TREE_WORK_ITEM_LIMIT
                ),
                meta: Some("truncated".to_string()),
                node_type: Some("info".to_string()),
                summary: Some(
                    "Use paginated work-item tools for additional backlog rows.".to_string(),
                ),
                source: None,
                confidence: None,
                evidence: vec![],
                children: vec![],
            });
        }

        if product_area_nodes.is_empty() {
            product_area_nodes.push(PlannerTreeNode {
                id: format!("{}-empty", product.id),
                label: "No work items".to_string(),
                meta: Some("empty".to_string()),
                node_type: Some("info".to_string()),
                summary: None,
                source: None,
                confidence: None,
                evidence: vec![],
                children: vec![],
            });
        }

        nodes.push(PlannerTreeNode {
            id: product.id,
            label: product.name,
            meta: None,
            node_type: Some("product".to_string()),
            summary: if product.description.is_empty() {
                None
            } else {
                Some(product.description)
            },
            source: None,
            confidence: None,
            evidence: vec![],
            children: product_area_nodes,
        });
    }
    Ok(nodes)
}

fn group_capabilities_by_product_area(
    capabilities: Vec<Capability>,
) -> HashMap<String, Vec<Capability>> {
    let mut grouped = HashMap::new();
    for capability in capabilities {
        grouped
            .entry(capability.product_area_id.clone())
            .or_insert_with(Vec::new)
            .push(capability);
    }
    grouped
}

fn build_tree_nodes_for_items(items: &[WorkItem], parent_id: Option<&str>) -> Vec<PlannerTreeNode> {
    let mut filtered = items
        .iter()
        .filter(|item| item.parent_work_item_id.as_deref() == parent_id)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then(left.title.cmp(&right.title))
    });
    filtered
        .into_iter()
        .map(|item| PlannerTreeNode {
            id: item.id.clone(),
            label: item.title.clone(),
            meta: Some(item.status.to_string()),
            node_type: Some("work_item".to_string()),
            summary: if item.description.is_empty() {
                None
            } else {
                Some(item.description.clone())
            },
            source: None,
            confidence: None,
            evidence: vec![],
            children: build_tree_nodes_for_items(items, Some(&item.id)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db as db_service;

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_planner_catalog_{}_{}",
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

    async fn create_test_product(pool: &SqlitePool) {
        product_repo::create_product(
            pool,
            product_repo::CreateProductInput {
                id: "planner-product",
                name: "Planner Product",
                description: "Planner product summary",
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

    async fn create_test_product_area(pool: &SqlitePool) {
        product_repo::create_product_area(
            pool,
            product_repo::CreateProductAreaInput {
                id: "planner-area",
                product_id: "planner-product",
                name: "Planner Area",
                description: "Planner area summary",
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

    async fn create_test_capability(pool: &SqlitePool, id: &str, name: &str) {
        product_repo::create_capability(
            pool,
            product_repo::CreateCapabilityInput {
                id,
                product_area_id: "planner-area",
                parent_capability_id: None,
                name,
                description: "",
                acceptance_criteria: "",
                priority: "medium",
                risk: "low",
                technical_notes: "",
                node_kind: Some("capability"),
                explanation: "",
                examples: "",
                implementation_notes: "",
                test_guidance: "",
            },
        )
        .await
        .expect("capability should be created");
    }

    async fn insert_work_item(
        pool: &SqlitePool,
        id: &str,
        title: &str,
        product_area_id: Option<&str>,
        capability_id: Option<&str>,
        sort_order: i64,
    ) {
        sqlx::query(
            "INSERT INTO work_items (
                id, product_id, product_area_id, capability_id, title,
                work_item_type, priority, complexity, sort_order
             )
             VALUES (?, 'planner-product', ?, ?, ?, 'story', 'medium', 'medium', ?)",
        )
        .bind(id)
        .bind(product_area_id)
        .bind(capability_id)
        .bind(title)
        .bind(sort_order)
        .execute(pool)
        .await
        .expect("work item should be inserted");
    }

    fn collect_labels(nodes: &[PlannerTreeNode], labels: &mut Vec<String>) {
        for node in nodes {
            labels.push(node.label.clone());
            collect_labels(&node.children, labels);
        }
    }

    #[tokio::test]
    async fn build_tree_nodes_uses_bounded_items_and_targeted_capabilities() {
        let pool = create_test_pool("bounded_context").await;
        create_test_product(&pool).await;
        create_test_product_area(&pool).await;
        create_test_capability(&pool, "cap-used", "Used Capability").await;
        create_test_capability(&pool, "cap-unused", "Unused Capability").await;

        insert_work_item(
            &pool,
            "story-used-capability",
            "Story 000",
            Some("planner-area"),
            Some("cap-used"),
            0,
        )
        .await;
        insert_work_item(
            &pool,
            "story-direct-area",
            "Area Direct Story",
            Some("planner-area"),
            None,
            1,
        )
        .await;
        for index in 2..=499 {
            let id = format!("story-unscoped-{index:03}");
            let title = format!("Story {index:03}");
            insert_work_item(&pool, &id, &title, None, None, index).await;
        }
        insert_work_item(&pool, "story-outside-page", "Story 500", None, None, 500).await;

        let nodes = build_tree_nodes(&pool, Some("Planner Product"))
            .await
            .expect("planner tree nodes should build");

        let mut labels = Vec::new();
        collect_labels(&nodes, &mut labels);

        assert!(labels.iter().any(|label| label == "Planner Product"));
        assert!(labels.iter().any(|label| label == "Planner Area"));
        assert!(labels.iter().any(|label| label == "Direct Work Items"));
        assert!(labels.iter().any(|label| label == "Used Capability"));
        assert!(labels.iter().any(|label| label == "Story 000"));
        assert!(labels.iter().any(|label| label == "Area Direct Story"));
        assert!(labels
            .iter()
            .any(|label| label == "Work item context truncated at 500 rows"));
        assert!(!labels.iter().any(|label| label == "Unused Capability"));
        assert!(!labels.iter().any(|label| label == "Story 500"));
    }
}

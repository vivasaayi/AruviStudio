use crate::domain::product::{Capability, CapabilityTree, Product, ProductArea};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::services::planner_service::PlannerTreeNode;
use sqlx::SqlitePool;
use std::collections::HashSet;

const PLANNER_TREE_WORK_ITEM_LIMIT: usize = 500;

pub(crate) async fn find_product(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<Product, AppError> {
    let products = product_repo::list_products(db).await?;
    if let Some(name) = product_name {
        let normalized = normalize_name(Some(name));
        let exact = products
            .iter()
            .find(|product| normalize_name(Some(&product.name)) == normalized)
            .cloned();
        if let Some(product) = exact {
            return Ok(product);
        }
        let partial = products
            .into_iter()
            .filter(|product| normalize_name(Some(&product.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple products match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!("No product matches {}", name)));
    }
    if products.len() == 1 {
        return Ok(products[0].clone());
    }
    Err(AppError::Validation("Product is required".to_string()))
}

pub(crate) async fn find_product_area(
    db: &SqlitePool,
    product_name: Option<&str>,
    product_area_name: Option<&str>,
) -> Result<ProductArea, AppError> {
    let product = find_product(db, product_name).await?;
    let product_areas = product_repo::list_product_areas(db, &product.id).await?;
    if let Some(name) = product_area_name {
        let normalized = normalize_name(Some(name));
        let exact = product_areas
            .iter()
            .find(|product_area| normalize_name(Some(&product_area.name)) == normalized)
            .cloned();
        if let Some(product_area) = exact {
            return Ok(product_area);
        }
        let partial = product_areas
            .into_iter()
            .filter(|product_area| normalize_name(Some(&product_area.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple product_areas match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!(
            "No product_area matches {}",
            name
        )));
    }
    if product_areas.len() == 1 {
        return Ok(product_areas[0].clone());
    }
    Err(AppError::Validation("Product Area is required".to_string()))
}

pub(crate) async fn find_capability(
    db: &SqlitePool,
    product_name: Option<&str>,
    product_area_name: Option<&str>,
    capability_name: Option<&str>,
) -> Result<Capability, AppError> {
    let product_area = find_product_area(db, product_name, product_area_name).await?;
    let capabilities = product_repo::list_capabilities(db, &product_area.id).await?;
    if let Some(name) = capability_name {
        let normalized = normalize_name(Some(name));
        let exact = capabilities
            .iter()
            .find(|capability| normalize_name(Some(&capability.name)) == normalized)
            .cloned();
        if let Some(capability) = exact {
            return Ok(capability);
        }
        let partial = capabilities
            .into_iter()
            .filter(|capability| normalize_name(Some(&capability.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple capabilities match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!(
            "No capability matches {}",
            name
        )));
    }
    Err(AppError::Validation("Capability is required".to_string()))
}

pub(crate) async fn find_work_item(
    db: &SqlitePool,
    work_item_title: Option<&str>,
    product_name: Option<&str>,
) -> Result<WorkItem, AppError> {
    let product_id = if let Some(name) = product_name {
        Some(find_product(db, Some(name)).await?.id)
    } else {
        None
    };
    if let Some(title) = work_item_title {
        let normalized = normalize_name(Some(title));
        let work_items =
            work_item_repo::search_work_items_by_title(db, product_id.as_deref(), title, 2).await?;
        let exact = work_items
            .iter()
            .find(|work_item| normalize_name(Some(&work_item.title)) == normalized)
            .cloned();
        if let Some(work_item) = exact {
            return Ok(work_item);
        }
        let partial = work_items
            .into_iter()
            .filter(|work_item| normalize_name(Some(&work_item.title)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple work items match {}",
                title
            )));
        }
        return Err(AppError::NotFound(format!(
            "No work item matches {}",
            title
        )));
    }
    Err(AppError::Validation("Work item is required".to_string()))
}

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
        let tree = product_repo::get_product_tree(db, &product.id).await?;
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
        let mut included = HashSet::new();
        let mut product_area_nodes = vec![];
        for product_area_tree in tree.product_areas {
            let mut children = vec![];
            let direct_items = product_items
                .iter()
                .filter(|item| {
                    item.product_area_id.as_deref() == Some(&product_area_tree.product_area.id)
                        && item.capability_id.is_none()
                })
                .cloned()
                .collect::<Vec<_>>();
            if !direct_items.is_empty() {
                for item in &direct_items {
                    included.insert(item.id.clone());
                }
                children.push(PlannerTreeNode {
                    id: format!("{}-direct", product_area_tree.product_area.id),
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

            let mut flattened = vec![];
            flatten_capabilities(&product_area_tree.features, &mut flattened);
            for capability in flattened {
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
                id: product_area_tree.product_area.id.clone(),
                label: product_area_tree.product_area.name.clone(),
                meta: None,
                node_type: Some("product_area".to_string()),
                summary: if product_area_tree.product_area.description.is_empty() {
                    None
                } else {
                    Some(product_area_tree.product_area.description.clone())
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

fn normalize_name(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

fn flatten_capabilities(nodes: &[CapabilityTree], bucket: &mut Vec<Capability>) {
    for node in nodes {
        bucket.push(node.capability.clone());
        flatten_capabilities(&node.children, bucket);
    }
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

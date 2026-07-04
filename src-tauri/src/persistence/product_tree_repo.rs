use crate::domain::product::{
    Capability, CapabilityTree, HierarchyNodeType, HierarchyTreeNode, ProductAreaTree, ProductTree,
    ProductTreeSummary,
};
use crate::error::AppError;
use crate::observability::performance::{
    elapsed_ms, record_persistence_query, record_persistence_query_error,
};
use crate::persistence::product_repo;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::time::Instant;
use tracing::trace;

const CAPABILITY_TREE_SELECT_COLUMNS: &str = "
    c.id AS id,
    c.product_area_id AS product_area_id,
    c.parent_capability_id AS parent_capability_id,
    c.level AS level,
    c.node_kind AS node_kind,
    c.sort_order AS sort_order,
    c.name AS name,
    c.description AS description,
    c.acceptance_criteria AS acceptance_criteria,
    c.explanation AS explanation,
    c.examples AS examples,
    c.priority AS priority,
    c.risk AS risk,
    c.status AS status,
    c.technical_notes AS technical_notes,
    c.implementation_notes AS implementation_notes,
    c.test_guidance AS test_guidance,
    c.created_at AS created_at,
    c.updated_at AS updated_at";

pub async fn get_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTree, AppError> {
    let started = Instant::now();
    let result = get_product_tree_impl(pool, product_id).await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(tree) => record_persistence_query(
            "products.get_tree",
            duration_ms,
            Some(product_tree_node_count(tree)),
        ),
        Err(error) => record_persistence_query_error("products.get_tree", duration_ms, error),
    }
    result
}

async fn get_product_tree_impl(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTree, AppError> {
    trace!(product_id = %product_id, "persist get_product_tree");
    let product = product_repo::get_product(pool, product_id).await?;
    let product_areas = product_repo::list_product_areas(pool, product_id).await?;
    let capabilities_by_product_area =
        group_capabilities_by_product_area(list_capabilities_for_product(pool, product_id).await?);
    let mut product_area_trees = Vec::new();
    for product_area in product_areas {
        let capability_trees = build_capability_trees(
            capabilities_by_product_area
                .get(&product_area.id)
                .cloned()
                .unwrap_or_default(),
        );
        product_area_trees.push(ProductAreaTree {
            product_area,
            features: capability_trees,
        });
    }
    let roots = product_area_trees
        .iter()
        .map(build_product_area_hierarchy_tree)
        .collect();
    Ok(ProductTree {
        product,
        product_areas: product_area_trees,
        roots,
    })
}

async fn list_capabilities_for_product(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<Capability>, AppError> {
    let started = Instant::now();
    let result = sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_TREE_SELECT_COLUMNS}
         FROM capabilities c
         JOIN product_areas pa ON pa.id = c.product_area_id
         WHERE pa.product_id = ?
         ORDER BY pa.sort_order, c.product_area_id, c.sort_order, c.name"
    ))
    .bind(product_id)
    .fetch_all(pool)
    .await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(rows) => record_persistence_query(
            "products.list_capabilities_for_tree",
            duration_ms,
            Some(rows.len()),
        ),
        Err(error) => record_persistence_query_error(
            "products.list_capabilities_for_tree",
            duration_ms,
            error,
        ),
    }
    result.map_err(|error| error.into())
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

fn build_capability_trees(capabilities: Vec<Capability>) -> Vec<CapabilityTree> {
    let mut children_by_parent = group_capabilities_by_parent(capabilities);
    children_by_parent
        .remove(&None)
        .unwrap_or_default()
        .into_iter()
        .map(|capability| build_capability_tree(capability, &mut children_by_parent))
        .collect()
}

fn group_capabilities_by_parent(
    capabilities: Vec<Capability>,
) -> HashMap<Option<String>, Vec<Capability>> {
    let mut grouped = HashMap::new();
    for capability in capabilities {
        grouped
            .entry(capability.parent_capability_id.clone())
            .or_insert_with(Vec::new)
            .push(capability);
    }
    grouped
}

fn build_capability_tree(
    capability: Capability,
    children_by_parent: &mut HashMap<Option<String>, Vec<Capability>>,
) -> CapabilityTree {
    let children = children_by_parent
        .remove(&Some(capability.id.clone()))
        .unwrap_or_default()
        .into_iter()
        .map(|child| build_capability_tree(child, children_by_parent))
        .collect();
    CapabilityTree {
        capability,
        children,
    }
}

pub async fn summarize_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTreeSummary, AppError> {
    let started = Instant::now();
    let result = summarize_product_tree_impl(pool, product_id).await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(summary) => record_persistence_query(
            "products.summarize_tree",
            duration_ms,
            usize::try_from(summary.total_node_count).ok(),
        ),
        Err(error) => record_persistence_query_error("products.summarize_tree", duration_ms, error),
    }
    result
}

async fn summarize_product_tree_impl(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTreeSummary, AppError> {
    product_repo::get_product(pool, product_id).await?;

    let product_area_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM product_areas WHERE product_id = ?")
            .bind(product_id)
            .fetch_one(pool)
            .await?;
    let capability_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM capabilities c
         JOIN product_areas pa ON pa.id = c.product_area_id
         WHERE pa.product_id = ?",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;
    let product_area_leaf_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM product_areas pa
         WHERE pa.product_id = ?
           AND NOT EXISTS (
             SELECT 1 FROM capabilities c WHERE c.product_area_id = pa.id
           )",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;
    let capability_leaf_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM capabilities c
         JOIN product_areas pa ON pa.id = c.product_area_id
         WHERE pa.product_id = ?
           AND NOT EXISTS (
             SELECT 1 FROM capabilities child WHERE child.parent_capability_id = c.id
           )",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;

    Ok(ProductTreeSummary {
        product_id: product_id.to_string(),
        product_area_count,
        capability_count,
        total_node_count: product_area_count + capability_count,
        leaf_node_count: product_area_leaf_count + capability_leaf_count,
    })
}

fn product_tree_node_count(tree: &ProductTree) -> usize {
    tree.product_areas.len()
        + tree
            .product_areas
            .iter()
            .flat_map(|product_area_tree| product_area_tree.features.iter())
            .map(capability_tree_node_count)
            .sum::<usize>()
}

fn capability_tree_node_count(tree: &CapabilityTree) -> usize {
    1 + tree
        .children
        .iter()
        .map(capability_tree_node_count)
        .sum::<usize>()
}

fn build_product_area_hierarchy_tree(product_area_tree: &ProductAreaTree) -> HierarchyTreeNode {
    let path = vec![product_area_tree.product_area.name.clone()];
    let children = product_area_tree
        .features
        .iter()
        .map(|capability_tree| build_hierarchy_tree(capability_tree, &path))
        .collect();
    HierarchyTreeNode {
        id: product_area_tree.product_area.id.clone(),
        node_type: HierarchyNodeType::ProductArea,
        node_kind: product_area_tree.product_area.node_kind,
        product_area_id: product_area_tree.product_area.id.clone(),
        capability_id: None,
        parent_node_id: None,
        parent_node_type: None,
        depth: 0,
        name: product_area_tree.product_area.name.clone(),
        description: product_area_tree.product_area.description.clone(),
        summary: first_non_empty(&[
            &product_area_tree.product_area.description,
            &product_area_tree.product_area.explanation,
            &product_area_tree.product_area.purpose,
            &product_area_tree.product_area.implementation_notes,
            &product_area_tree.product_area.test_guidance,
        ]),
        path,
        allowed_child_kinds: product_area_tree
            .product_area
            .node_kind
            .allowed_child_kinds(),
        children,
    }
}

fn build_hierarchy_tree(
    capability_tree: &CapabilityTree,
    parent_path: &[String],
) -> HierarchyTreeNode {
    let mut path = parent_path.to_vec();
    path.push(capability_tree.capability.name.clone());
    let children = capability_tree
        .children
        .iter()
        .map(|child| build_hierarchy_tree(child, &path))
        .collect();
    HierarchyTreeNode {
        id: capability_tree.capability.id.clone(),
        node_type: HierarchyNodeType::Capability,
        node_kind: capability_tree.capability.node_kind,
        product_area_id: capability_tree.capability.product_area_id.clone(),
        capability_id: Some(capability_tree.capability.id.clone()),
        parent_node_id: capability_tree
            .capability
            .parent_capability_id
            .clone()
            .or_else(|| {
                parent_path
                    .first()
                    .map(|_| capability_tree.capability.product_area_id.clone())
            }),
        parent_node_type: Some(
            if capability_tree.capability.parent_capability_id.is_some() {
                HierarchyNodeType::Capability
            } else {
                HierarchyNodeType::ProductArea
            },
        ),
        depth: capability_tree.capability.level + 1,
        name: capability_tree.capability.name.clone(),
        description: capability_tree.capability.description.clone(),
        summary: first_non_empty(&[
            &capability_tree.capability.description,
            &capability_tree.capability.explanation,
            &capability_tree.capability.acceptance_criteria,
            &capability_tree.capability.technical_notes,
            &capability_tree.capability.examples,
            &capability_tree.capability.implementation_notes,
            &capability_tree.capability.test_guidance,
        ]),
        path,
        allowed_child_kinds: capability_tree.capability.node_kind.allowed_child_kinds(),
        children,
    }
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

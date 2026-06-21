use crate::domain::product::{
    Capability, CapabilityTree, HierarchyNodeType, HierarchyTreeNode, ProductAreaTree, ProductTree,
    ProductTreeSummary,
};
use crate::error::AppError;
use crate::persistence::product_repo;
use sqlx::SqlitePool;
use tracing::trace;

pub async fn get_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTree, AppError> {
    trace!(product_id = %product_id, "persist get_product_tree");
    let product = product_repo::get_product(pool, product_id).await?;
    let product_areas = product_repo::list_product_areas(pool, product_id).await?;
    let mut product_area_trees = Vec::new();
    for product_area in product_areas {
        let features = product_repo::list_capabilities(pool, &product_area.id).await?;
        let root_features: Vec<_> = features
            .iter()
            .filter(|feature| feature.parent_capability_id.is_none())
            .collect();
        let capability_trees = root_features
            .iter()
            .map(|feature| build_capability_tree(feature, &features))
            .collect();
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

pub async fn summarize_product_tree(
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

fn build_capability_tree(
    capability: &Capability,
    all_capabilities: &[Capability],
) -> CapabilityTree {
    let children: Vec<_> = all_capabilities
        .iter()
        .filter(|feature| feature.parent_capability_id.as_deref() == Some(&capability.id))
        .map(|feature| build_capability_tree(feature, all_capabilities))
        .collect();
    CapabilityTree {
        capability: capability.clone(),
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

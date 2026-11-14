use crate::domain::product::{HierarchyNodeKind, HierarchyNodeType, HierarchyTreeNode, ProductReference, ProductTree};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_tree_repo, work_item_read_repo};
use serde::Serialize;
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashSet;

#[derive(Debug, Serialize)]
pub struct ProductBookSource {
    pub tree: ProductTree,
    pub work_items: Vec<WorkItem>,
    pub references: Vec<ProductReference>,
}

/// All reads share one SQLite snapshot. No result is returned if any read fails.
pub async fn get_product_book_source(pool: &SqlitePool, product_id: &str) -> Result<ProductBookSource, AppError> {
    if product_id.trim().is_empty() {
        return Err(AppError::Validation("A product ID is required".into()));
    }
    let mut tx = pool.begin().await?;
    let result = read_product_book_source(&mut tx, product_id).await?;
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn read_product_book_source(conn: &mut SqliteConnection, product_id: &str) -> Result<ProductBookSource, AppError> {
    let tree = product_tree_repo::get_product_tree_in_conn(conn, product_id).await?;
    let work_items = work_item_read_repo::list_product_export_work_items_in_conn(conn, product_id).await?;
    // Narrow the database read to this product before applying the same reachable
    // hierarchy and node-kind rules as the desktop renderer.
    let candidates = sqlx::query_as::<_, ProductReference>(
        "SELECT id, scope_type, scope_id, title, reference_kind, uri, content, created_at, updated_at
         FROM \"references\" r WHERE
         (r.scope_type = 'product' AND r.scope_id = ?)
         OR (r.scope_type = 'product_area' AND EXISTS (SELECT 1 FROM product_areas a WHERE a.id = r.scope_id AND a.product_id = ?))
         OR (r.scope_type IN ('capability', 'feature') AND EXISTS (SELECT 1 FROM capabilities c JOIN product_areas a ON a.id = c.product_area_id WHERE c.id = r.scope_id AND a.product_id = ?))
         ORDER BY updated_at DESC, created_at DESC"
    ).bind(product_id).bind(product_id).bind(product_id).fetch_all(&mut *conn).await?;
    let mut scopes = HashSet::new();
    scopes.insert(("product", product_id.to_owned()));
    for node in &tree.roots { collect_scopes(node, &mut scopes); }
    let references = candidates.into_iter().filter(|r| scopes.contains(&(r.scope_type.as_str(), r.scope_id.clone()))).collect();
    Ok(ProductBookSource { tree, work_items, references })
}

fn collect_scopes(node: &HierarchyTreeNode, scopes: &mut HashSet<(&'static str, String)>) {
    if node.node_type == HierarchyNodeType::ProductArea {
        scopes.insert(("product_area", node.id.clone()));
    } else if let Some(id) = &node.capability_id {
        scopes.insert((if node.node_kind == HierarchyNodeKind::Feature { "feature" } else { "capability" }, id.clone()));
    }
    for child in &node.children { collect_scopes(child, scopes); }
}

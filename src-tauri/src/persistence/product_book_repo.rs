use crate::domain::product::{HierarchyNodeKind, HierarchyNodeType, HierarchyTreeNode, ProductReference, ProductTree};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_tree_repo, work_item_read_repo};
use serde::Serialize;
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::HashSet;

// Count raw database values before loading a snapshot. This is a conservative
// budget, not an estimate of the final JSON or a process memory limit.
const MAX_SOURCE_BYTES: i64 = 64 * 1024 * 1024;
const MAX_SOURCE_ROWS: i64 = 100_000;

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
    check_snapshot_budget(conn, product_id, MAX_SOURCE_BYTES, MAX_SOURCE_ROWS).await?;
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

async fn check_snapshot_budget(conn: &mut SqliteConnection, product_id: &str, byte_limit: i64, row_limit: i64) -> Result<(), AppError> {
    // Include all columns, even those not currently rendered, so future columns
    // cannot silently bypass the bound. Reference candidates use the same scope
    // as the later query and can therefore conservatively include unreachable nodes.
    let tables = [
        ("products", "s.id = ?", 1),
        ("product_areas", "s.product_id = ?", 1),
        ("capabilities", "EXISTS (SELECT 1 FROM product_areas a WHERE a.id = s.product_area_id AND a.product_id = ?)", 1),
        ("work_items", "s.product_id = ?", 1),
        ("references", "(s.scope_type = 'product' AND s.scope_id = ?) OR (s.scope_type = 'product_area' AND EXISTS (SELECT 1 FROM product_areas a WHERE a.id = s.scope_id AND a.product_id = ?)) OR (s.scope_type IN ('capability', 'feature') AND EXISTS (SELECT 1 FROM capabilities c JOIN product_areas a ON a.id = c.product_area_id WHERE c.id = s.scope_id AND a.product_id = ?))", 3),
    ];
    let mut total_bytes = 0_i64;
    let mut total_rows = 0_i64;
    for (table, filter, parameters) in tables {
        let columns = sqlx::query(&format!("PRAGMA table_info(\"{table}\")")).fetch_all(&mut *conn).await?;
        let lengths: Vec<String> = columns.iter().map(|row| {
            let name: String = row.get("name");
            format!("COALESCE(length(CAST(s.\"{}\" AS BLOB)), 0)", name.replace('"', "\"\""))
        }).collect();
        if lengths.is_empty() { return Err(AppError::Internal("Product Book source table unavailable".into())); }
        let sql = format!("SELECT count(*) AS rows, COALESCE(sum({}), 0) AS bytes FROM \"{table}\" s WHERE {filter}", lengths.join(" + "));
        let mut query = sqlx::query(&sql);
        for _ in 0..parameters { query = query.bind(product_id); }
        let row = query.fetch_one(&mut *conn).await?;
        total_rows = total_rows.checked_add(row.try_get::<i64, _>("rows")?).ok_or_else(snapshot_limit_error)?;
        total_bytes = total_bytes.checked_add(row.try_get::<i64, _>("bytes")?).ok_or_else(snapshot_limit_error)?;
        if total_rows > row_limit || total_bytes > byte_limit { return Err(snapshot_limit_error()); }
    }
    Ok(())
}

fn snapshot_limit_error() -> AppError {
    AppError::Validation("Product Book source exceeds the snapshot size limit".into())
}

#[cfg(test)]
pub(crate) async fn check_snapshot_budget_for_test(conn: &mut SqliteConnection, product_id: &str, bytes: i64, rows: i64) -> Result<(), AppError> {
    check_snapshot_budget(conn, product_id, bytes, rows).await
}

fn collect_scopes(node: &HierarchyTreeNode, scopes: &mut HashSet<(&'static str, String)>) {
    if node.node_type == HierarchyNodeType::ProductArea {
        scopes.insert(("product_area", node.id.clone()));
    } else if let Some(id) = &node.capability_id {
        scopes.insert((if node.node_kind == HierarchyNodeKind::Feature { "feature" } else { "capability" }, id.clone()));
    }
    for child in &node.children { collect_scopes(child, scopes); }
}

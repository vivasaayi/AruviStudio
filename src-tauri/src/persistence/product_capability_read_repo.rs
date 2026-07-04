use crate::domain::product::Capability;
use crate::error::AppError;
use crate::observability::performance::{
    elapsed_ms, record_persistence_query, record_persistence_query_error,
};
use sqlx::SqlitePool;
use std::time::Instant;

pub(crate) const CAPABILITY_SELECT_COLUMNS: &str = "id, product_area_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, explanation, examples, priority, risk, status, technical_notes, implementation_notes, test_guidance, created_at, updated_at";
const QUALIFIED_CAPABILITY_SELECT_COLUMNS: &str = "c.id AS id, c.product_area_id AS product_area_id, c.parent_capability_id AS parent_capability_id, c.level AS level, c.node_kind AS node_kind, c.sort_order AS sort_order, c.name AS name, c.description AS description, c.acceptance_criteria AS acceptance_criteria, c.explanation AS explanation, c.examples AS examples, c.priority AS priority, c.risk AS risk, c.status AS status, c.technical_notes AS technical_notes, c.implementation_notes AS implementation_notes, c.test_guidance AS test_guidance, c.created_at AS created_at, c.updated_at AS updated_at";

pub async fn list_capabilities(
    pool: &SqlitePool,
    product_area_id: &str,
) -> Result<Vec<Capability>, AppError> {
    let started = Instant::now();
    let result = sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_SELECT_COLUMNS} FROM capabilities WHERE product_area_id=? ORDER BY sort_order, name"
    ))
        .bind(product_area_id)
        .fetch_all(pool).await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(rows) => {
            record_persistence_query("products.list_capabilities", duration_ms, Some(rows.len()))
        }
        Err(error) => {
            record_persistence_query_error("products.list_capabilities", duration_ms, error)
        }
    }
    result.map_err(|error| error.into())
}

pub async fn list_product_capabilities(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<Capability>, AppError> {
    let started = Instant::now();
    let result = sqlx::query_as::<_, Capability>(&format!(
        "WITH RECURSIVE capability_tree AS (
            SELECT
                c.id,
                c.product_area_id,
                c.parent_capability_id,
                c.level,
                c.node_kind,
                c.sort_order,
                c.name,
                c.description,
                c.acceptance_criteria,
                c.explanation,
                c.examples,
                c.priority,
                c.risk,
                c.status,
                c.technical_notes,
                c.implementation_notes,
                c.test_guidance,
                c.created_at,
                c.updated_at,
                pa.sort_order AS product_area_sort_order,
                printf('%08d:%s', c.sort_order, c.name) AS tree_path
            FROM capabilities c
            JOIN product_areas pa ON pa.id = c.product_area_id
            WHERE pa.product_id = ? AND c.parent_capability_id IS NULL
            UNION ALL
            SELECT
                child.id,
                child.product_area_id,
                child.parent_capability_id,
                child.level,
                child.node_kind,
                child.sort_order,
                child.name,
                child.description,
                child.acceptance_criteria,
                child.explanation,
                child.examples,
                child.priority,
                child.risk,
                child.status,
                child.technical_notes,
                child.implementation_notes,
                child.test_guidance,
                child.created_at,
                child.updated_at,
                parent.product_area_sort_order,
                parent.tree_path || '/' || printf('%08d:%s', child.sort_order, child.name) AS tree_path
            FROM capabilities child
            JOIN capability_tree parent ON parent.id = child.parent_capability_id
        )
        SELECT {CAPABILITY_SELECT_COLUMNS}
        FROM capability_tree
        ORDER BY product_area_sort_order, product_area_id, tree_path"
    ))
    .bind(product_id)
    .fetch_all(pool)
    .await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(rows) => record_persistence_query(
            "products.list_product_capabilities",
            duration_ms,
            Some(rows.len()),
        ),
        Err(error) => {
            record_persistence_query_error("products.list_product_capabilities", duration_ms, error)
        }
    }
    result.map_err(|error| error.into())
}

pub async fn list_capabilities_by_ids_for_product(
    pool: &SqlitePool,
    product_id: &str,
    capability_ids: &[String],
) -> Result<Vec<Capability>, AppError> {
    let mut ids = capability_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT {QUALIFIED_CAPABILITY_SELECT_COLUMNS}
         FROM capabilities c
         JOIN product_areas pa ON pa.id = c.product_area_id
         WHERE pa.product_id = ? AND c.id IN ({placeholders})
         ORDER BY pa.sort_order, c.product_area_id, c.sort_order, c.name"
    );
    let mut query = sqlx::query_as::<_, Capability>(&sql).bind(product_id);
    for id in ids {
        query = query.bind(id);
    }
    let started = Instant::now();
    let result = query.fetch_all(pool).await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(rows) => record_persistence_query(
            "products.list_capabilities_by_ids",
            duration_ms,
            Some(rows.len()),
        ),
        Err(error) => {
            record_persistence_query_error("products.list_capabilities_by_ids", duration_ms, error)
        }
    }
    result.map_err(|error| error.into())
}

pub async fn get_capability(pool: &SqlitePool, id: &str) -> Result<Capability, AppError> {
    let started = Instant::now();
    let result = sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_SELECT_COLUMNS} FROM capabilities WHERE id=?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await;
    let duration_ms = elapsed_ms(started);
    match &result {
        Ok(capability) => record_persistence_query(
            "products.get_capability",
            duration_ms,
            Some(usize::from(capability.is_some())),
        ),
        Err(error) => record_persistence_query_error("products.get_capability", duration_ms, error),
    }
    result?.ok_or_else(|| AppError::NotFound(format!("Capability {id} not found")))
}

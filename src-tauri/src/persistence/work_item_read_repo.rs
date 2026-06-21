use crate::domain::work_item::{ProductWorkItemSummary, WorkItem, WorkItemScopeSummary};
use crate::error::AppError;
use sqlx::SqlitePool;
use tracing::trace;

const DEFAULT_LIST_WORK_ITEMS_LIMIT: i64 = 500;
const MAX_LIST_WORK_ITEMS_LIMIT: i64 = 2_000;
const WORK_ITEM_SELECT_COLUMNS: &str = "id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at";

#[derive(Clone, Copy, Debug, Default)]
pub struct WorkItemListQuery<'a> {
    pub product_id: Option<&'a str>,
    pub product_area_id: Option<&'a str>,
    pub capability_id: Option<&'a str>,
    pub source_node_id: Option<&'a str>,
    pub source_node_type: Option<&'a str>,
    pub status: Option<&'a str>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl WorkItemListQuery<'_> {
    fn bounded_page(&self) -> (i64, i64) {
        let limit = self
            .limit
            .unwrap_or(DEFAULT_LIST_WORK_ITEMS_LIMIT)
            .clamp(1, MAX_LIST_WORK_ITEMS_LIMIT);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}

pub async fn get_work_item(pool: &SqlitePool, id: &str) -> Result<WorkItem, AppError> {
    sqlx::query_as::<_, WorkItem>(&format!(
        "SELECT {WORK_ITEM_SELECT_COLUMNS} FROM work_items WHERE id=?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Work item {id} not found")))
}

pub async fn list_work_items_page(
    pool: &SqlitePool,
    query: WorkItemListQuery<'_>,
) -> Result<Vec<WorkItem>, AppError> {
    list_work_items_internal(pool, query, false).await
}

pub async fn list_top_level_work_items_page(
    pool: &SqlitePool,
    query: WorkItemListQuery<'_>,
) -> Result<Vec<WorkItem>, AppError> {
    list_work_items_internal(pool, query, true).await
}

pub async fn search_work_items_by_title(
    pool: &SqlitePool,
    product_id: Option<&str>,
    title: &str,
    limit: i64,
) -> Result<Vec<WorkItem>, AppError> {
    let normalized = title.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.clamp(1, 100);
    let mut query = format!(
        "SELECT {WORK_ITEM_SELECT_COLUMNS} FROM work_items
         WHERE instr(lower(trim(title)), ?) > 0"
    );
    if product_id.is_some() {
        query.push_str(" AND product_id = ?");
    }
    query.push_str(
        " ORDER BY CASE WHEN lower(trim(title)) = ? THEN 0 ELSE 1 END,
                  sort_order, created_at DESC
          LIMIT ?",
    );

    let mut query_builder = sqlx::query_as::<_, WorkItem>(&query).bind(&normalized);
    if let Some(product_id) = product_id {
        query_builder = query_builder.bind(product_id);
    }
    query_builder
        .bind(&normalized)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|error| error.into())
}

pub async fn summarize_work_items_by_product(
    pool: &SqlitePool,
) -> Result<Vec<ProductWorkItemSummary>, AppError> {
    sqlx::query_as::<_, ProductWorkItemSummary>(
        "SELECT product_id, COUNT(*) as total_count,
         SUM(CASE WHEN status NOT IN ('done', 'cancelled') THEN 1 ELSE 0 END) as active_count,
         SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) as done_count,
         SUM(CASE WHEN status IN ('blocked', 'failed') THEN 1 ELSE 0 END) as blocked_count
         FROM work_items
         WHERE product_id IS NOT NULL
         GROUP BY product_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.into())
}

pub async fn summarize_work_items_by_scope(
    pool: &SqlitePool,
    product_id: Option<&str>,
) -> Result<Vec<WorkItemScopeSummary>, AppError> {
    let mut query = String::from(
        "SELECT product_id,
                product_area_id,
                capability_id,
                source_node_id,
                source_node_type,
                status,
                COUNT(*) as total_count,
                SUM(CASE WHEN parent_work_item_id IS NULL THEN 1 ELSE 0 END) as top_level_count,
                SUM(CASE WHEN status NOT IN ('done', 'cancelled') THEN 1 ELSE 0 END) as active_count,
                SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) as done_count,
                SUM(CASE WHEN status IN ('blocked', 'failed') THEN 1 ELSE 0 END) as blocked_count
         FROM work_items
         WHERE product_id IS NOT NULL",
    );
    if product_id.is_some() {
        query.push_str(" AND product_id = ?");
    }
    query.push_str(
        " GROUP BY product_id, product_area_id, capability_id, source_node_id, source_node_type, status",
    );

    let mut query_builder = sqlx::query_as::<_, WorkItemScopeSummary>(&query);
    if let Some(product_id) = product_id {
        query_builder = query_builder.bind(product_id);
    }
    query_builder
        .fetch_all(pool)
        .await
        .map_err(|error| error.into())
}

pub async fn get_sub_work_items_page(
    pool: &SqlitePool,
    parent_work_item_id: &str,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<WorkItem>, AppError> {
    let limit = limit
        .unwrap_or(DEFAULT_LIST_WORK_ITEMS_LIMIT)
        .clamp(1, MAX_LIST_WORK_ITEMS_LIMIT);
    let offset = offset.unwrap_or(0).max(0);
    trace!(parent_work_item_id = %parent_work_item_id, limit, offset, "persist get_sub_work_items_page");
    sqlx::query_as::<_, WorkItem>(&format!(
        "SELECT {WORK_ITEM_SELECT_COLUMNS}
         FROM work_items
         WHERE parent_work_item_id=?
         ORDER BY sort_order
         LIMIT ? OFFSET ?"
    ))
    .bind(parent_work_item_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|error| error.into())
}

async fn list_work_items_internal(
    pool: &SqlitePool,
    query: WorkItemListQuery<'_>,
    top_level_only: bool,
) -> Result<Vec<WorkItem>, AppError> {
    let normalized_source_node_type = query
        .source_node_type
        .map(normalize_source_node_type)
        .transpose()?;
    let (limit, offset) = query.bounded_page();
    trace!(product_id = ?query.product_id, product_area_id = ?query.product_area_id, capability_id = ?query.capability_id, source_node_id = ?query.source_node_id, source_node_type = ?query.source_node_type, status = ?query.status, limit, offset, "persist list_work_items");
    let mut sql = format!("SELECT {WORK_ITEM_SELECT_COLUMNS} FROM work_items WHERE 1=1");
    if query.product_id.is_some() {
        sql.push_str(" AND product_id = ?");
    }
    if query.product_area_id.is_some() {
        sql.push_str(" AND product_area_id = ?");
    }
    if query.capability_id.is_some() {
        sql.push_str(" AND capability_id = ?");
    }
    if query.source_node_id.is_some() {
        sql.push_str(" AND source_node_id = ?");
    }
    if normalized_source_node_type.is_some() {
        sql.push_str(" AND source_node_type = ?");
    }
    if query.status.is_some() {
        sql.push_str(" AND status = ?");
    }
    if top_level_only {
        sql.push_str(" AND parent_work_item_id IS NULL");
    }
    sql.push_str(" ORDER BY sort_order, created_at DESC");
    sql.push_str(" LIMIT ? OFFSET ?");

    let mut query_builder = sqlx::query_as::<_, WorkItem>(&sql);
    if let Some(value) = query.product_id {
        query_builder = query_builder.bind(value);
    }
    if let Some(value) = query.product_area_id {
        query_builder = query_builder.bind(value);
    }
    if let Some(value) = query.capability_id {
        query_builder = query_builder.bind(value);
    }
    if let Some(value) = query.source_node_id {
        query_builder = query_builder.bind(value);
    }
    if let Some(value) = normalized_source_node_type {
        query_builder = query_builder.bind(value);
    }
    if let Some(value) = query.status {
        query_builder = query_builder.bind(value);
    }
    query_builder = query_builder.bind(limit).bind(offset);
    query_builder
        .fetch_all(pool)
        .await
        .map_err(|error| error.into())
}

fn normalize_source_node_type(value: &str) -> Result<&'static str, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "product_area" => Ok("product_area"),
        "capability" | "feature" => Ok("capability"),
        _ => Err(AppError::Validation(format!(
            "Unsupported source node type '{value}'. Use product_area, capability, or feature."
        ))),
    }
}

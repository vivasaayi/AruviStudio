use crate::domain::product::{Capability, ProductArea};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use sqlx::SqlitePool;

const MCP_PRODUCT_AREA_SELECT_COLUMNS: &str = "id, product_id, node_kind, name, description, purpose, explanation, examples, implementation_notes, test_guidance, sort_order, created_at, updated_at";
const MCP_CAPABILITY_SELECT_COLUMNS: &str = "id, product_area_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, explanation, examples, priority, risk, status, technical_notes, implementation_notes, test_guidance, created_at, updated_at";

pub(super) async fn get_product_area_by_id(
    pool: &SqlitePool,
    product_area_id: &str,
) -> Result<ProductArea, AppError> {
    sqlx::query_as::<_, ProductArea>(&format!(
        "SELECT {MCP_PRODUCT_AREA_SELECT_COLUMNS} FROM product_areas WHERE id=?"
    ))
    .bind(product_area_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Product Area {product_area_id} not found")))
}

pub(super) async fn list_capability_children(
    pool: &SqlitePool,
    product_area_id: &str,
    parent_capability_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Capability>, AppError> {
    let limit = limit.clamp(1, 500);
    if let Some(parent_id) = parent_capability_id {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE product_area_id=? AND parent_capability_id=?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(product_area_id)
        .bind(parent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE product_area_id=? AND parent_capability_id IS NULL
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(product_area_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

pub(super) async fn list_capability_siblings(
    pool: &SqlitePool,
    capability: &Capability,
    limit: i64,
) -> Result<Vec<Capability>, AppError> {
    let limit = limit.clamp(1, 100);
    if let Some(parent_id) = capability.parent_capability_id.as_deref() {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE product_area_id=? AND parent_capability_id=? AND id<>?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(&capability.product_area_id)
        .bind(parent_id)
        .bind(&capability.id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, Capability>(&format!(
            "SELECT {MCP_CAPABILITY_SELECT_COLUMNS}
             FROM capabilities
             WHERE product_area_id=? AND parent_capability_id IS NULL AND id<>?
             ORDER BY sort_order, name
             LIMIT ?"
        ))
        .bind(&capability.product_area_id)
        .bind(&capability.id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

pub(super) async fn capability_ancestors(
    pool: &SqlitePool,
    capability: &Capability,
) -> Result<Vec<Capability>, AppError> {
    let mut ancestors = Vec::new();
    let mut parent_id = capability.parent_capability_id.clone();
    while let Some(id) = parent_id {
        let parent = product_repo::get_capability(pool, &id).await?;
        parent_id = parent.parent_capability_id.clone();
        ancestors.push(parent);
    }
    ancestors.reverse();
    Ok(ancestors)
}

pub(super) async fn top_level_work_items_for_feature(
    pool: &SqlitePool,
    product_id: Option<&str>,
    product_area_id: Option<&str>,
    feature_id: Option<&str>,
    limit: i64,
) -> Result<Vec<WorkItem>, AppError> {
    let Some(feature_id) = feature_id else {
        return Ok(Vec::new());
    };
    work_item_repo::list_top_level_work_items_page(
        pool,
        work_item_repo::WorkItemListQuery {
            product_id,
            product_area_id,
            capability_id: Some(feature_id),
            source_node_id: Some(feature_id),
            source_node_type: Some("capability"),
            limit: Some(limit),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
}

pub(super) async fn work_item_parent_chain(
    pool: &SqlitePool,
    work_item: &WorkItem,
) -> Result<Vec<WorkItem>, AppError> {
    let mut parents = Vec::new();
    let mut parent_id = work_item.parent_work_item_id.clone();
    while let Some(id) = parent_id {
        let parent = work_item_repo::get_work_item(pool, &id).await?;
        parent_id = parent.parent_work_item_id.clone();
        parents.push(parent);
    }
    parents.reverse();
    Ok(parents)
}

pub(super) async fn work_item_siblings(
    pool: &SqlitePool,
    work_item: &WorkItem,
    limit: i64,
) -> Result<Vec<WorkItem>, AppError> {
    let limit = limit.clamp(1, 100);
    let mut siblings = if let Some(parent_id) = work_item.parent_work_item_id.as_deref() {
        work_item_repo::get_sub_work_items_page(pool, parent_id, Some(limit + 1), Some(0)).await?
    } else {
        let source_node_type = work_item
            .source_node_type
            .as_ref()
            .map(|source_type| source_type.to_string());
        work_item_repo::list_top_level_work_items_page(
            pool,
            work_item_repo::WorkItemListQuery {
                product_id: work_item.product_id.as_deref(),
                product_area_id: work_item.product_area_id.as_deref(),
                capability_id: work_item.capability_id.as_deref(),
                source_node_id: work_item.source_node_id.as_deref(),
                source_node_type: source_node_type.as_deref(),
                limit: Some(limit + 1),
                offset: Some(0),
                ..Default::default()
            },
        )
        .await?
    };
    siblings.retain(|item| item.id != work_item.id);
    siblings.truncate(limit as usize);
    Ok(siblings)
}

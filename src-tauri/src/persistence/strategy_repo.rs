use crate::domain::strategy::{
    ProductDependency, ProductDependencyKind, ProductDependencyStatus, ProductStrategyLink,
    StrategyNode, StrategyNodeKind,
};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};

const STRATEGY_NODE_COLUMNS: &str = "id, parent_node_id, node_kind, name, description, owner_label, sort_order, created_at, updated_at";
const PRODUCT_STRATEGY_LINK_COLUMNS: &str =
    "id, product_id, strategy_node_id, is_primary, created_at";
const PRODUCT_DEPENDENCY_COLUMNS: &str = "id, product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind, description, status, created_at, updated_at";

fn parse_strategy_node_kind(value: &str) -> Result<StrategyNodeKind, AppError> {
    StrategyNodeKind::parse(value).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported strategy node kind '{value}'. Use strategic_area, domain, or subdomain."
        ))
    })
}

fn parse_dependency_kind(value: &str) -> Result<ProductDependencyKind, AppError> {
    ProductDependencyKind::parse(value).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported product dependency kind '{value}'. Use platform, capability, data, integration, operational, or other."
        ))
    })
}

fn parse_dependency_status(value: &str) -> Result<ProductDependencyStatus, AppError> {
    ProductDependencyStatus::parse(value).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported product dependency status '{value}'. Use active, planned, blocked, or retired."
        ))
    })
}

fn row_to_strategy_node(row: sqlx::sqlite::SqliteRow) -> Result<StrategyNode, AppError> {
    Ok(StrategyNode {
        id: row.get("id"),
        parent_node_id: row.get("parent_node_id"),
        node_kind: parse_strategy_node_kind(row.get::<String, _>("node_kind").as_str())?,
        name: row.get("name"),
        description: row.get("description"),
        owner_label: row.get("owner_label"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_product_strategy_link(row: sqlx::sqlite::SqliteRow) -> ProductStrategyLink {
    ProductStrategyLink {
        id: row.get("id"),
        product_id: row.get("product_id"),
        strategy_node_id: row.get("strategy_node_id"),
        is_primary: row.get::<i64, _>("is_primary") != 0,
        created_at: row.get("created_at"),
    }
}

fn row_to_product_dependency(row: sqlx::sqlite::SqliteRow) -> Result<ProductDependency, AppError> {
    Ok(ProductDependency {
        id: row.get("id"),
        product_id: row.get("product_id"),
        capability_id: row.get("capability_id"),
        depends_on_product_id: row.get("depends_on_product_id"),
        depends_on_capability_id: row.get("depends_on_capability_id"),
        dependency_kind: parse_dependency_kind(row.get::<String, _>("dependency_kind").as_str())?,
        description: row.get("description"),
        status: parse_dependency_status(row.get::<String, _>("status").as_str())?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn list_strategy_nodes(pool: &SqlitePool) -> Result<Vec<StrategyNode>, AppError> {
    let rows = sqlx::query(&format!(
        "SELECT {STRATEGY_NODE_COLUMNS} FROM strategy_nodes ORDER BY sort_order ASC, name ASC"
    ))
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_strategy_node).collect()
}

pub async fn get_strategy_node(pool: &SqlitePool, id: &str) -> Result<StrategyNode, AppError> {
    let row = sqlx::query(&format!(
        "SELECT {STRATEGY_NODE_COLUMNS} FROM strategy_nodes WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Strategy node {id} not found")))?;
    row_to_strategy_node(row)
}

async fn validate_strategy_parent(
    pool: &SqlitePool,
    parent_node_id: Option<&str>,
    node_kind: &StrategyNodeKind,
) -> Result<(), AppError> {
    match parent_node_id {
        Some(parent_id) => {
            let parent = get_strategy_node(pool, parent_id).await?;
            if !parent.node_kind.supports_child_kind(node_kind) {
                return Err(AppError::Validation(format!(
                    "{} cannot contain {} strategy nodes.",
                    parent.node_kind, node_kind
                )));
            }
            Ok(())
        }
        None => {
            if !node_kind.is_root_kind() {
                return Err(AppError::Validation(
                    "Root strategy nodes must use strategic_area.".to_string(),
                ));
            }
            Ok(())
        }
    }
}

pub async fn create_strategy_node(
    pool: &SqlitePool,
    id: &str,
    parent_node_id: Option<&str>,
    node_kind: &str,
    name: &str,
    description: &str,
    owner_label: &str,
) -> Result<StrategyNode, AppError> {
    let kind = parse_strategy_node_kind(node_kind)?;
    validate_strategy_parent(pool, parent_node_id, &kind).await?;
    let sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM strategy_nodes WHERE parent_node_id IS ?",
    )
    .bind(parent_node_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO strategy_nodes (id, parent_node_id, node_kind, name, description, owner_label, sort_order)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(parent_node_id)
    .bind(kind.to_string())
    .bind(name)
    .bind(description)
    .bind(owner_label)
    .bind(sort_order)
    .execute(pool)
    .await?;

    get_strategy_node(pool, id).await
}

pub async fn update_strategy_node(
    pool: &SqlitePool,
    id: &str,
    parent_node_id: Option<Option<&str>>,
    node_kind: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
    owner_label: Option<&str>,
) -> Result<StrategyNode, AppError> {
    let existing = get_strategy_node(pool, id).await?;
    let next_kind = match node_kind {
        Some(value) => parse_strategy_node_kind(value)?,
        None => existing.node_kind,
    };
    let next_parent = parent_node_id.unwrap_or(existing.parent_node_id.as_deref());
    validate_strategy_parent(pool, next_parent, &next_kind).await?;

    sqlx::query(
        "UPDATE strategy_nodes
         SET parent_node_id = ?,
             node_kind = ?,
             name = COALESCE(?, name),
             description = COALESCE(?, description),
             owner_label = COALESCE(?, owner_label),
             updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(next_parent)
    .bind(next_kind.to_string())
    .bind(name)
    .bind(description)
    .bind(owner_label)
    .bind(id)
    .execute(pool)
    .await?;

    get_strategy_node(pool, id).await
}

pub async fn delete_strategy_node(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM strategy_nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_product_strategy_links(
    pool: &SqlitePool,
) -> Result<Vec<ProductStrategyLink>, AppError> {
    let rows = sqlx::query(&format!(
        "SELECT {PRODUCT_STRATEGY_LINK_COLUMNS} FROM product_strategy_links ORDER BY is_primary DESC, created_at ASC"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(row_to_product_strategy_link).collect())
}

pub async fn link_product_to_strategy(
    pool: &SqlitePool,
    id: &str,
    product_id: &str,
    strategy_node_id: &str,
    is_primary: bool,
) -> Result<ProductStrategyLink, AppError> {
    if is_primary {
        sqlx::query("UPDATE product_strategy_links SET is_primary = 0 WHERE product_id = ?")
            .bind(product_id)
            .execute(pool)
            .await?;
    }

    sqlx::query(
        "INSERT INTO product_strategy_links (id, product_id, strategy_node_id, is_primary)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(product_id, strategy_node_id)
         DO UPDATE SET is_primary = excluded.is_primary",
    )
    .bind(id)
    .bind(product_id)
    .bind(strategy_node_id)
    .bind(if is_primary { 1 } else { 0 })
    .execute(pool)
    .await?;

    let row = sqlx::query(&format!(
        "SELECT {PRODUCT_STRATEGY_LINK_COLUMNS} FROM product_strategy_links WHERE product_id = ? AND strategy_node_id = ?"
    ))
    .bind(product_id)
    .bind(strategy_node_id)
    .fetch_one(pool)
    .await?;
    Ok(row_to_product_strategy_link(row))
}

pub async fn unlink_product_from_strategy(
    pool: &SqlitePool,
    product_id: &str,
    strategy_node_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM product_strategy_links WHERE product_id = ? AND strategy_node_id = ?")
        .bind(product_id)
        .bind(strategy_node_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_product_dependencies(
    pool: &SqlitePool,
) -> Result<Vec<ProductDependency>, AppError> {
    let rows = sqlx::query(&format!(
        "SELECT {PRODUCT_DEPENDENCY_COLUMNS} FROM product_dependencies ORDER BY created_at ASC"
    ))
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_product_dependency).collect()
}

pub async fn create_product_dependency(
    pool: &SqlitePool,
    id: &str,
    product_id: &str,
    capability_id: Option<&str>,
    depends_on_product_id: &str,
    depends_on_capability_id: Option<&str>,
    dependency_kind: &str,
    description: &str,
    status: &str,
) -> Result<ProductDependency, AppError> {
    if product_id == depends_on_product_id {
        return Err(AppError::Validation(
            "A product cannot depend on itself.".to_string(),
        ));
    }
    let kind = parse_dependency_kind(dependency_kind)?;
    let status = parse_dependency_status(status)?;
    sqlx::query(
        "INSERT INTO product_dependencies (id, product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind, description, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(product_id)
    .bind(capability_id)
    .bind(depends_on_product_id)
    .bind(depends_on_capability_id)
    .bind(kind.to_string())
    .bind(description)
    .bind(status.to_string())
    .execute(pool)
    .await?;

    let row = sqlx::query(&format!(
        "SELECT {PRODUCT_DEPENDENCY_COLUMNS} FROM product_dependencies WHERE id = ?"
    ))
    .bind(id)
    .fetch_one(pool)
    .await?;
    row_to_product_dependency(row)
}

pub async fn delete_product_dependency(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM product_dependencies WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

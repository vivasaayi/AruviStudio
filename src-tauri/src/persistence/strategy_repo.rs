use crate::domain::strategy::{
    ProductDependency, ProductDependencyKind, ProductDependencyStatus, ProductStrategyLink,
    StrategyNode, StrategyNodeKind,
};
use crate::error::AppError;
use sqlx::{Row, SqlitePool, SqliteConnection};

const STRATEGY_NODE_COLUMNS: &str = "id, parent_node_id, node_kind, name, description, owner_label, sort_order, created_at, updated_at";
const PRODUCT_STRATEGY_LINK_COLUMNS: &str =
    "id, product_id, strategy_node_id, is_primary, created_at";
const PRODUCT_DEPENDENCY_COLUMNS: &str = "id, product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind, description, status, created_at, updated_at";

fn parse_strategy_node_kind(value: &str) -> Result<StrategyNodeKind, AppError> {
    StrategyNodeKind::parse(value).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported strategy node kind '{value}'. Use strategic_product_area, domain, or sub_domain."
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

async fn strategy_node_on(conn: &mut SqliteConnection, id: &str) -> Result<StrategyNode, AppError> {
    let row = sqlx::query(&format!("SELECT {STRATEGY_NODE_COLUMNS} FROM strategy_nodes WHERE id = ?"))
        .bind(id).fetch_optional(&mut *conn).await?
        .ok_or_else(|| AppError::NotFound(format!("Strategy node {id} not found")))?;
    row_to_strategy_node(row)
}

async fn validate_strategy_parent(
    conn: &mut SqliteConnection,
    parent_node_id: Option<&str>,
    node_kind: &StrategyNodeKind,
) -> Result<(), AppError> {
    match parent_node_id {
        Some(parent_id) => {
            let parent = strategy_node_on(conn, parent_id).await?;
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
                    "Root strategy nodes must use strategic_product_area.".to_string(),
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
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    validate_strategy_parent(&mut transaction, parent_node_id, &kind).await?;
    let sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM strategy_nodes WHERE parent_node_id IS ?",
    )
    .bind(parent_node_id)
    .fetch_one(&mut *transaction)
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
    .execute(&mut *transaction)
    .await?;

    let node = strategy_node_on(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(node)
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
    update_strategy_node_guarded(pool, id, parent_node_id, node_kind, name, description, owner_label, None).await
}

pub async fn update_strategy_node_guarded(
    pool: &SqlitePool, id: &str, parent_node_id: Option<Option<&str>>, node_kind: Option<&str>,
    name: Option<&str>, description: Option<&str>, owner_label: Option<&str>, expected: Option<&StrategyNode>,
) -> Result<StrategyNode, AppError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let existing = strategy_node_on(&mut transaction, id).await?;
    if let Some(expected) = expected {
        if expected.id != existing.id || expected.parent_node_id != existing.parent_node_id || expected.node_kind != existing.node_kind || expected.name != existing.name || expected.description != existing.description || expected.owner_label != existing.owner_label || expected.sort_order != existing.sort_order {
            return Err(AppError::Validation("Strategy changed; refresh before saving.".into()));
        }
    }
    let next_kind = match node_kind {
        Some(value) => parse_strategy_node_kind(value)?,
        None => existing.node_kind.clone(),
    };
    let next_parent = parent_node_id.unwrap_or(existing.parent_node_id.as_deref());
    validate_strategy_parent(&mut transaction, next_parent, &next_kind).await?;
    if let Some(parent_id) = next_parent {
        let cycle: i64 = sqlx::query_scalar("WITH RECURSIVE descendants(id) AS (SELECT id FROM strategy_nodes WHERE id=? UNION SELECT s.id FROM strategy_nodes s JOIN descendants d ON s.parent_node_id=d.id) SELECT EXISTS(SELECT 1 FROM descendants WHERE id=?)")
            .bind(id).bind(parent_id).fetch_one(&mut *transaction).await?;
        if cycle != 0 { return Err(AppError::Validation("A strategy node cannot be moved under itself or its descendants.".into())); }
    }
    let child_kinds: Vec<String> = sqlx::query_scalar("SELECT DISTINCT node_kind FROM strategy_nodes WHERE parent_node_id=?")
        .bind(id).fetch_all(&mut *transaction).await?;
    for child in child_kinds {
        if !next_kind.supports_child_kind(&parse_strategy_node_kind(&child)?) {
            return Err(AppError::Validation("The selected kind cannot contain the existing child nodes.".into()));
        }
    }

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
    .execute(&mut *transaction)
    .await?;

    let node = strategy_node_on(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(node)
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
    let mut transaction = pool.begin().await?;
    if is_primary {
        sqlx::query("UPDATE product_strategy_links SET is_primary = 0 WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut *transaction)
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
    .execute(&mut *transaction)
    .await?;

    let row = sqlx::query(&format!(
        "SELECT {PRODUCT_STRATEGY_LINK_COLUMNS} FROM product_strategy_links WHERE product_id = ? AND strategy_node_id = ?"
    ))
    .bind(product_id)
    .bind(strategy_node_id)
    .fetch_one(&mut *transaction)
    .await?;
    let link = row_to_product_strategy_link(row);
    transaction.commit().await?;
    Ok(link)
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

pub struct CreateProductDependencyInput<'a> {
    pub id: &'a str,
    pub product_id: &'a str,
    pub capability_id: Option<&'a str>,
    pub depends_on_product_id: &'a str,
    pub depends_on_capability_id: Option<&'a str>,
    pub dependency_kind: &'a str,
    pub description: &'a str,
    pub status: &'a str,
}

pub async fn create_product_dependency(
    pool: &SqlitePool,
    input: CreateProductDependencyInput<'_>,
) -> Result<ProductDependency, AppError> {
    if input.product_id == input.depends_on_product_id {
        return Err(AppError::Validation(
            "A product cannot depend on itself.".to_string(),
        ));
    }
    let kind = parse_dependency_kind(input.dependency_kind)?;
    let status = parse_dependency_status(input.status)?;
    let inserted = sqlx::query(
        "INSERT INTO product_dependencies (id, product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind, description, status)
         SELECT ?, ?, ?, ?, ?, ?, ?, ?
         WHERE (? IS NULL OR EXISTS (SELECT 1 FROM capabilities c JOIN product_areas a ON a.id=c.product_area_id WHERE c.id=? AND a.product_id=?))
           AND (? IS NULL OR EXISTS (SELECT 1 FROM capabilities c JOIN product_areas a ON a.id=c.product_area_id WHERE c.id=? AND a.product_id=?))",
    )
    .bind(input.id)
    .bind(input.product_id)
    .bind(input.capability_id)
    .bind(input.depends_on_product_id)
    .bind(input.depends_on_capability_id)
    .bind(kind.to_string())
    .bind(input.description)
    .bind(status.to_string())
    .bind(input.capability_id).bind(input.capability_id).bind(input.product_id)
    .bind(input.depends_on_capability_id).bind(input.depends_on_capability_id).bind(input.depends_on_product_id)
    .execute(pool)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(AppError::Validation("Dependency capabilities must belong to their respective products.".into()));
    }

    let row = sqlx::query(&format!(
        "SELECT {PRODUCT_DEPENDENCY_COLUMNS} FROM product_dependencies WHERE id = ?"
    ))
    .bind(input.id)
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

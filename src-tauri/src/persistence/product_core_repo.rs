use crate::domain::product::Product;
use crate::error::AppError;
use sqlx::{Row, SqliteConnection, SqlitePool};
use tracing::{debug, error};

const PRODUCT_SELECT_COLUMNS: &str = "id, name, description, vision, goals, tags, status, lifecycle, health, owner_label, investment_status, roadmap, evidence, created_at, updated_at";

#[derive(Clone, Copy, Debug)]
pub struct CreateProductInput<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub vision: &'a str,
    pub goals: &'a str,
    pub tags: &'a str,
    pub lifecycle: Option<&'a str>,
    pub health: Option<&'a str>,
    pub owner_label: Option<&'a str>,
    pub investment_status: Option<&'a str>,
    pub roadmap: Option<&'a str>,
    pub evidence: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub struct UpdateProductPatch<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub vision: Option<&'a str>,
    pub goals: Option<&'a str>,
    pub tags: Option<&'a str>,
    pub lifecycle: Option<&'a str>,
    pub health: Option<&'a str>,
    pub owner_label: Option<&'a str>,
    pub investment_status: Option<&'a str>,
    pub roadmap: Option<&'a str>,
    pub evidence: Option<&'a str>,
}

fn row_to_product(row: sqlx::sqlite::SqliteRow) -> Product {
    Product {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        vision: row.get("vision"),
        goals: serde_json::from_str::<Vec<String>>(row.get::<String, _>("goals").as_str())
            .unwrap_or_default(),
        tags: serde_json::from_str::<Vec<String>>(row.get::<String, _>("tags").as_str())
            .unwrap_or_default(),
        status: row.get("status"),
        lifecycle: row.get("lifecycle"),
        health: row.get("health"),
        owner_label: row.get("owner_label"),
        investment_status: row.get("investment_status"),
        roadmap: row.get("roadmap"),
        evidence: row.get("evidence"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn create_product(pool: &SqlitePool, input: CreateProductInput<'_>) -> Result<Product, AppError> {
    let mut conn = pool.acquire().await?;
    create_product_in_conn(&mut conn, input).await
}

pub(crate) async fn create_product_in_conn(conn: &mut SqliteConnection, input: CreateProductInput<'_>) -> Result<Product, AppError> {
    debug!(product_id = %input.id, product_name = %input.name, "persist create_product");
    let result = sqlx::query(&format!(
        "INSERT INTO products (id, name, description, vision, goals, tags, lifecycle, health, owner_label, investment_status, roadmap, evidence)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {PRODUCT_SELECT_COLUMNS}"
    ))
        .bind(input.id).bind(input.name).bind(input.description).bind(input.vision).bind(input.goals).bind(input.tags)
        .bind(input.lifecycle.unwrap_or("incubating"))
        .bind(input.health.unwrap_or("unknown"))
        .bind(input.owner_label.unwrap_or_default())
        .bind(input.investment_status.unwrap_or("evaluate"))
        .bind(input.roadmap.unwrap_or_default())
        .bind(input.evidence.unwrap_or_default())
        .map(row_to_product)
        .fetch_one(&mut *conn).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(product_id = %input.id, error = %err, "persist create_product failed");
    }
    result
}

pub async fn get_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    let mut conn = pool.acquire().await?;
    get_product_in_conn(&mut conn, id).await
}

pub(crate) async fn get_product_in_conn(conn: &mut SqliteConnection, id: &str) -> Result<Product, AppError> {
    sqlx::query(&format!(
        "SELECT {PRODUCT_SELECT_COLUMNS} FROM products WHERE id = ?"
    ))
    .bind(id)
    .map(row_to_product)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Product {id} not found")))
}

pub async fn list_products(pool: &SqlitePool) -> Result<Vec<Product>, AppError> {
    let mut conn = pool.acquire().await?;
    list_products_in_conn(&mut conn).await
}

pub(crate) async fn list_products_in_conn(conn: &mut SqliteConnection) -> Result<Vec<Product>, AppError> {
    sqlx::query(&format!(
        "SELECT {PRODUCT_SELECT_COLUMNS} FROM products ORDER BY created_at DESC"
    ))
    .map(row_to_product)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.into())
}

pub async fn update_product(pool: &SqlitePool, patch: UpdateProductPatch<'_>) -> Result<Product, AppError> {
    let mut conn = pool.acquire().await?;
    update_product_in_conn(&mut conn, patch).await
}

pub(crate) async fn update_product_in_conn(conn: &mut SqliteConnection, patch: UpdateProductPatch<'_>) -> Result<Product, AppError> {
    debug!(product_id = %patch.id, "persist update_product");
    let result = sqlx::query(
        "UPDATE products SET name=COALESCE(?,name), description=COALESCE(?,description),
         vision=COALESCE(?,vision), goals=COALESCE(?,goals), tags=COALESCE(?,tags),
         lifecycle=COALESCE(?,lifecycle), health=COALESCE(?,health), owner_label=COALESCE(?,owner_label),
         investment_status=COALESCE(?,investment_status), roadmap=COALESCE(?,roadmap),
         evidence=COALESCE(?,evidence), updated_at=datetime('now') WHERE id=?"
    )
        .bind(patch.name).bind(patch.description).bind(patch.vision).bind(patch.goals).bind(patch.tags)
        .bind(patch.lifecycle).bind(patch.health).bind(patch.owner_label).bind(patch.investment_status)
        .bind(patch.roadmap).bind(patch.evidence).bind(patch.id).execute(&mut *conn).await?;
    if result.rows_affected() == 0 { return Err(AppError::NotFound(format!("Product {} not found", patch.id))); }
    get_product_in_conn(conn, patch.id).await
}

/// Compare and update the entire editable product in one write transaction.
pub async fn update_product_checked(pool: &SqlitePool, id: &str, expected: &Product, fields: &Product) -> Result<Option<Product>, AppError> {
    if expected.id != id || fields.id != id || fields.status != expected.status {
        return Err(AppError::Validation("Product identity and status cannot be changed here".into()));
    }
    if fields.name.trim().is_empty() || fields.name.len() > 200 {
        return Err(AppError::Validation("Name must contain 1 to 200 characters".into()));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let current = match get_product_in_conn(&mut tx, id).await {
        Ok(product) => product,
        Err(AppError::NotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    if &current != expected { return Ok(None); }
    let goals = serde_json::to_string(&fields.goals)?;
    let tags = serde_json::to_string(&fields.tags)?;
    let lifecycle = fields.lifecycle.to_string(); let health = fields.health.to_string(); let investment = fields.investment_status.to_string();
    let product = update_product_in_conn(&mut tx, UpdateProductPatch {id, name:Some(fields.name.trim()), description:Some(&fields.description), vision:Some(&fields.vision), goals:Some(&goals), tags:Some(&tags), lifecycle:Some(&lifecycle), health:Some(&health), owner_label:Some(&fields.owner_label), investment_status:Some(&investment), roadmap:Some(&fields.roadmap), evidence:Some(&fields.evidence)}).await?;
    tx.commit().await?;
    Ok(Some(product))
}

pub async fn archive_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    sqlx::query("UPDATE products SET status='archived', updated_at=datetime('now') WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    get_product(pool, id).await
}

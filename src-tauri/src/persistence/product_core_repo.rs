use crate::domain::product::Product;
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
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

pub async fn create_product(
    pool: &SqlitePool,
    input: CreateProductInput<'_>,
) -> Result<Product, AppError> {
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
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(product_id = %input.id, error = %err, "persist create_product failed");
    }
    result
}

pub async fn get_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    sqlx::query(&format!(
        "SELECT {PRODUCT_SELECT_COLUMNS} FROM products WHERE id = ?"
    ))
    .bind(id)
    .map(row_to_product)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Product {id} not found")))
}

pub async fn list_products(pool: &SqlitePool) -> Result<Vec<Product>, AppError> {
    sqlx::query(&format!(
        "SELECT {PRODUCT_SELECT_COLUMNS} FROM products ORDER BY created_at DESC"
    ))
    .map(row_to_product)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn update_product(
    pool: &SqlitePool,
    patch: UpdateProductPatch<'_>,
) -> Result<Product, AppError> {
    debug!(product_id = %patch.id, "persist update_product");
    let existing = get_product(pool, patch.id).await?;
    let name = patch.name.unwrap_or(&existing.name);
    let description = patch.description.unwrap_or(&existing.description);
    let vision = patch.vision.unwrap_or(&existing.vision);
    let existing_goals = serde_json::to_string(&existing.goals).unwrap_or_default();
    let existing_tags = serde_json::to_string(&existing.tags).unwrap_or_default();
    let goals_str = patch.goals.unwrap_or(&existing_goals);
    let tags_str = patch.tags.unwrap_or(&existing_tags);
    let existing_lifecycle = existing.lifecycle.to_string();
    let existing_health = existing.health.to_string();
    let existing_investment_status = existing.investment_status.to_string();
    sqlx::query(
        "UPDATE products
         SET name=?, description=?, vision=?, goals=?, tags=?, lifecycle=?, health=?, owner_label=?, investment_status=?, roadmap=?, evidence=?, updated_at=datetime('now')
         WHERE id=?"
    )
        .bind(name).bind(description).bind(vision).bind(goals_str).bind(tags_str)
        .bind(patch.lifecycle.unwrap_or(&existing_lifecycle))
        .bind(patch.health.unwrap_or(&existing_health))
        .bind(patch.owner_label.unwrap_or(&existing.owner_label))
        .bind(patch.investment_status.unwrap_or(&existing_investment_status))
        .bind(patch.roadmap.unwrap_or(&existing.roadmap))
        .bind(patch.evidence.unwrap_or(&existing.evidence))
        .bind(patch.id)
        .execute(pool).await?;
    get_product(pool, patch.id).await
}

pub async fn archive_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    sqlx::query("UPDATE products SET status='archived', updated_at=datetime('now') WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    get_product(pool, id).await
}

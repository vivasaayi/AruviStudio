use crate::domain::product::ProductArea;
use crate::error::AppError;
use crate::persistence::product_hierarchy_rules::resolve_root_node_kind;
use sqlx::SqlitePool;
use tracing::{debug, trace};

const PRODUCT_AREA_SELECT_COLUMNS: &str = "id, product_id, node_kind, name, description, purpose, explanation, examples, implementation_notes, test_guidance, sort_order, created_at, updated_at";

#[derive(Clone, Copy, Debug)]
pub struct CreateProductAreaInput<'a> {
    pub id: &'a str,
    pub product_id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub purpose: &'a str,
    pub node_kind: Option<&'a str>,
    pub explanation: &'a str,
    pub examples: &'a str,
    pub implementation_notes: &'a str,
    pub test_guidance: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct UpdateProductAreaPatch<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub purpose: Option<&'a str>,
    pub node_kind: Option<&'a str>,
    pub explanation: Option<&'a str>,
    pub examples: Option<&'a str>,
    pub implementation_notes: Option<&'a str>,
    pub test_guidance: Option<&'a str>,
}

pub async fn create_product_area(
    pool: &SqlitePool,
    input: CreateProductAreaInput<'_>,
) -> Result<ProductArea, AppError> {
    debug!(product_area_id = %input.id, product_id = %input.product_id, product_area_name = %input.name, "persist create_product_area");
    let node_kind = resolve_root_node_kind(input.node_kind)?;
    let next_sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM product_areas WHERE product_id = ?",
    )
    .bind(input.product_id)
    .fetch_one(pool)
    .await?;
    trace!(product_area_id = %input.id, product_id = %input.product_id, sort_order = next_sort_order, "resolved product_area sort order");
    sqlx::query_as::<_, ProductArea>(
        &format!(
            "INSERT INTO product_areas (id, product_id, node_kind, name, description, purpose, explanation, examples, implementation_notes, test_guidance, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {PRODUCT_AREA_SELECT_COLUMNS}"
        ),
    )
        .bind(input.id).bind(input.product_id).bind(node_kind).bind(input.name).bind(input.description).bind(input.purpose)
        .bind(input.explanation).bind(input.examples).bind(input.implementation_notes).bind(input.test_guidance)
        .bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_product_areas(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<ProductArea>, AppError> {
    sqlx::query_as::<_, ProductArea>(&format!(
        "SELECT {PRODUCT_AREA_SELECT_COLUMNS} FROM product_areas WHERE product_id=? ORDER BY sort_order"
    ))
    .bind(product_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn update_product_area(
    pool: &SqlitePool,
    patch: UpdateProductAreaPatch<'_>,
) -> Result<ProductArea, AppError> {
    debug!(product_area_id = %patch.id, "persist update_product_area");
    let existing = sqlx::query_as::<_, ProductArea>(&format!(
        "SELECT {PRODUCT_AREA_SELECT_COLUMNS} FROM product_areas WHERE id=?"
    ))
    .bind(patch.id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Product Area {} not found", patch.id)))?;
    let name = patch.name.unwrap_or(&existing.name);
    let description = patch.description.unwrap_or(&existing.description);
    let purpose = patch.purpose.unwrap_or(&existing.purpose);
    let explanation = patch.explanation.unwrap_or(&existing.explanation);
    let examples = patch.examples.unwrap_or(&existing.examples);
    let implementation_notes = patch
        .implementation_notes
        .unwrap_or(&existing.implementation_notes);
    let test_guidance = patch.test_guidance.unwrap_or(&existing.test_guidance);
    let node_kind = if let Some(value) = patch.node_kind {
        resolve_root_node_kind(Some(value))?
    } else {
        existing.node_kind
    };
    sqlx::query("UPDATE product_areas SET name=?, description=?, purpose=?, explanation=?, examples=?, implementation_notes=?, test_guidance=?, node_kind=?, updated_at=datetime('now') WHERE id=?")
        .bind(name).bind(description).bind(purpose).bind(explanation).bind(examples)
        .bind(implementation_notes).bind(test_guidance).bind(node_kind).bind(patch.id).execute(pool).await?;
    sqlx::query_as::<_, ProductArea>(&format!(
        "SELECT {PRODUCT_AREA_SELECT_COLUMNS} FROM product_areas WHERE id=?"
    ))
    .bind(patch.id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn delete_product_area(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM product_areas WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder_product_areas(
    pool: &SqlitePool,
    product_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    debug!(product_id = %product_id, item_count = ordered_ids.len(), "persist reorder_product_areas");
    for (index, id) in ordered_ids.iter().enumerate() {
        sqlx::query("UPDATE product_areas SET sort_order=?, updated_at=datetime('now') WHERE id=? AND product_id=?")
            .bind(index as i64)
            .bind(id)
            .bind(product_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

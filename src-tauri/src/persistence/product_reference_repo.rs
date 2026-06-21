use crate::domain::product::ProductReference;
use crate::error::AppError;
use sqlx::SqlitePool;

const PRODUCT_REFERENCE_SELECT_COLUMNS: &str =
    "id, scope_type, scope_id, title, reference_kind, uri, content, created_at, updated_at";

pub async fn list_product_references(
    pool: &SqlitePool,
    scope_type: Option<&str>,
    scope_id: Option<&str>,
) -> Result<Vec<ProductReference>, AppError> {
    let mut query = format!("SELECT {PRODUCT_REFERENCE_SELECT_COLUMNS} FROM \"references\"");
    if scope_type.is_some() && scope_id.is_some() {
        query.push_str(" WHERE scope_type = ? AND scope_id = ?");
    }
    query.push_str(" ORDER BY updated_at DESC, created_at DESC");
    let mut q = sqlx::query_as::<_, ProductReference>(&query);
    if let (Some(scope_type), Some(scope_id)) = (scope_type, scope_id) {
        q = q.bind(scope_type).bind(scope_id);
    }
    q.fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn create_product_reference(
    pool: &SqlitePool,
    input: CreateProductReferenceInput<'_>,
) -> Result<ProductReference, AppError> {
    sqlx::query_as::<_, ProductReference>(&format!(
        "INSERT INTO \"references\" (id, scope_type, scope_id, title, reference_kind, uri, content)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING {PRODUCT_REFERENCE_SELECT_COLUMNS}"
    ))
    .bind(input.id)
    .bind(input.scope_type)
    .bind(input.scope_id)
    .bind(input.title)
    .bind(input.reference_kind)
    .bind(input.uri)
    .bind(input.content)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub struct CreateProductReferenceInput<'a> {
    pub id: &'a str,
    pub scope_type: &'a str,
    pub scope_id: &'a str,
    pub title: &'a str,
    pub reference_kind: &'a str,
    pub uri: &'a str,
    pub content: &'a str,
}

pub async fn delete_product_reference(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM \"references\" WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

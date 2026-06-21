use crate::domain::product::{ProductTree, ProductTreeSummary};
use crate::error::AppError;
pub use crate::persistence::product_core_repo::{
    archive_product, create_product, get_product, list_products, update_product,
    CreateProductInput, UpdateProductPatch,
};
pub use crate::persistence::product_hierarchy_repo::{
    convert_capability_node_kind, create_capability, create_product_area, delete_capability,
    delete_product_area, get_capability, list_capabilities, list_product_areas,
    reorder_capabilities, reorder_product_areas, update_capability, update_product_area,
    CreateCapabilityInput, CreateProductAreaInput, UpdateCapabilityPatch, UpdateProductAreaPatch,
};
pub use crate::persistence::product_plan_reset_repo::reset_product_plan;
pub use crate::persistence::product_reference_repo::{
    create_product_reference, delete_product_reference, list_product_references,
    CreateProductReferenceInput,
};
use sqlx::SqlitePool;

pub async fn get_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTree, AppError> {
    crate::persistence::product_tree_repo::get_product_tree(pool, product_id).await
}

pub async fn summarize_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTreeSummary, AppError> {
    crate::persistence::product_tree_repo::summarize_product_tree(pool, product_id).await
}

#[cfg(test)]
mod tests;

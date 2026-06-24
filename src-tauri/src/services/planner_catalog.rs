use crate::domain::product::{Capability, Product, ProductArea};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use sqlx::SqlitePool;

pub(crate) use crate::services::planner_catalog_tree::build_tree_nodes;

pub(crate) async fn find_product(
    db: &SqlitePool,
    product_name: Option<&str>,
) -> Result<Product, AppError> {
    let products = product_repo::list_products(db).await?;
    if let Some(name) = product_name {
        let normalized = normalize_name(Some(name));
        let exact = products
            .iter()
            .find(|product| normalize_name(Some(&product.name)) == normalized)
            .cloned();
        if let Some(product) = exact {
            return Ok(product);
        }
        let partial = products
            .into_iter()
            .filter(|product| normalize_name(Some(&product.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple products match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!("No product matches {}", name)));
    }
    if products.len() == 1 {
        return Ok(products[0].clone());
    }
    Err(AppError::Validation("Product is required".to_string()))
}

pub(crate) async fn find_product_area(
    db: &SqlitePool,
    product_name: Option<&str>,
    product_area_name: Option<&str>,
) -> Result<ProductArea, AppError> {
    let product = find_product(db, product_name).await?;
    let product_areas = product_repo::list_product_areas(db, &product.id).await?;
    if let Some(name) = product_area_name {
        let normalized = normalize_name(Some(name));
        let exact = product_areas
            .iter()
            .find(|product_area| normalize_name(Some(&product_area.name)) == normalized)
            .cloned();
        if let Some(product_area) = exact {
            return Ok(product_area);
        }
        let partial = product_areas
            .into_iter()
            .filter(|product_area| normalize_name(Some(&product_area.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple product_areas match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!(
            "No product_area matches {}",
            name
        )));
    }
    if product_areas.len() == 1 {
        return Ok(product_areas[0].clone());
    }
    Err(AppError::Validation("Product Area is required".to_string()))
}

pub(crate) async fn find_capability(
    db: &SqlitePool,
    product_name: Option<&str>,
    product_area_name: Option<&str>,
    capability_name: Option<&str>,
) -> Result<Capability, AppError> {
    let product_area = find_product_area(db, product_name, product_area_name).await?;
    let capabilities = product_repo::list_capabilities(db, &product_area.id).await?;
    if let Some(name) = capability_name {
        let normalized = normalize_name(Some(name));
        let exact = capabilities
            .iter()
            .find(|capability| normalize_name(Some(&capability.name)) == normalized)
            .cloned();
        if let Some(capability) = exact {
            return Ok(capability);
        }
        let partial = capabilities
            .into_iter()
            .filter(|capability| normalize_name(Some(&capability.name)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple capabilities match {}",
                name
            )));
        }
        return Err(AppError::NotFound(format!(
            "No capability matches {}",
            name
        )));
    }
    Err(AppError::Validation("Capability is required".to_string()))
}

pub(crate) async fn find_work_item(
    db: &SqlitePool,
    work_item_title: Option<&str>,
    product_name: Option<&str>,
) -> Result<WorkItem, AppError> {
    let product_id = if let Some(name) = product_name {
        Some(find_product(db, Some(name)).await?.id)
    } else {
        None
    };
    if let Some(title) = work_item_title {
        let normalized = normalize_name(Some(title));
        let work_items =
            work_item_repo::search_work_items_by_title(db, product_id.as_deref(), title, 2).await?;
        let exact = work_items
            .iter()
            .find(|work_item| normalize_name(Some(&work_item.title)) == normalized)
            .cloned();
        if let Some(work_item) = exact {
            return Ok(work_item);
        }
        let partial = work_items
            .into_iter()
            .filter(|work_item| normalize_name(Some(&work_item.title)).contains(&normalized))
            .collect::<Vec<_>>();
        if partial.len() == 1 {
            return Ok(partial[0].clone());
        }
        if partial.len() > 1 {
            return Err(AppError::Validation(format!(
                "Multiple work items match {}",
                title
            )));
        }
        return Err(AppError::NotFound(format!(
            "No work item matches {}",
            title
        )));
    }
    Err(AppError::Validation("Work item is required".to_string()))
}

fn normalize_name(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_lowercase()
}

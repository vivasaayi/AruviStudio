use crate::domain::product::HierarchyNodeType;
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
use tracing::{debug, error, trace};

#[derive(Clone, Copy, Debug)]
pub struct CreateWorkItemInput<'a> {
    pub id: &'a str,
    pub product_id: &'a str,
    pub product_area_id: Option<&'a str>,
    pub capability_id: Option<&'a str>,
    pub source_node_id: Option<&'a str>,
    pub source_node_type: Option<&'a str>,
    pub parent_work_item_id: Option<&'a str>,
    pub title: &'a str,
    pub problem_statement: &'a str,
    pub description: &'a str,
    pub acceptance_criteria: &'a str,
    pub constraints: &'a str,
    pub work_item_type: &'a str,
    pub priority: &'a str,
    pub complexity: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct UpdateWorkItemPatch<'a> {
    pub id: &'a str,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub status: Option<&'a str>,
    pub problem_statement: Option<&'a str>,
    pub acceptance_criteria: Option<&'a str>,
    pub constraints: Option<&'a str>,
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

fn parse_source_node_type(value: &str) -> Result<HierarchyNodeType, AppError> {
    match normalize_source_node_type(value)? {
        "product_area" => Ok(HierarchyNodeType::ProductArea),
        "capability" => Ok(HierarchyNodeType::Capability),
        _ => unreachable!("normalize_source_node_type only returns known values"),
    }
}

fn normalize_work_item_type(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "story" => Ok("story".to_string()),
        "task" => Ok("task".to_string()),
        "setup" | "bug" | "refactor" | "test" | "review" | "security_fix"
        | "performance_improvement" => Ok(normalized),
        other => Err(AppError::Validation(format!(
            "Unsupported work item type '{other}'. Use story, task, setup, bug, refactor, test, review, security_fix, or performance_improvement."
        ))),
    }
}

async fn inherit_source_from_parent(
    pool: &SqlitePool,
    parent_work_item_id: &str,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<HierarchyNodeType>,
    ),
    AppError,
> {
    let row = sqlx::query(
        "SELECT product_area_id, capability_id, source_node_id, source_node_type FROM work_items WHERE id = ?",
    )
    .bind(parent_work_item_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Work item {parent_work_item_id} not found")))?;
    let source_node_type = row
        .get::<Option<String>, _>("source_node_type")
        .map(|value| parse_source_node_type(&value))
        .transpose()?;
    Ok((
        row.get("product_area_id"),
        row.get("capability_id"),
        row.get("source_node_id"),
        source_node_type,
    ))
}

async fn resolve_source_scope(
    pool: &SqlitePool,
    product_id: &str,
    product_area_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    parent_work_item_id: Option<&str>,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<HierarchyNodeType>,
    ),
    AppError,
> {
    let mut resolved_product_area_id = product_area_id.map(str::to_owned);
    let mut resolved_capability_id = capability_id.map(str::to_owned);
    let mut resolved_source_node_id = source_node_id.map(str::to_owned);
    let mut resolved_source_node_type = source_node_type.map(parse_source_node_type).transpose()?;

    if resolved_source_node_id.is_none() {
        if let Some(capability_id) = resolved_capability_id.clone() {
            resolved_source_node_id = Some(capability_id);
            resolved_source_node_type = Some(HierarchyNodeType::Capability);
        } else if let Some(product_area_id) = resolved_product_area_id.clone() {
            resolved_source_node_id = Some(product_area_id);
            resolved_source_node_type = Some(HierarchyNodeType::ProductArea);
        } else if let Some(parent_work_item_id) = parent_work_item_id {
            let inherited = inherit_source_from_parent(pool, parent_work_item_id).await?;
            resolved_product_area_id = inherited.0;
            resolved_capability_id = inherited.1;
            resolved_source_node_id = inherited.2;
            resolved_source_node_type = inherited.3;
        }
    }

    match (
        resolved_source_node_id.as_deref(),
        resolved_source_node_type,
    ) {
        (Some(node_id), Some(HierarchyNodeType::ProductArea)) => {
            let product_area_row = sqlx::query("SELECT product_id FROM product_areas WHERE id = ?")
                .bind(node_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Product Area {node_id} not found")))?;
            let scoped_product_id: String = product_area_row.get("product_id");
            if scoped_product_id != product_id {
                return Err(AppError::Validation(
                    "Work item source node must belong to the selected product.".to_string(),
                ));
            }
            resolved_product_area_id = Some(node_id.to_string());
            resolved_capability_id = None;
        }
        (Some(node_id), Some(HierarchyNodeType::Capability)) => {
            let capability_row = sqlx::query(
                "SELECT c.product_area_id, pa.product_id FROM capabilities c JOIN product_areas pa ON pa.id = c.product_area_id WHERE c.id = ?",
            )
            .bind(node_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Capability {node_id} not found")))?;
            let scoped_product_id: String = capability_row.get("product_id");
            if scoped_product_id != product_id {
                return Err(AppError::Validation(
                    "Work item source node must belong to the selected product.".to_string(),
                ));
            }
            resolved_product_area_id = Some(capability_row.get("product_area_id"));
            resolved_capability_id = Some(node_id.to_string());
        }
        (Some(_), None) => {
            return Err(AppError::Validation(
                "source_node_type is required when source_node_id is provided.".to_string(),
            ));
        }
        (None, Some(_)) => {
            return Err(AppError::Validation(
                "source_node_id is required when source_node_type is provided.".to_string(),
            ));
        }
        (None, None) => {}
    }

    Ok((
        resolved_product_area_id,
        resolved_capability_id,
        resolved_source_node_id,
        resolved_source_node_type,
    ))
}

pub async fn create_work_item(
    pool: &SqlitePool,
    input: CreateWorkItemInput<'_>,
) -> Result<WorkItem, AppError> {
    let work_item_type = normalize_work_item_type(input.work_item_type)?;
    let (product_area_id, capability_id, source_node_id, source_node_type) = resolve_source_scope(
        pool,
        input.product_id,
        input.product_area_id,
        input.capability_id,
        input.source_node_id,
        input.source_node_type,
        input.parent_work_item_id,
    )
    .await?;
    debug!(work_item_id = %input.id, product_id = %input.product_id, product_area_id = ?product_area_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?input.parent_work_item_id, title = %input.title, "persist create_work_item");
    let next_sort_order: i64 = if let Some(parent_id) = input.parent_work_item_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE parent_work_item_id = ?")
            .bind(parent_id)
            .fetch_one(pool)
            .await?
    } else if let (Some(source_node_id), Some(source_node_type)) =
        (source_node_id.as_deref(), source_node_type)
    {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE source_node_id = ? AND source_node_type = ? AND parent_work_item_id IS NULL")
            .bind(source_node_id)
            .bind(source_node_type)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE product_id = ? AND product_area_id IS NULL AND capability_id IS NULL AND parent_work_item_id IS NULL")
            .bind(input.product_id)
            .fetch_one(pool)
            .await?
    };
    trace!(work_item_id = %input.id, sort_order = next_sort_order, "resolved work item sort order");
    let result = sqlx::query_as::<_, WorkItem>("INSERT INTO work_items (id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,sort_order) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at")
        .bind(input.id).bind(input.product_id).bind(&product_area_id).bind(&capability_id).bind(&source_node_id).bind(source_node_type).bind(input.parent_work_item_id).bind(input.title).bind(input.problem_statement).bind(input.description).bind(input.acceptance_criteria).bind(input.constraints).bind(&work_item_type).bind(input.priority).bind(input.complexity).bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(work_item_id = %input.id, product_id = %input.product_id, product_area_id = ?product_area_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?input.parent_work_item_id, error = %err, "persist create_work_item failed");
    }
    result
}

pub async fn update_work_item(
    pool: &SqlitePool,
    patch: UpdateWorkItemPatch<'_>,
) -> Result<WorkItem, AppError> {
    debug!(work_item_id = %patch.id, "persist update_work_item");
    let existing = crate::persistence::work_item_read_repo::get_work_item(pool, patch.id).await?;
    let title = patch.title.unwrap_or(&existing.title);
    let description = patch.description.unwrap_or(&existing.description);
    let existing_status = existing.status.to_string();
    let status = patch.status.unwrap_or(&existing_status);
    let problem_statement = patch
        .problem_statement
        .unwrap_or(&existing.problem_statement);
    let acceptance_criteria = patch
        .acceptance_criteria
        .unwrap_or(&existing.acceptance_criteria);
    let constraints = patch.constraints.unwrap_or(&existing.constraints);
    sqlx::query("UPDATE work_items SET title=?,description=?,status=?,problem_statement=?,acceptance_criteria=?,constraints=?,updated_at=datetime('now') WHERE id=?")
        .bind(title).bind(description).bind(status).bind(problem_statement).bind(acceptance_criteria).bind(constraints).bind(patch.id)
        .execute(pool).await?;
    crate::persistence::work_item_read_repo::get_work_item(pool, patch.id).await
}

pub async fn assign_work_item_workspace(
    pool: &SqlitePool,
    id: &str,
    repository_id: Option<&str>,
    branch_name: Option<&str>,
) -> Result<WorkItem, AppError> {
    debug!(work_item_id = %id, repository_id = ?repository_id, branch_name = ?branch_name, "persist assign_work_item_workspace");
    if let Some(repository_id) = repository_id {
        sqlx::query(
            "UPDATE work_items
             SET repo_override_id=?, active_repo_id=?, branch_name=?, updated_at=datetime('now')
             WHERE id=?",
        )
        .bind(repository_id)
        .bind(repository_id)
        .bind(branch_name)
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE work_items
             SET repo_override_id=NULL, active_repo_id=NULL, branch_name=NULL, updated_at=datetime('now')
             WHERE id=?",
        )
        .bind(id)
        .execute(pool)
        .await?;
    }
    crate::persistence::work_item_read_repo::get_work_item(pool, id).await
}

pub async fn delete_work_item(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    debug!(work_item_id = %id, "persist delete_work_item");
    sqlx::query("DELETE FROM work_items WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder_work_items(pool: &SqlitePool, ordered_ids: &[String]) -> Result<(), AppError> {
    debug!(item_count = ordered_ids.len(), "persist reorder_work_items");
    for (index, id) in ordered_ids.iter().enumerate() {
        sqlx::query("UPDATE work_items SET sort_order=?, updated_at=datetime('now') WHERE id=?")
            .bind(index as i64)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

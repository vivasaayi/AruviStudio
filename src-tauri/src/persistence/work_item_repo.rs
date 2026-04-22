use crate::domain::product::HierarchyNodeType;
use crate::domain::work_item::{ProductWorkItemSummary, WorkItem};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
use tracing::{debug, error, trace};

fn parse_source_node_type(value: &str) -> Result<HierarchyNodeType, AppError> {
    match value {
        "module" => Ok(HierarchyNodeType::Module),
        "capability" => Ok(HierarchyNodeType::Capability),
        _ => Err(AppError::Validation(format!(
            "Unsupported source node type '{value}'. Use module or capability."
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
        "SELECT module_id, capability_id, source_node_id, source_node_type FROM work_items WHERE id = ?",
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
        row.get("module_id"),
        row.get("capability_id"),
        row.get("source_node_id"),
        source_node_type,
    ))
}

async fn resolve_source_scope(
    pool: &SqlitePool,
    product_id: &str,
    module_id: Option<&str>,
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
    let mut resolved_module_id = module_id.map(str::to_owned);
    let mut resolved_capability_id = capability_id.map(str::to_owned);
    let mut resolved_source_node_id = source_node_id.map(str::to_owned);
    let mut resolved_source_node_type = source_node_type.map(parse_source_node_type).transpose()?;

    if resolved_source_node_id.is_none() {
        if let Some(capability_id) = resolved_capability_id.clone() {
            resolved_source_node_id = Some(capability_id);
            resolved_source_node_type = Some(HierarchyNodeType::Capability);
        } else if let Some(module_id) = resolved_module_id.clone() {
            resolved_source_node_id = Some(module_id);
            resolved_source_node_type = Some(HierarchyNodeType::Module);
        } else if let Some(parent_work_item_id) = parent_work_item_id {
            let inherited = inherit_source_from_parent(pool, parent_work_item_id).await?;
            resolved_module_id = inherited.0;
            resolved_capability_id = inherited.1;
            resolved_source_node_id = inherited.2;
            resolved_source_node_type = inherited.3;
        }
    }

    match (
        resolved_source_node_id.as_deref(),
        resolved_source_node_type,
    ) {
        (Some(node_id), Some(HierarchyNodeType::Module)) => {
            let module_row = sqlx::query("SELECT product_id FROM modules WHERE id = ?")
                .bind(node_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Module {node_id} not found")))?;
            let scoped_product_id: String = module_row.get("product_id");
            if scoped_product_id != product_id {
                return Err(AppError::Validation(
                    "Work item source node must belong to the selected product.".to_string(),
                ));
            }
            resolved_module_id = Some(node_id.to_string());
            resolved_capability_id = None;
        }
        (Some(node_id), Some(HierarchyNodeType::Capability)) => {
            let capability_row = sqlx::query(
                "SELECT c.module_id, m.product_id FROM capabilities c JOIN modules m ON m.id = c.module_id WHERE c.id = ?",
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
            resolved_module_id = Some(capability_row.get("module_id"));
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
        resolved_module_id,
        resolved_capability_id,
        resolved_source_node_id,
        resolved_source_node_type,
    ))
}

pub async fn create_work_item(
    pool: &SqlitePool,
    id: &str,
    product_id: &str,
    module_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    parent_work_item_id: Option<&str>,
    title: &str,
    problem_statement: &str,
    description: &str,
    acceptance_criteria: &str,
    constraints: &str,
    work_item_type: &str,
    priority: &str,
    complexity: &str,
) -> Result<WorkItem, AppError> {
    let (module_id, capability_id, source_node_id, source_node_type) = resolve_source_scope(
        pool,
        product_id,
        module_id,
        capability_id,
        source_node_id,
        source_node_type,
        parent_work_item_id,
    )
    .await?;
    debug!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, title = %title, "persist create_work_item");
    let next_sort_order: i64 = if let Some(parent_id) = parent_work_item_id {
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
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE product_id = ? AND module_id IS NULL AND capability_id IS NULL AND parent_work_item_id IS NULL")
            .bind(product_id)
            .fetch_one(pool)
            .await?
    };
    trace!(work_item_id = %id, sort_order = next_sort_order, "resolved work item sort order");
    let result = sqlx::query_as::<_, WorkItem>("INSERT INTO work_items (id,product_id,module_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,sort_order) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id,product_id,module_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at")
        .bind(id).bind(product_id).bind(&module_id).bind(&capability_id).bind(&source_node_id).bind(source_node_type).bind(parent_work_item_id).bind(title).bind(problem_statement).bind(description).bind(acceptance_criteria).bind(constraints).bind(work_item_type).bind(priority).bind(complexity).bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, error = %err, "persist create_work_item failed");
    }
    result
}

pub async fn get_work_item(pool: &SqlitePool, id: &str) -> Result<WorkItem, AppError> {
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,module_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Work item {id} not found")))
}

pub async fn list_work_items(
    pool: &SqlitePool,
    product_id: Option<&str>,
    module_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<WorkItem>, AppError> {
    trace!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, status = ?status, "persist list_work_items");
    let mut query = String::from("SELECT id,product_id,module_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE 1=1");
    if product_id.is_some() {
        query.push_str(" AND product_id = ?");
    }
    if module_id.is_some() {
        query.push_str(" AND module_id = ?");
    }
    if capability_id.is_some() {
        query.push_str(" AND capability_id = ?");
    }
    if source_node_id.is_some() {
        query.push_str(" AND source_node_id = ?");
    }
    if source_node_type.is_some() {
        query.push_str(" AND source_node_type = ?");
    }
    if status.is_some() {
        query.push_str(" AND status = ?");
    }
    query.push_str(" ORDER BY sort_order, created_at DESC");

    let mut q = sqlx::query_as::<_, WorkItem>(&query);
    if let Some(v) = product_id {
        q = q.bind(v);
    }
    if let Some(v) = module_id {
        q = q.bind(v);
    }
    if let Some(v) = capability_id {
        q = q.bind(v);
    }
    if let Some(v) = source_node_id {
        q = q.bind(v);
    }
    if let Some(v) = source_node_type {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    q.fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn summarize_work_items_by_product(
    pool: &SqlitePool,
) -> Result<Vec<ProductWorkItemSummary>, AppError> {
    sqlx::query_as::<_, ProductWorkItemSummary>(
        "SELECT product_id, COUNT(*) as total_count,
         SUM(CASE WHEN status NOT IN ('done', 'cancelled') THEN 1 ELSE 0 END) as active_count
         FROM work_items
         WHERE product_id IS NOT NULL
         GROUP BY product_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn update_work_item(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    problem_statement: Option<&str>,
    acceptance_criteria: Option<&str>,
    constraints: Option<&str>,
) -> Result<WorkItem, AppError> {
    debug!(work_item_id = %id, "persist update_work_item");
    let existing = get_work_item(pool, id).await?;
    let title = title.unwrap_or(&existing.title);
    let description = description.unwrap_or(&existing.description);
    let existing_status = existing.status.to_string();
    let status = status.unwrap_or(&existing_status);
    let problem_statement = problem_statement.unwrap_or(&existing.problem_statement);
    let acceptance_criteria = acceptance_criteria.unwrap_or(&existing.acceptance_criteria);
    let constraints = constraints.unwrap_or(&existing.constraints);
    sqlx::query("UPDATE work_items SET title=?,description=?,status=?,problem_statement=?,acceptance_criteria=?,constraints=?,updated_at=datetime('now') WHERE id=?")
        .bind(title).bind(description).bind(status).bind(problem_statement).bind(acceptance_criteria).bind(constraints).bind(id)
        .execute(pool).await?;
    get_work_item(pool, id).await
}

pub async fn delete_work_item(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    debug!(work_item_id = %id, "persist delete_work_item");
    sqlx::query("DELETE FROM work_items WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_sub_work_items(
    pool: &SqlitePool,
    parent_work_item_id: &str,
) -> Result<Vec<WorkItem>, AppError> {
    trace!(parent_work_item_id = %parent_work_item_id, "persist get_sub_work_items");
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,module_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE parent_work_item_id=? ORDER BY sort_order")
        .bind(parent_work_item_id)
        .fetch_all(pool).await.map_err(|e| e.into())
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

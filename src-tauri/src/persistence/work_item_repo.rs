use crate::domain::work_item::WorkItem;
use crate::error::AppError;
use sqlx::SqlitePool;
use tracing::{debug, error, trace};

pub async fn create_work_item(
    pool: &SqlitePool,
    id: &str,
    product_id: &str,
    module_id: Option<&str>,
    capability_id: Option<&str>,
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
    debug!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, parent_work_item_id = ?parent_work_item_id, title = %title, "persist create_work_item");
    let next_sort_order: i64 = if let Some(parent_id) = parent_work_item_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE parent_work_item_id = ?")
            .bind(parent_id)
            .fetch_one(pool)
            .await?
    } else if let Some(capability_id) = capability_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE capability_id = ? AND parent_work_item_id IS NULL")
            .bind(capability_id)
            .fetch_one(pool)
            .await?
    } else if let Some(module_id) = module_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE module_id = ? AND capability_id IS NULL AND parent_work_item_id IS NULL")
            .bind(module_id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE product_id = ? AND module_id IS NULL AND capability_id IS NULL AND parent_work_item_id IS NULL")
            .bind(product_id)
            .fetch_one(pool)
            .await?
    };
    trace!(work_item_id = %id, sort_order = next_sort_order, "resolved work item sort order");
    let result = sqlx::query_as::<_, WorkItem>("INSERT INTO work_items (id,product_id,module_id,capability_id,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,sort_order) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id,product_id,module_id,capability_id,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at")
        .bind(id).bind(product_id).bind(module_id).bind(capability_id).bind(parent_work_item_id).bind(title).bind(problem_statement).bind(description).bind(acceptance_criteria).bind(constraints).bind(work_item_type).bind(priority).bind(complexity).bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(work_item_id = %id, product_id = %product_id, module_id = ?module_id, capability_id = ?capability_id, parent_work_item_id = ?parent_work_item_id, error = %err, "persist create_work_item failed");
    }
    result
}

pub async fn get_work_item(pool: &SqlitePool, id: &str) -> Result<WorkItem, AppError> {
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,module_id,capability_id,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Work item {id} not found")))
}

pub async fn list_work_items(
    pool: &SqlitePool,
    product_id: Option<&str>,
    module_id: Option<&str>,
    capability_id: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<WorkItem>, AppError> {
    trace!(product_id = ?product_id, module_id = ?module_id, capability_id = ?capability_id, status = ?status, "persist list_work_items");
    let mut query = String::from("SELECT id,product_id,module_id,capability_id,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE 1=1");
    if product_id.is_some() {
        query.push_str(" AND product_id = ?");
    }
    if module_id.is_some() {
        query.push_str(" AND module_id = ?");
    }
    if capability_id.is_some() {
        query.push_str(" AND capability_id = ?");
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
    if let Some(v) = status {
        q = q.bind(v);
    }
    q.fetch_all(pool).await.map_err(|e| e.into())
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
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,module_id,capability_id,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE parent_work_item_id=? ORDER BY sort_order")
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

use crate::domain::product::HierarchyNodeType;
use crate::domain::work_item::{ProductWorkItemSummary, WorkItem};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
use tracing::{debug, error, trace};

pub const DEFAULT_LIST_WORK_ITEMS_LIMIT: i64 = 500;
pub const MAX_LIST_WORK_ITEMS_LIMIT: i64 = 2_000;

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
                .ok_or_else(|| AppError::NotFound(format!("ProductArea {node_id} not found")))?;
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
    id: &str,
    product_id: &str,
    product_area_id: Option<&str>,
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
    let work_item_type = normalize_work_item_type(work_item_type)?;
    let (product_area_id, capability_id, source_node_id, source_node_type) = resolve_source_scope(
        pool,
        product_id,
        product_area_id,
        capability_id,
        source_node_id,
        source_node_type,
        parent_work_item_id,
    )
    .await?;
    debug!(work_item_id = %id, product_id = %product_id, product_area_id = ?product_area_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, title = %title, "persist create_work_item");
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
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM work_items WHERE product_id = ? AND product_area_id IS NULL AND capability_id IS NULL AND parent_work_item_id IS NULL")
            .bind(product_id)
            .fetch_one(pool)
            .await?
    };
    trace!(work_item_id = %id, sort_order = next_sort_order, "resolved work item sort order");
    let result = sqlx::query_as::<_, WorkItem>("INSERT INTO work_items (id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,sort_order) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at")
        .bind(id).bind(product_id).bind(&product_area_id).bind(&capability_id).bind(&source_node_id).bind(source_node_type).bind(parent_work_item_id).bind(title).bind(problem_statement).bind(description).bind(acceptance_criteria).bind(constraints).bind(&work_item_type).bind(priority).bind(complexity).bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(work_item_id = %id, product_id = %product_id, product_area_id = ?product_area_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, parent_work_item_id = ?parent_work_item_id, error = %err, "persist create_work_item failed");
    }
    result
}

pub async fn get_work_item(pool: &SqlitePool, id: &str) -> Result<WorkItem, AppError> {
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Work item {id} not found")))
}

pub async fn list_work_items(
    pool: &SqlitePool,
    product_id: Option<&str>,
    product_area_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<WorkItem>, AppError> {
    list_work_items_internal(
        pool,
        product_id,
        product_area_id,
        capability_id,
        source_node_id,
        source_node_type,
        status,
        None,
    )
    .await
}

pub async fn list_work_items_page(
    pool: &SqlitePool,
    product_id: Option<&str>,
    product_area_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    status: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<WorkItem>, AppError> {
    let limit = limit
        .unwrap_or(DEFAULT_LIST_WORK_ITEMS_LIMIT)
        .clamp(1, MAX_LIST_WORK_ITEMS_LIMIT);
    let offset = offset.unwrap_or(0).max(0);
    list_work_items_internal(
        pool,
        product_id,
        product_area_id,
        capability_id,
        source_node_id,
        source_node_type,
        status,
        Some((limit, offset)),
    )
    .await
}

async fn list_work_items_internal(
    pool: &SqlitePool,
    product_id: Option<&str>,
    product_area_id: Option<&str>,
    capability_id: Option<&str>,
    source_node_id: Option<&str>,
    source_node_type: Option<&str>,
    status: Option<&str>,
    page: Option<(i64, i64)>,
) -> Result<Vec<WorkItem>, AppError> {
    let normalized_source_node_type = source_node_type
        .map(normalize_source_node_type)
        .transpose()?;
    trace!(product_id = ?product_id, product_area_id = ?product_area_id, capability_id = ?capability_id, source_node_id = ?source_node_id, source_node_type = ?source_node_type, status = ?status, page = ?page, "persist list_work_items");
    let mut query = String::from("SELECT id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE 1=1");
    if product_id.is_some() {
        query.push_str(" AND product_id = ?");
    }
    if product_area_id.is_some() {
        query.push_str(" AND product_area_id = ?");
    }
    if capability_id.is_some() {
        query.push_str(" AND capability_id = ?");
    }
    if source_node_id.is_some() {
        query.push_str(" AND source_node_id = ?");
    }
    if normalized_source_node_type.is_some() {
        query.push_str(" AND source_node_type = ?");
    }
    if status.is_some() {
        query.push_str(" AND status = ?");
    }
    query.push_str(" ORDER BY sort_order, created_at DESC");
    if page.is_some() {
        query.push_str(" LIMIT ? OFFSET ?");
    }

    let mut q = sqlx::query_as::<_, WorkItem>(&query);
    if let Some(v) = product_id {
        q = q.bind(v);
    }
    if let Some(v) = product_area_id {
        q = q.bind(v);
    }
    if let Some(v) = capability_id {
        q = q.bind(v);
    }
    if let Some(v) = source_node_id {
        q = q.bind(v);
    }
    if let Some(v) = normalized_source_node_type {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some((limit, offset)) = page {
        q = q.bind(limit).bind(offset);
    }
    q.fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn summarize_work_items_by_product(
    pool: &SqlitePool,
) -> Result<Vec<ProductWorkItemSummary>, AppError> {
    sqlx::query_as::<_, ProductWorkItemSummary>(
        "SELECT product_id, COUNT(*) as total_count,
         SUM(CASE WHEN status NOT IN ('done', 'cancelled') THEN 1 ELSE 0 END) as active_count,
         SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) as done_count,
         SUM(CASE WHEN status IN ('blocked', 'failed') THEN 1 ELSE 0 END) as blocked_count
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
    sqlx::query_as::<_, WorkItem>("SELECT id,product_id,product_area_id,capability_id,source_node_id,source_node_type,parent_work_item_id,title,problem_statement,description,acceptance_criteria,constraints,work_item_type,priority,complexity,status,repo_override_id,active_repo_id,branch_name,sort_order,created_at,updated_at FROM work_items WHERE parent_work_item_id=? ORDER BY sort_order")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{db as db_service, product_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aruvi_work_item_repo_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ))
    }

    async fn create_test_pool(name: &str) -> SqlitePool {
        let temp_root = make_temp_dir(name);
        std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
        let db_path = temp_root.join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());
        db_service::create_pool(&database_url)
            .await
            .expect("test database should be created")
    }

    async fn create_test_product(pool: &SqlitePool, product_id: &str) {
        product_repo::create_product(
            pool,
            product_id,
            "Test Product",
            "",
            "",
            "[]",
            "[]",
            Some("active"),
            Some("healthy"),
            None,
            Some("invest"),
            None,
            None,
        )
        .await
        .expect("product should be created");
    }

    #[tokio::test]
    async fn list_work_items_page_applies_limit_and_offset() {
        let pool = create_test_pool("list_page").await;
        create_test_product(&pool, "product-work-page").await;

        for index in 0..5 {
            create_work_item(
                &pool,
                &format!("work-item-{index}"),
                "product-work-page",
                None,
                None,
                None,
                None,
                None,
                &format!("Work item {index}"),
                "",
                "",
                "",
                "",
                "story",
                "medium",
                "medium",
            )
            .await
            .expect("work item should be created");
        }

        let page = list_work_items_page(
            &pool,
            Some("product-work-page"),
            None,
            None,
            None,
            None,
            None,
            Some(2),
            Some(1),
        )
        .await
        .expect("page should load");

        assert_eq!(page.len(), 2);
        assert_eq!(page[0].title, "Work item 1");
        assert_eq!(page[1].title, "Work item 2");
    }

    #[tokio::test]
    async fn summarize_work_items_by_product_counts_total_active_and_done() {
        let pool = create_test_pool("summarize_counts").await;
        create_test_product(&pool, "product-summary-counts").await;

        for (index, status) in ["draft", "done", "cancelled", "blocked"].iter().enumerate() {
            let work_item_id = format!("summary-work-item-{index}");
            create_work_item(
                &pool,
                &work_item_id,
                "product-summary-counts",
                None,
                None,
                None,
                None,
                None,
                &format!("Summary work item {index}"),
                "",
                "",
                "",
                "",
                "story",
                "medium",
                "medium",
            )
            .await
            .expect("work item should be created");
            update_work_item(
                &pool,
                &work_item_id,
                None,
                None,
                Some(status),
                None,
                None,
                None,
            )
            .await
            .expect("status should update");
        }

        let summaries = summarize_work_items_by_product(&pool)
            .await
            .expect("summaries should load");
        let summary = summaries
            .iter()
            .find(|entry| entry.product_id == "product-summary-counts")
            .expect("product summary should be present");

        assert_eq!(summary.total_count, 4);
        assert_eq!(summary.active_count, 2);
        assert_eq!(summary.done_count, 1);
        assert_eq!(summary.blocked_count, 1);
    }

    #[tokio::test]
    async fn get_sub_work_items_returns_all_child_tasks_for_story() {
        let pool = create_test_pool("sub_work_items").await;
        create_test_product(&pool, "product-sub-work").await;

        create_work_item(
            &pool,
            "story-with-two-tasks",
            "product-sub-work",
            None,
            None,
            None,
            None,
            None,
            "Story with two tasks",
            "",
            "",
            "",
            "",
            "story",
            "high",
            "medium",
        )
        .await
        .expect("story should be created");

        for (index, title) in ["First task", "Second task"].iter().enumerate() {
            create_work_item(
                &pool,
                &format!("story-child-task-{index}"),
                "product-sub-work",
                None,
                None,
                None,
                None,
                Some("story-with-two-tasks"),
                title,
                "",
                "",
                "",
                "",
                "task",
                "medium",
                "medium",
            )
            .await
            .expect("task should be created");
        }

        let tasks = get_sub_work_items(&pool, "story-with-two-tasks")
            .await
            .expect("child tasks should load");

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "First task");
        assert_eq!(tasks[1].title, "Second task");
        assert!(tasks
            .iter()
            .all(|task| task.parent_work_item_id.as_deref() == Some("story-with-two-tasks")));
    }

    #[tokio::test]
    async fn create_work_item_resolves_capability_scope_and_story_type() {
        let pool = create_test_pool("normalize_scope").await;
        create_test_product(&pool, "product-normalize").await;
        product_repo::create_product_area(
            &pool,
            "product_area-normalize",
            "product-normalize",
            "Operations",
            "",
            "",
            Some("product_area"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("product_area should be created");
        product_repo::create_capability(
            &pool,
            "capability-normalize",
            "product_area-normalize",
            None,
            "Checkout",
            "",
            "",
            "medium",
            "low",
            "",
            Some("capability"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("capability should be created");

        let work_item = create_work_item(
            &pool,
            "work-item-normalize",
            "product-normalize",
            None,
            Some("capability-normalize"),
            None,
            None,
            None,
            "Normalized story",
            "",
            "",
            "",
            "",
            "story",
            "medium",
            "medium",
        )
        .await
        .expect("work item should be created");

        assert_eq!(
            work_item.product_area_id.as_deref(),
            Some("product_area-normalize")
        );
        assert_eq!(
            work_item.capability_id.as_deref(),
            Some("capability-normalize")
        );
        assert_eq!(
            work_item.source_node_id.as_deref(),
            Some("capability-normalize")
        );
        assert!(matches!(
            work_item.source_node_type,
            Some(HierarchyNodeType::Capability)
        ));
        assert!(matches!(
            work_item.work_item_type,
            crate::domain::work_item::WorkItemType::Story
        ));
    }

    #[tokio::test]
    async fn create_child_work_item_inherits_parent_source_scope() {
        let pool = create_test_pool("inherit_scope").await;
        create_test_product(&pool, "product-inherit").await;
        product_repo::create_product_area(
            &pool,
            "product_area-inherit",
            "product-inherit",
            "Platform",
            "",
            "",
            Some("product_area"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("product_area should be created");
        product_repo::create_capability(
            &pool,
            "capability-inherit",
            "product_area-inherit",
            None,
            "Runtime",
            "",
            "",
            "medium",
            "low",
            "",
            Some("capability"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("capability should be created");

        create_work_item(
            &pool,
            "parent-story",
            "product-inherit",
            None,
            Some("capability-inherit"),
            None,
            None,
            None,
            "Parent story",
            "",
            "",
            "",
            "",
            "story",
            "high",
            "medium",
        )
        .await
        .expect("parent work item should be created");

        let child = create_work_item(
            &pool,
            "child-task",
            "product-inherit",
            None,
            None,
            None,
            None,
            Some("parent-story"),
            "Child task",
            "",
            "",
            "",
            "",
            "task",
            "medium",
            "low",
        )
        .await
        .expect("child work item should be created");

        assert_eq!(child.parent_work_item_id.as_deref(), Some("parent-story"));
        assert_eq!(
            child.product_area_id.as_deref(),
            Some("product_area-inherit")
        );
        assert_eq!(child.capability_id.as_deref(), Some("capability-inherit"));
        assert_eq!(child.source_node_id.as_deref(), Some("capability-inherit"));
        assert!(matches!(
            child.source_node_type,
            Some(HierarchyNodeType::Capability)
        ));
    }

    #[tokio::test]
    async fn create_work_item_rejects_source_node_id_without_type() {
        let pool = create_test_pool("source_type_validation").await;
        create_test_product(&pool, "product-source-validation").await;

        let error = create_work_item(
            &pool,
            "work-item-invalid-source",
            "product-source-validation",
            None,
            None,
            Some("product_area-missing-type"),
            None,
            None,
            "Invalid source",
            "",
            "",
            "",
            "",
            "story",
            "medium",
            "medium",
        )
        .await
        .expect_err("missing source node type should fail validation");

        assert!(
            matches!(error, AppError::Validation(message) if message == "source_node_type is required when source_node_id is provided.")
        );
    }
}

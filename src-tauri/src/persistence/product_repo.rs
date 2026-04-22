use crate::domain::product::{
    Capability, CapabilityTree, HierarchyNodeKind, HierarchyNodeType, HierarchyTreeNode, Module,
    ModuleTree, Product, ProductTree,
};
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
use tracing::{debug, error, trace};

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
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn parse_node_kind(value: &str) -> Result<HierarchyNodeKind, AppError> {
    HierarchyNodeKind::parse(value).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported hierarchy node kind '{value}'. Use area, domain, subdomain, system, subsystem, feature_set, capability, rollout, or reference."
        ))
    })
}

fn resolve_root_node_kind(node_kind: Option<&str>) -> Result<HierarchyNodeKind, AppError> {
    let kind = node_kind
        .map(parse_node_kind)
        .transpose()?
        .unwrap_or_else(HierarchyNodeKind::default_root);
    if !kind.is_root_kind() {
        return Err(AppError::Validation(
            "Root product sections must use area, domain, or system.".to_string(),
        ));
    }
    Ok(kind)
}

fn resolve_child_node_kind(
    parent_kind: HierarchyNodeKind,
    node_kind: Option<&str>,
) -> Result<HierarchyNodeKind, AppError> {
    if !parent_kind.can_have_children() {
        return Err(AppError::Validation(format!(
            "{} nodes cannot contain structural children.",
            parent_kind
        )));
    }
    let child_kind = node_kind
        .map(parse_node_kind)
        .transpose()?
        .unwrap_or_else(|| HierarchyNodeKind::default_child(&parent_kind));
    if !parent_kind.supports_child_kind(&child_kind) {
        return Err(AppError::Validation(format!(
            "{} cannot contain {}.",
            parent_kind, child_kind
        )));
    }
    Ok(child_kind)
}

async fn get_module_node_kind(
    pool: &SqlitePool,
    module_id: &str,
) -> Result<HierarchyNodeKind, AppError> {
    sqlx::query_scalar("SELECT node_kind FROM modules WHERE id = ?")
        .bind(module_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Module {module_id} not found")))
        .and_then(|value: String| parse_node_kind(&value))
}

async fn ensure_capability_children_allowed(
    pool: &SqlitePool,
    capability_id: &str,
    parent_kind: HierarchyNodeKind,
) -> Result<(), AppError> {
    let child_kinds: Vec<String> =
        sqlx::query_scalar("SELECT node_kind FROM capabilities WHERE parent_capability_id = ?")
            .bind(capability_id)
            .fetch_all(pool)
            .await?;
    if child_kinds.is_empty() {
        return Ok(());
    }
    if !parent_kind.can_have_children() {
        return Err(AppError::Validation(format!(
            "{} nodes cannot contain structural children.",
            parent_kind
        )));
    }
    for child_kind in child_kinds {
        let parsed_child_kind = parse_node_kind(&child_kind)?;
        if !parent_kind.supports_child_kind(&parsed_child_kind) {
            return Err(AppError::Validation(format!(
                "{} cannot contain existing {} children.",
                parent_kind, parsed_child_kind
            )));
        }
    }
    Ok(())
}

pub async fn create_product(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: &str,
    vision: &str,
    goals: &str,
    tags: &str,
) -> Result<Product, AppError> {
    debug!(product_id = %id, product_name = %name, "persist create_product");
    let result = sqlx::query(r#"INSERT INTO products (id, name, description, vision, goals, tags) VALUES (?, ?, ?, ?, ?, ?) RETURNING id, name, description, vision, goals, tags, status, created_at, updated_at"#)
        .bind(id).bind(name).bind(description).bind(vision).bind(goals).bind(tags)
        .map(row_to_product)
        .fetch_one(pool).await.map_err(|e| e.into());
    if let Err(err) = &result {
        error!(product_id = %id, error = %err, "persist create_product failed");
    }
    result
}

pub async fn get_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    sqlx::query("SELECT id, name, description, vision, goals, tags, status, created_at, updated_at FROM products WHERE id = ?")
        .bind(id)
        .map(row_to_product)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Product {id} not found")))
}

pub async fn list_products(pool: &SqlitePool) -> Result<Vec<Product>, AppError> {
    sqlx::query("SELECT id, name, description, vision, goals, tags, status, created_at, updated_at FROM products ORDER BY created_at DESC")
        .map(row_to_product)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn update_product(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    vision: Option<&str>,
    goals: Option<&str>,
    tags: Option<&str>,
) -> Result<Product, AppError> {
    debug!(product_id = %id, "persist update_product");
    let existing = get_product(pool, id).await?;
    let name = name.unwrap_or(&existing.name);
    let description = description.unwrap_or(&existing.description);
    let vision = vision.unwrap_or(&existing.vision);
    let existing_goals = serde_json::to_string(&existing.goals).unwrap_or_default();
    let existing_tags = serde_json::to_string(&existing.tags).unwrap_or_default();
    let goals_str = goals.unwrap_or(&existing_goals);
    let tags_str = tags.unwrap_or(&existing_tags);
    sqlx::query("UPDATE products SET name=?, description=?, vision=?, goals=?, tags=?, updated_at=datetime('now') WHERE id=?")
        .bind(name).bind(description).bind(vision).bind(goals_str).bind(tags_str).bind(id)
        .execute(pool).await?;
    get_product(pool, id).await
}

pub async fn archive_product(pool: &SqlitePool, id: &str) -> Result<Product, AppError> {
    sqlx::query("UPDATE products SET status='archived', updated_at=datetime('now') WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    get_product(pool, id).await
}

pub async fn create_module(
    pool: &SqlitePool,
    id: &str,
    product_id: &str,
    name: &str,
    description: &str,
    purpose: &str,
    node_kind: Option<&str>,
) -> Result<Module, AppError> {
    debug!(module_id = %id, product_id = %product_id, module_name = %name, "persist create_module");
    let node_kind = resolve_root_node_kind(node_kind)?;
    let next_sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM modules WHERE product_id = ?",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;
    trace!(module_id = %id, product_id = %product_id, sort_order = next_sort_order, "resolved module sort order");
    sqlx::query_as::<_, Module>("INSERT INTO modules (id, product_id, node_kind, name, description, purpose, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id, product_id, node_kind, name, description, purpose, sort_order, created_at, updated_at")
        .bind(id).bind(product_id).bind(node_kind).bind(name).bind(description).bind(purpose).bind(next_sort_order)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_modules(pool: &SqlitePool, product_id: &str) -> Result<Vec<Module>, AppError> {
    sqlx::query_as::<_, Module>("SELECT id, product_id, node_kind, name, description, purpose, sort_order, created_at, updated_at FROM modules WHERE product_id=? ORDER BY sort_order")
        .bind(product_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn update_module(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    purpose: Option<&str>,
    node_kind: Option<&str>,
) -> Result<Module, AppError> {
    debug!(module_id = %id, "persist update_module");
    let existing = sqlx::query_as::<_, Module>("SELECT id, product_id, node_kind, name, description, purpose, sort_order, created_at, updated_at FROM modules WHERE id=?")
        .bind(id)
        .fetch_optional(pool).await?.ok_or_else(|| AppError::NotFound(format!("Module {id} not found")))?;
    let name = name.unwrap_or(&existing.name);
    let description = description.unwrap_or(&existing.description);
    let purpose = purpose.unwrap_or(&existing.purpose);
    let node_kind = if let Some(value) = node_kind {
        resolve_root_node_kind(Some(value))?
    } else {
        existing.node_kind
    };
    sqlx::query("UPDATE modules SET name=?, description=?, purpose=?, node_kind=?, updated_at=datetime('now') WHERE id=?")
        .bind(name).bind(description).bind(purpose).bind(node_kind).bind(id).execute(pool).await?;
    sqlx::query_as::<_, Module>("SELECT id, product_id, node_kind, name, description, purpose, sort_order, created_at, updated_at FROM modules WHERE id=?")
        .bind(id)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn delete_module(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM modules WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_capability(
    pool: &SqlitePool,
    id: &str,
    module_id: &str,
    parent_capability_id: Option<&str>,
    name: &str,
    description: &str,
    acceptance_criteria: &str,
    priority: &str,
    risk: &str,
    technical_notes: &str,
    node_kind: Option<&str>,
) -> Result<Capability, AppError> {
    debug!(capability_id = %id, module_id = %module_id, parent_capability_id = ?parent_capability_id, capability_name = %name, "persist create_capability");
    let (level, parent_kind) = if let Some(parent_id) = parent_capability_id {
        let parent =
            sqlx::query("SELECT level, module_id, node_kind FROM capabilities WHERE id = ?")
                .bind(parent_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Capability {parent_id} not found")))?;
        let parent_module_id: String = parent.get("module_id");
        if parent_module_id != module_id {
            return Err(AppError::Validation(
                "Capability children must stay inside the same root product section.".to_string(),
            ));
        }
        let parent_kind = parse_node_kind(parent.get::<String, _>("node_kind").as_str())?;
        (parent.get::<i64, _>("level") + 1, parent_kind)
    } else {
        (0, get_module_node_kind(pool, module_id).await?)
    };
    let node_kind = resolve_child_node_kind(parent_kind, node_kind)?;
    trace!(capability_id = %id, level = level, "resolved capability level");
    let next_sort_order: i64 = if let Some(parent_id) = parent_capability_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM capabilities WHERE module_id = ? AND parent_capability_id = ?")
            .bind(module_id)
            .bind(parent_id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM capabilities WHERE module_id = ? AND parent_capability_id IS NULL")
            .bind(module_id)
            .fetch_one(pool)
            .await?
    };
    trace!(capability_id = %id, sort_order = next_sort_order, "resolved capability sort order");
    sqlx::query_as::<_, Capability>("INSERT INTO capabilities (id, module_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, priority, risk, technical_notes) VALUES (?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id, module_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, priority, risk, status, technical_notes, created_at, updated_at")
        .bind(id).bind(module_id).bind(parent_capability_id).bind(level).bind(node_kind).bind(next_sort_order).bind(name).bind(description).bind(acceptance_criteria).bind(priority).bind(risk).bind(technical_notes)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn list_capabilities(
    pool: &SqlitePool,
    module_id: &str,
) -> Result<Vec<Capability>, AppError> {
    sqlx::query_as::<_, Capability>("SELECT id, module_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, priority, risk, status, technical_notes, created_at, updated_at FROM capabilities WHERE module_id=? ORDER BY sort_order, name")
        .bind(module_id)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub async fn update_capability(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    acceptance_criteria: Option<&str>,
    priority: Option<&str>,
    risk: Option<&str>,
    technical_notes: Option<&str>,
    node_kind: Option<&str>,
) -> Result<Capability, AppError> {
    debug!(capability_id = %id, "persist update_capability");
    let existing = sqlx::query_as::<_, Capability>(
        "SELECT id, module_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, priority, risk, status, technical_notes, created_at, updated_at FROM capabilities WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Capability {id} not found")))?;

    let name = name.unwrap_or(&existing.name);
    let description = description.unwrap_or(&existing.description);
    let acceptance_criteria = acceptance_criteria.unwrap_or(&existing.acceptance_criteria);
    let existing_priority = existing.priority.to_string();
    let existing_risk = existing.risk.to_string();
    let priority = priority.unwrap_or(&existing_priority);
    let risk = risk.unwrap_or(&existing_risk);
    let technical_notes = technical_notes.unwrap_or(&existing.technical_notes);
    let parent_kind = if let Some(parent_capability_id) = existing.parent_capability_id.as_deref() {
        let parent_node_kind: String =
            sqlx::query_scalar("SELECT node_kind FROM capabilities WHERE id = ?")
                .bind(parent_capability_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Capability {parent_capability_id} not found"))
                })?;
        parse_node_kind(&parent_node_kind)?
    } else {
        get_module_node_kind(pool, &existing.module_id).await?
    };
    let node_kind = if let Some(value) = node_kind {
        resolve_child_node_kind(parent_kind, Some(value))?
    } else {
        existing.node_kind
    };
    ensure_capability_children_allowed(pool, id, node_kind).await?;

    sqlx::query(
        "UPDATE capabilities SET name=?, description=?, acceptance_criteria=?, priority=?, risk=?, technical_notes=?, node_kind=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(name)
    .bind(description)
    .bind(acceptance_criteria)
    .bind(priority)
    .bind(risk)
    .bind(technical_notes)
    .bind(node_kind)
    .bind(id)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, Capability>(
        "SELECT id, module_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, priority, risk, status, technical_notes, created_at, updated_at FROM capabilities WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn reorder_modules(
    pool: &SqlitePool,
    product_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    debug!(product_id = %product_id, item_count = ordered_ids.len(), "persist reorder_modules");
    for (index, id) in ordered_ids.iter().enumerate() {
        sqlx::query("UPDATE modules SET sort_order=?, updated_at=datetime('now') WHERE id=? AND product_id=?")
            .bind(index as i64)
            .bind(id)
            .bind(product_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn reorder_capabilities(
    pool: &SqlitePool,
    module_id: &str,
    parent_capability_id: Option<&str>,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    debug!(module_id = %module_id, parent_capability_id = ?parent_capability_id, item_count = ordered_ids.len(), "persist reorder_capabilities");
    for (index, id) in ordered_ids.iter().enumerate() {
        let mut query = String::from("UPDATE capabilities SET sort_order=?, updated_at=datetime('now') WHERE id=? AND module_id=?");
        if parent_capability_id.is_some() {
            query.push_str(" AND parent_capability_id=?");
        } else {
            query.push_str(" AND parent_capability_id IS NULL");
        }
        let mut q = sqlx::query(&query)
            .bind(index as i64)
            .bind(id)
            .bind(module_id);
        if let Some(parent_id) = parent_capability_id {
            q = q.bind(parent_id);
        }
        q.execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_capability(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM capabilities WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_product_tree(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<ProductTree, AppError> {
    trace!(product_id = %product_id, "persist get_product_tree");
    let product = get_product(pool, product_id).await?;
    let modules = list_modules(pool, product_id).await?;
    let mut module_trees = Vec::new();
    for m in modules {
        let features = list_capabilities(pool, &m.id).await?;
        let root_features: Vec<_> = features
            .iter()
            .filter(|f| f.parent_capability_id.is_none())
            .collect();
        let capability_trees = root_features
            .iter()
            .map(|f| build_capability_tree(f, &features))
            .collect();
        module_trees.push(ModuleTree {
            module: m,
            features: capability_trees,
        });
    }
    let roots = module_trees
        .iter()
        .map(|module_tree| build_module_hierarchy_tree(module_tree))
        .collect();
    Ok(ProductTree {
        product,
        modules: module_trees,
        roots,
    })
}

fn build_module_hierarchy_tree(module_tree: &ModuleTree) -> HierarchyTreeNode {
    let path = vec![module_tree.module.name.clone()];
    let children = module_tree
        .features
        .iter()
        .map(|capability_tree| build_hierarchy_tree(capability_tree, &path))
        .collect();
    HierarchyTreeNode {
        id: module_tree.module.id.clone(),
        node_type: HierarchyNodeType::Module,
        node_kind: module_tree.module.node_kind,
        module_id: module_tree.module.id.clone(),
        capability_id: None,
        parent_node_id: None,
        parent_node_type: None,
        depth: 0,
        name: module_tree.module.name.clone(),
        description: module_tree.module.description.clone(),
        summary: if module_tree.module.description.is_empty() {
            module_tree.module.purpose.clone()
        } else {
            module_tree.module.description.clone()
        },
        path,
        allowed_child_kinds: module_tree.module.node_kind.allowed_child_kinds(),
        children,
    }
}

fn build_hierarchy_tree(
    capability_tree: &CapabilityTree,
    parent_path: &[String],
) -> HierarchyTreeNode {
    let mut path = parent_path.to_vec();
    path.push(capability_tree.capability.name.clone());
    let children = capability_tree
        .children
        .iter()
        .map(|child| build_hierarchy_tree(child, &path))
        .collect();
    HierarchyTreeNode {
        id: capability_tree.capability.id.clone(),
        node_type: HierarchyNodeType::Capability,
        node_kind: capability_tree.capability.node_kind,
        module_id: capability_tree.capability.module_id.clone(),
        capability_id: Some(capability_tree.capability.id.clone()),
        parent_node_id: capability_tree
            .capability
            .parent_capability_id
            .clone()
            .or_else(|| {
                parent_path
                    .first()
                    .map(|_| capability_tree.capability.module_id.clone())
            }),
        parent_node_type: Some(
            if capability_tree.capability.parent_capability_id.is_some() {
                HierarchyNodeType::Capability
            } else {
                HierarchyNodeType::Module
            },
        ),
        depth: capability_tree.capability.level + 1,
        name: capability_tree.capability.name.clone(),
        description: capability_tree.capability.description.clone(),
        summary: capability_tree.capability.description.clone(),
        path,
        allowed_child_kinds: capability_tree.capability.node_kind.allowed_child_kinds(),
        children,
    }
}

fn build_capability_tree(
    capability: &Capability,
    all_capabilities: &[Capability],
) -> CapabilityTree {
    let children: Vec<_> = all_capabilities
        .iter()
        .filter(|f| f.parent_capability_id.as_deref() == Some(&capability.id))
        .map(|f| build_capability_tree(f, all_capabilities))
        .collect();
    CapabilityTree {
        capability: capability.clone(),
        children,
    }
}

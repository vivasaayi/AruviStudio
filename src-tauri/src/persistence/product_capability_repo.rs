use crate::domain::product::{
    Capability, ChildReparentStrategy, HierarchyNodeKind, NodeKindConversionResult,
};
use crate::error::AppError;
use crate::persistence::product_capability_read_repo::{get_capability, CAPABILITY_SELECT_COLUMNS};
use crate::persistence::product_hierarchy_rules::{
    parse_capability_node_kind, parse_root_node_kind, resolve_child_node_kind,
};
use sqlx::{Row, SqlitePool};
use tracing::{debug, trace};

#[derive(Clone, Copy, Debug)]
pub struct CreateCapabilityInput<'a> {
    pub id: &'a str,
    pub product_area_id: &'a str,
    pub parent_capability_id: Option<&'a str>,
    pub name: &'a str,
    pub description: &'a str,
    pub acceptance_criteria: &'a str,
    pub priority: &'a str,
    pub risk: &'a str,
    pub technical_notes: &'a str,
    pub node_kind: Option<&'a str>,
    pub explanation: &'a str,
    pub examples: &'a str,
    pub implementation_notes: &'a str,
    pub test_guidance: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct UpdateCapabilityPatch<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub acceptance_criteria: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub risk: Option<&'a str>,
    pub technical_notes: Option<&'a str>,
    pub node_kind: Option<&'a str>,
    pub explanation: Option<&'a str>,
    pub examples: Option<&'a str>,
    pub implementation_notes: Option<&'a str>,
    pub test_guidance: Option<&'a str>,
}

async fn get_product_area_node_kind(
    pool: &SqlitePool,
    product_area_id: &str,
) -> Result<HierarchyNodeKind, AppError> {
    sqlx::query_scalar("SELECT node_kind FROM product_areas WHERE id = ?")
        .bind(product_area_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product Area {product_area_id} not found")))
        .and_then(|value: String| parse_root_node_kind(&value))
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
        let parsed_child_kind = parse_capability_node_kind(&child_kind)?;
        if !parent_kind.supports_child_kind(&parsed_child_kind) {
            return Err(AppError::Validation(format!(
                "{} cannot contain existing {} children.",
                parent_kind, parsed_child_kind
            )));
        }
    }
    Ok(())
}

async fn update_capability_subtree_levels(
    pool: &SqlitePool,
    capability_id: &str,
    level: i64,
) -> Result<(), AppError> {
    let mut pending = vec![(capability_id.to_string(), level)];
    while let Some((current_id, current_level)) = pending.pop() {
        sqlx::query("UPDATE capabilities SET level=?, updated_at=datetime('now') WHERE id=?")
            .bind(current_level)
            .bind(&current_id)
            .execute(pool)
            .await?;
        let child_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM capabilities WHERE parent_capability_id=?")
                .bind(&current_id)
                .fetch_all(pool)
                .await?;
        for child_id in child_ids {
            pending.push((child_id, current_level + 1));
        }
    }
    Ok(())
}

pub async fn create_capability(
    pool: &SqlitePool,
    input: CreateCapabilityInput<'_>,
) -> Result<Capability, AppError> {
    debug!(capability_id = %input.id, product_area_id = %input.product_area_id, parent_capability_id = ?input.parent_capability_id, capability_name = %input.name, "persist create_capability");
    let (level, parent_kind) = if let Some(parent_id) = input.parent_capability_id {
        let parent =
            sqlx::query("SELECT level, product_area_id, node_kind FROM capabilities WHERE id = ?")
                .bind(parent_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Capability {parent_id} not found")))?;
        let parent_product_area_id: String = parent.get("product_area_id");
        if parent_product_area_id != input.product_area_id {
            return Err(AppError::Validation(
                "Capability children must stay inside the same root product section.".to_string(),
            ));
        }
        let parent_kind =
            parse_capability_node_kind(parent.get::<String, _>("node_kind").as_str())?;
        (parent.get::<i64, _>("level") + 1, parent_kind)
    } else {
        (
            0,
            get_product_area_node_kind(pool, input.product_area_id).await?,
        )
    };
    let node_kind = resolve_child_node_kind(parent_kind, input.node_kind)?;
    trace!(capability_id = %input.id, level = level, "resolved capability level");
    let next_sort_order: i64 = if let Some(parent_id) = input.parent_capability_id {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM capabilities WHERE product_area_id = ? AND parent_capability_id = ?")
            .bind(input.product_area_id)
            .bind(parent_id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM capabilities WHERE product_area_id = ? AND parent_capability_id IS NULL")
            .bind(input.product_area_id)
            .fetch_one(pool)
            .await?
    };
    trace!(capability_id = %input.id, sort_order = next_sort_order, "resolved capability sort order");
    sqlx::query_as::<_, Capability>(
        &format!(
            "INSERT INTO capabilities (id, product_area_id, parent_capability_id, level, node_kind, sort_order, name, description, acceptance_criteria, explanation, examples, priority, risk, technical_notes, implementation_notes, test_guidance) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING {CAPABILITY_SELECT_COLUMNS}"
        ),
    )
        .bind(input.id).bind(input.product_area_id).bind(input.parent_capability_id).bind(level).bind(node_kind)
        .bind(next_sort_order).bind(input.name).bind(input.description).bind(input.acceptance_criteria)
        .bind(input.explanation).bind(input.examples).bind(input.priority).bind(input.risk).bind(input.technical_notes)
        .bind(input.implementation_notes).bind(input.test_guidance)
        .fetch_one(pool).await.map_err(|e| e.into())
}

pub async fn update_capability(
    pool: &SqlitePool,
    patch: UpdateCapabilityPatch<'_>,
) -> Result<Capability, AppError> {
    debug!(capability_id = %patch.id, "persist update_capability");
    let existing = sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_SELECT_COLUMNS} FROM capabilities WHERE id=?"
    ))
    .bind(patch.id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Capability {} not found", patch.id)))?;

    let name = patch.name.unwrap_or(&existing.name);
    let description = patch.description.unwrap_or(&existing.description);
    let acceptance_criteria = patch
        .acceptance_criteria
        .unwrap_or(&existing.acceptance_criteria);
    let existing_priority = existing.priority.to_string();
    let existing_risk = existing.risk.to_string();
    let priority = patch.priority.unwrap_or(&existing_priority);
    let risk = patch.risk.unwrap_or(&existing_risk);
    let technical_notes = patch.technical_notes.unwrap_or(&existing.technical_notes);
    let explanation = patch.explanation.unwrap_or(&existing.explanation);
    let examples = patch.examples.unwrap_or(&existing.examples);
    let implementation_notes = patch
        .implementation_notes
        .unwrap_or(&existing.implementation_notes);
    let test_guidance = patch.test_guidance.unwrap_or(&existing.test_guidance);
    let parent_kind = if let Some(parent_capability_id) = existing.parent_capability_id.as_deref() {
        let parent_node_kind: String =
            sqlx::query_scalar("SELECT node_kind FROM capabilities WHERE id = ?")
                .bind(parent_capability_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Capability {parent_capability_id} not found"))
                })?;
        parse_capability_node_kind(&parent_node_kind)?
    } else {
        get_product_area_node_kind(pool, &existing.product_area_id).await?
    };
    let node_kind = if let Some(value) = patch.node_kind {
        resolve_child_node_kind(parent_kind, Some(value))?
    } else {
        existing.node_kind
    };
    ensure_capability_children_allowed(pool, patch.id, node_kind).await?;

    sqlx::query(
        "UPDATE capabilities SET name=?, description=?, acceptance_criteria=?, explanation=?, examples=?, priority=?, risk=?, technical_notes=?, implementation_notes=?, test_guidance=?, node_kind=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(name)
    .bind(description)
    .bind(acceptance_criteria)
    .bind(explanation)
    .bind(examples)
    .bind(priority)
    .bind(risk)
    .bind(technical_notes)
    .bind(implementation_notes)
    .bind(test_guidance)
    .bind(node_kind)
    .bind(patch.id)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_SELECT_COLUMNS} FROM capabilities WHERE id=?"
    ))
    .bind(patch.id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn convert_capability_node_kind(
    pool: &SqlitePool,
    id: &str,
    target_node_kind: &str,
    child_strategy: Option<&str>,
) -> Result<NodeKindConversionResult, AppError> {
    let existing = get_capability(pool, id).await?;
    let previous_node_kind = existing.node_kind;
    let parent_kind = if let Some(parent_capability_id) = existing.parent_capability_id.as_deref() {
        get_capability(pool, parent_capability_id).await?.node_kind
    } else {
        get_product_area_node_kind(pool, &existing.product_area_id).await?
    };
    let next_node_kind = resolve_child_node_kind(parent_kind, Some(target_node_kind))?;
    let strategy = child_strategy
        .map(|value| {
            ChildReparentStrategy::parse(value).ok_or_else(|| {
                AppError::Validation(format!(
                    "Unsupported child strategy '{value}'. Use reject or reparent_to_parent."
                ))
            })
        })
        .transpose()?
        .unwrap_or(ChildReparentStrategy::Reject);

    let direct_children = sqlx::query_as::<_, Capability>(&format!(
        "SELECT {CAPABILITY_SELECT_COLUMNS} FROM capabilities WHERE parent_capability_id=? ORDER BY sort_order, name"
    ))
    .bind(id)
    .fetch_all(pool)
    .await?;

    let reparented_children = if !direct_children.is_empty() && !next_node_kind.can_have_children()
    {
        if strategy != ChildReparentStrategy::ReparentToParent {
            return Err(AppError::Validation(format!(
                "{} cannot contain structural children. Re-run with child_strategy=reparent_to_parent to preserve descendants.",
                next_node_kind
            )));
        }
        for child in &direct_children {
            if !parent_kind.supports_child_kind(&child.node_kind) {
                return Err(AppError::Validation(format!(
                    "{} cannot receive existing {} children from {}.",
                    parent_kind, child.node_kind, existing.name
                )));
            }
        }
        let next_parent_level = existing.level as i64;
        for child in &direct_children {
            sqlx::query(
                "UPDATE capabilities SET parent_capability_id=?, updated_at=datetime('now') WHERE id=?",
            )
            .bind(existing.parent_capability_id.as_deref())
            .bind(&child.id)
            .execute(pool)
            .await?;
            update_capability_subtree_levels(pool, &child.id, next_parent_level).await?;
        }
        direct_children
    } else {
        ensure_capability_children_allowed(pool, id, next_node_kind).await?;
        vec![]
    };

    let capability = update_capability(
        pool,
        UpdateCapabilityPatch {
            id,
            node_kind: Some(target_node_kind),
            name: None,
            description: None,
            acceptance_criteria: None,
            priority: None,
            risk: None,
            technical_notes: None,
            explanation: None,
            examples: None,
            implementation_notes: None,
            test_guidance: None,
        },
    )
    .await?;

    Ok(NodeKindConversionResult {
        capability,
        previous_node_kind,
        child_strategy: if reparented_children.is_empty() {
            None
        } else {
            Some(strategy)
        },
        reparented_children,
    })
}

pub async fn reorder_capabilities(
    pool: &SqlitePool,
    product_area_id: &str,
    parent_capability_id: Option<&str>,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    debug!(product_area_id = %product_area_id, parent_capability_id = ?parent_capability_id, item_count = ordered_ids.len(), "persist reorder_capabilities");
    for (index, id) in ordered_ids.iter().enumerate() {
        let mut query = String::from("UPDATE capabilities SET sort_order=?, updated_at=datetime('now') WHERE id=? AND product_area_id=?");
        if parent_capability_id.is_some() {
            query.push_str(" AND parent_capability_id=?");
        } else {
            query.push_str(" AND parent_capability_id IS NULL");
        }
        let mut q = sqlx::query(&query)
            .bind(index as i64)
            .bind(id)
            .bind(product_area_id);
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

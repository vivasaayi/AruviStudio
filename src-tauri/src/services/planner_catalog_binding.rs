use crate::domain::product::{Capability, Product, ProductArea};
use crate::error::AppError;
use crate::services::planner_action_fields::string_field;
use crate::services::planner_types::PlannerDraftNode;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use sqlx::SqliteConnection;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding<T> {
    kind: String,
    snapshot: T,
}

fn binding<T: DeserializeOwned>(node: &PlannerDraftNode, kind: &str) -> Result<Option<T>, AppError> {
    let Some(value) = node.details.get("_catalog_binding") else { return Ok(None) };
    let binding: Binding<T> = serde_json::from_value(value.clone())
        .map_err(|_| AppError::Validation("Invalid staged catalog binding.".into()))?;
    if binding.kind != kind || node.node_type != kind {
        return Err(AppError::Validation("Staged catalog binding has the wrong node kind.".into()));
    }
    Ok(Some(binding.snapshot))
}

fn field(node: &PlannerDraftNode, camel: &str, snake: &str, fallback: &str) -> String {
    string_field(&node.details, camel).or_else(|| string_field(&node.details, snake)).unwrap_or_else(|| fallback.to_string())
}

fn validate_name(node: &PlannerDraftNode) -> Result<(), AppError> {
    if node.name.trim().is_empty() { return Err(AppError::Validation("Staged catalog node name cannot be empty.".into())); }
    Ok(())
}

pub(crate) fn check_bound_product(node: &PlannerDraftNode, current: Option<&Product>, required: bool) -> Result<(), AppError> {
    let Some(snapshot) = binding::<Product>(node, "product")? else {
        return if required { Err(AppError::Validation("A staged catalog update requires a product snapshot.".into())) } else { Ok(()) };
    };
    let product = current.ok_or_else(|| AppError::Validation("The staged product no longer exists.".into()))?;
    if node.id != snapshot.id || product.id != snapshot.id || serde_json::to_value(product)? != serde_json::to_value(&snapshot)? {
        return Err(AppError::Validation("The staged product changed; refresh and review it before applying.".into()));
    }
    Ok(())
}

pub(crate) async fn apply_bound_area(
    conn: &mut SqliteConnection, node: &PlannerDraftNode, product_id: &str,
) -> Result<Option<ProductArea>, AppError> {
    let Some(snapshot) = binding::<ProductArea>(node, "product_area")? else { return Ok(None) };
    validate_name(node)?;
    if snapshot.id != node.id || snapshot.product_id != product_id {
        return Err(AppError::Validation("Staged product area does not belong to the selected product.".into()));
    }
    let current = sqlx::query_as::<_, ProductArea>("SELECT * FROM product_areas WHERE id = ?")
        .bind(&snapshot.id).fetch_optional(&mut *conn).await?
        .ok_or_else(|| AppError::Validation("The staged product area no longer exists.".into()))?;
    if serde_json::to_value(&current)? != serde_json::to_value(&snapshot)? {
        return Err(AppError::Validation("The staged product area changed; refresh and review it before applying.".into()));
    }
    let kind = current.node_kind.to_string();
    if field(node, "nodeKind", "node_kind", &kind) != kind {
        return Err(AppError::Validation("Changing an existing product area kind is not supported in a staged design.".into()));
    }
    let description = node.summary.as_deref().unwrap_or(&current.description);
    let updated = sqlx::query_as::<_, ProductArea>("UPDATE product_areas SET name=?, description=?, purpose=?, explanation=?, examples=?, implementation_notes=?, test_guidance=?, updated_at=datetime('now') WHERE id=? RETURNING *")
        .bind(node.name.trim()).bind(description)
        .bind(field(node,"purpose","purpose",&current.purpose))
        .bind(field(node,"explanation","explanation",&current.explanation))
        .bind(field(node,"examples","examples",&current.examples))
        .bind(field(node,"implementationNotes","implementation_notes",&current.implementation_notes))
        .bind(field(node,"testGuidance","test_guidance",&current.test_guidance))
        .bind(&snapshot.id).fetch_one(&mut *conn).await?;
    Ok(Some(updated))
}

pub(crate) async fn apply_bound_capability(
    conn: &mut SqliteConnection, node: &PlannerDraftNode, area_id: &str, parent_id: Option<&str>,
) -> Result<Option<Capability>, AppError> {
    let Some(snapshot) = binding::<Capability>(node, "capability")? else { return Ok(None) };
    validate_name(node)?;
    if snapshot.id != node.id || snapshot.product_area_id != area_id || snapshot.parent_capability_id.as_deref() != parent_id {
        return Err(AppError::Validation("Staged capability does not belong to the selected hierarchy.".into()));
    }
    let current = sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE id = ?")
        .bind(&snapshot.id).fetch_optional(&mut *conn).await?
        .ok_or_else(|| AppError::Validation("The staged capability no longer exists.".into()))?;
    if serde_json::to_value(&current)? != serde_json::to_value(&snapshot)? {
        return Err(AppError::Validation("The staged capability changed; refresh and review it before applying.".into()));
    }
    let kind = current.node_kind.to_string();
    if field(node, "nodeKind", "node_kind", &kind) != kind {
        return Err(AppError::Validation("Changing an existing capability kind is not supported in a staged design.".into()));
    }
    let description = node.summary.as_deref().unwrap_or(&current.description);
    let updated = sqlx::query_as::<_, Capability>("UPDATE capabilities SET name=?, description=?, acceptance_criteria=?, explanation=?, examples=?, priority=?, risk=?, technical_notes=?, implementation_notes=?, test_guidance=?, updated_at=datetime('now') WHERE id=? RETURNING *")
        .bind(node.name.trim()).bind(description)
        .bind(field(node,"acceptanceCriteria","acceptance_criteria",&current.acceptance_criteria))
        .bind(field(node,"explanation","explanation",&current.explanation))
        .bind(field(node,"examples","examples",&current.examples))
        .bind(field(node,"priority","priority",&current.priority.to_string()))
        .bind(field(node,"risk","risk",&current.risk.to_string()))
        .bind(field(node,"technicalNotes","technical_notes",&current.technical_notes))
        .bind(field(node,"implementationNotes","implementation_notes",&current.implementation_notes))
        .bind(field(node,"testGuidance","test_guidance",&current.test_guidance))
        .bind(&snapshot.id).fetch_one(&mut *conn).await?;
    Ok(Some(updated))
}

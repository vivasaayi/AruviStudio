use crate::domain::product::{Capability, Product, ProductArea};
use crate::domain::work_item::{WorkItem, WorkItemStatus};
use crate::error::AppError;
use crate::persistence::product_core_repo;
use crate::services::planner_action_fields::string_field;
use crate::services::planner_response::build_session_state_response;
use crate::services::planner_session::{get_or_load_session, lock_planner_session, PlannerService};
use crate::services::planner_turn_policy::seed_draft_with_product;
use crate::services::planner_types::{PlannerDraftNode, PlannerDraftPlan, PlannerTurnResponse};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;

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

fn stage_node(draft: &mut PlannerDraftPlan, proposed: PlannerDraftNode) -> Result<(), AppError> {
    if let Some(existing) = draft.nodes.iter().find(|node| node.id == proposed.id) {
        if existing.node_type != proposed.node_type || existing.parent_id != proposed.parent_id ||
            existing.details.get("_catalog_binding") != proposed.details.get("_catalog_binding") {
            return Err(AppError::Validation("The staged catalog node changed or conflicts with the existing draft; review it before continuing.".into()));
        }
        return Ok(());
    }
    if draft.nodes.iter().any(|node| node.node_type == proposed.node_type && node.parent_id == proposed.parent_id && node.name.trim().to_lowercase() == proposed.name.trim().to_lowercase()) {
        return Err(AppError::Validation("A different staged sibling has the same name; resolve the draft before staging this node.".into()));
    }
    draft.nodes.push(proposed);
    Ok(())
}

/// Stages a snapshot of an existing area or capability and its ancestors.
/// The session lock and database write transaction keep the snapshot and draft
/// update together; an existing staged node is never silently refreshed.
pub(crate) async fn stage_catalog_node(
    planner_service: Arc<Mutex<PlannerService>>, db: &SqlitePool, session_id: String,
    product_id: String, node_type: String, node_id: String,
) -> Result<PlannerTurnResponse, AppError> {
    let _guard = lock_planner_session(&planner_service, db, &session_id).await?;
    let mut session = get_or_load_session(&planner_service, db, &session_id).await?;
    let mut tx = db.begin_with("BEGIN IMMEDIATE").await?;
    let draft = build_staged_catalog_draft(&mut tx, session.draft_plan.clone(), &product_id, &node_type, &node_id).await?;
    session.draft_plan = Some(draft);
    session.selected_draft_node_id = Some(node_id);
    let updated = sqlx::query("UPDATE planner_sessions SET draft_plan_json=?, selected_draft_node_id=?, updated_at=datetime('now') WHERE id=?")
        .bind(serde_json::to_string(&session.draft_plan)?).bind(session.selected_draft_node_id.as_deref()).bind(&session_id).execute(&mut *tx).await?;
    if updated.rows_affected() != 1 { return Err(AppError::NotFound("Planner session not found".into())); }
    tx.commit().await?;
    planner_service.lock().await.save_session(&session_id, session.clone());
    Ok(build_session_state_response(session_id, "session_update", "Existing catalog node staged for review.".into(), &session, vec![], vec![], vec![]))
}

/// Build a staged snapshot under the caller's database transaction. Callers
/// can commit the draft and related session exchange atomically.
pub(crate) async fn build_staged_catalog_draft(
    conn: &mut SqliteConnection, existing_draft: Option<PlannerDraftPlan>,
    product_id: &str, node_type: &str, node_id: &str,
) -> Result<PlannerDraftPlan, AppError> {
    if !matches!(node_type, "product" | "product_area" | "capability" | "feature" | "work_item") {
        return Err(AppError::Validation("Select a product, product area, capability, feature or work item.".into()));
    }
    let product = product_core_repo::get_product_in_conn(&mut *conn, product_id).await?;
    let product = &product;
    if let Some(existing) = existing_draft.as_ref().filter(|draft| !draft.nodes.is_empty()) {
        let roots: Vec<_> = existing.nodes.iter().filter(|node| node.node_type == "product" && node.parent_id.is_none()).collect();
        if roots.len() != 1 || roots[0].id != product.id {
            return Err(AppError::Validation("The current draft belongs to another or unidentified product; review it before staging a catalog node.".into()));
        }
    }
    let mut draft = seed_draft_with_product(existing_draft.clone(), product);
    let roots: Vec<_> = draft.nodes.iter().filter(|node| node.node_type == "product" && node.parent_id.is_none()).collect();
    if roots.len() != 1 || roots[0].id != product.id {
        return Err(AppError::Validation("The current draft belongs to another or unidentified product; review it before staging a catalog node.".into()));
    }
    let root = draft.nodes.iter_mut().find(|node| node.id == product.id && node.node_type == "product").expect("root checked above");
    if !root.details.is_object() { return Err(AppError::Validation("The product draft is malformed; review it before staging catalog context.".into())); }
    if root.details.get("_catalog_binding").is_some() {
        check_bound_product(root, Some(product), true)?;
    } else {
        let seed = seed_draft_with_product(None, product);
        let expected = &seed.nodes[0];
        if root.name != expected.name || root.summary != expected.summary || ["type", "name", "description", "vision", "goals", "tags", "lifecycle", "health", "ownerLabel", "owner_label", "investmentStatus", "investment_status", "roadmap", "evidence"].iter().any(|key| root.details.get(*key) != expected.details.get(*key)) {
            return Err(AppError::Validation("The product draft has unbound changes; review or clear it before staging catalog context.".into()));
        }
        root.summary = Some(product.description.clone());
        root.details["_catalog_binding"] = json!({"kind":"product", "snapshot":product});
    }
    let missing = || AppError::Validation("Catalog node was not found in the selected product.".into());
    if node_type == "product" && node_id != product.id { return Err(missing()); }
    let mut work_path: Vec<WorkItem> = Vec::new();
    if node_type == "work_item" {
        let mut next = Some(node_id.to_string());
        let mut seen = HashSet::new();
        while let Some(id) = next {
            if work_path.len() >= 1000 || !seen.insert(id.clone()) { return Err(AppError::Validation("Work item ancestry is cyclic or exceeds the supported depth.".into())); }
            let work = sqlx::query_as::<_, WorkItem>("SELECT * FROM work_items WHERE id=?")
                .bind(&id).fetch_optional(&mut *conn).await?.ok_or_else(missing)?;
            if work.product_id.as_deref() != Some(product.id.as_str()) { return Err(missing()); }
            if let Some(selected) = work_path.first() {
                if selected.product_area_id != work.product_area_id || selected.capability_id != work.capability_id { return Err(AppError::Validation("Work item ancestry has inconsistent catalog scope.".into())); }
            }
            next = work.parent_work_item_id.clone();
            work_path.push(work);
        }
        work_path.reverse();
    }
    let hierarchy_id = if node_type == "product" { None } else if node_type == "work_item" { work_path.last().and_then(|item| item.capability_id.clone().or_else(|| item.product_area_id.clone())) } else { Some(node_id.to_string()) };
    let hierarchy_is_area = node_type == "product_area" || (node_type == "work_item" && work_path.last().is_some_and(|item| item.capability_id.is_none()));
    let mut parent_id = product.id.clone();
    if let Some(hierarchy_id) = hierarchy_id {
        let (area, path) = if hierarchy_is_area {
            let area = sqlx::query_as::<_, ProductArea>("SELECT * FROM product_areas WHERE id=? AND product_id=?")
                .bind(&hierarchy_id).bind(&product.id).fetch_optional(&mut *conn).await?.ok_or_else(missing)?;
            (area, Vec::new())
        } else {
            let mut path = Vec::new(); let mut seen = HashSet::new(); let mut next = Some(hierarchy_id);
            while let Some(id) = next {
                if path.len() >= 1000 || !seen.insert(id.clone()) { return Err(AppError::Validation("Catalog ancestry is cyclic or exceeds the supported depth.".into())); }
                let capability = sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE id=?")
                    .bind(&id).fetch_optional(&mut *conn).await?.ok_or_else(missing)?;
                if path.is_empty() && node_type == "feature" && capability.node_kind.to_string() != "feature" { return Err(missing()); }
                next = capability.parent_capability_id.clone(); path.push(capability);
            }
            let area_id = &path.first().expect("node id is nonempty").product_area_id;
            if path.iter().any(|capability| &capability.product_area_id != area_id) { return Err(missing()); }
            if let Some(work) = work_path.last() { if work.product_area_id.as_ref() != Some(area_id) { return Err(missing()); } }
            let area = sqlx::query_as::<_, ProductArea>("SELECT * FROM product_areas WHERE id=? AND product_id=?")
                .bind(area_id).bind(&product.id).fetch_optional(&mut *conn).await?.ok_or_else(missing)?;
            path.reverse(); (area, path)
        };
        stage_node(&mut draft, PlannerDraftNode {
            id:area.id.clone(), parent_id:Some(product.id.clone()), node_type:"product_area".into(), name:area.name.clone(), summary:Some(area.description.clone()),
            details:json!({"type":"update_product_area", "purpose":area.purpose, "nodeKind":area.node_kind, "explanation":area.explanation, "examples":area.examples, "implementationNotes":area.implementation_notes, "testGuidance":area.test_guidance, "_catalog_binding":{"kind":"product_area","snapshot":area}}),
        })?;
        parent_id = area.id.clone();
        for capability in path {
            stage_node(&mut draft, PlannerDraftNode {
                id:capability.id.clone(), parent_id:Some(parent_id.clone()), node_type:"capability".into(), name:capability.name.clone(), summary:Some(capability.description.clone()),
                details:json!({"type":"update_capability", "nodeKind":capability.node_kind, "acceptanceCriteria":capability.acceptance_criteria, "explanation":capability.explanation, "examples":capability.examples, "priority":capability.priority, "risk":capability.risk, "technicalNotes":capability.technical_notes, "implementationNotes":capability.implementation_notes, "testGuidance":capability.test_guidance, "_catalog_binding":{"kind":"capability","snapshot":capability}}),
            })?;
            parent_id = capability.id.clone();
        }
    }
    for work in work_path {
        stage_node(&mut draft, PlannerDraftNode {
            id:work.id.clone(), parent_id:Some(parent_id.clone()), node_type:"work_item".into(), name:work.title.clone(), summary:Some(work.description.clone()),
            details:json!({"type":"update_work_item", "problemStatement":work.problem_statement, "acceptanceCriteria":work.acceptance_criteria, "constraints":work.constraints, "status":work.status, "workItemType":work.work_item_type, "priority":work.priority, "complexity":work.complexity, "_catalog_binding":{"kind":"work_item","snapshot":work}}),
        })?;
        parent_id = work.id.clone();
    }
    Ok(draft)
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

pub(crate) async fn apply_bound_work_item(
    conn: &mut SqliteConnection, node: &PlannerDraftNode, product_id: &str,
    area_id: Option<&str>, capability_id: Option<&str>, parent_id: Option<&str>,
) -> Result<Option<WorkItem>, AppError> {
    let Some(snapshot) = binding::<WorkItem>(node, "work_item")? else { return Ok(None) };
    validate_name(node)?;
    if snapshot.id != node.id || snapshot.product_id.as_deref() != Some(product_id) ||
        snapshot.product_area_id.as_deref() != area_id || snapshot.capability_id.as_deref() != capability_id ||
        snapshot.parent_work_item_id.as_deref() != parent_id {
        return Err(AppError::Validation("Staged work item does not belong to the selected hierarchy or parent.".into()));
    }
    let current = sqlx::query_as::<_, WorkItem>("SELECT * FROM work_items WHERE id=?")
        .bind(&snapshot.id).fetch_optional(&mut *conn).await?
        .ok_or_else(|| AppError::Validation("The staged work item no longer exists.".into()))?;
    if serde_json::to_value(&current)? != serde_json::to_value(&snapshot)? {
        return Err(AppError::Validation("The staged work item changed; refresh and review it before applying.".into()));
    }
    let title = node.name.trim();
    let description = node.summary.as_deref().unwrap_or(&current.description);
    let problem = field(node, "problemStatement", "problem_statement", &current.problem_statement);
    let criteria = field(node, "acceptanceCriteria", "acceptance_criteria", &current.acceptance_criteria);
    let constraints = field(node, "constraints", "constraints", &current.constraints);
    let status = field(node, "status", "status", &current.status.to_string());
    serde_json::from_value::<WorkItemStatus>(serde_json::Value::String(status.clone()))
        .map_err(|_| AppError::Validation("Invalid staged work item status.".into()))?;
    let changed = title != current.title || description != current.description || problem != current.problem_statement ||
        criteria != current.acceptance_criteria || constraints != current.constraints || status != current.status.to_string();
    if !changed { return Ok(Some(current)); }
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_runs WHERE work_item_id=? AND status IN ('running','paused') AND current_stage NOT IN ('done','failed','cancelled')) OR EXISTS(SELECT 1 FROM external_cli_runs WHERE work_item_id=? AND status='running')")
        .bind(&snapshot.id).bind(&snapshot.id).fetch_one(&mut *conn).await?;
    if active { return Err(AppError::Validation("The staged work item has an active workflow or CLI run; finish it before applying changes.".into())); }
    let updated = sqlx::query_as::<_, WorkItem>("UPDATE work_items SET title=?, description=?, problem_statement=?, acceptance_criteria=?, constraints=?, status=?, updated_at=datetime('now') WHERE id=? RETURNING *")
        .bind(title).bind(description).bind(problem).bind(criteria).bind(constraints).bind(status).bind(&snapshot.id)
        .fetch_one(&mut *conn).await?;
    Ok(Some(updated))
}

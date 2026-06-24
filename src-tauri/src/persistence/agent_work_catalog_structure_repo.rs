use crate::domain::agent_work::AgentWorkItem;
use crate::error::AppError;
use crate::persistence::agent_work_catalog_materialization::{
    materialized_area_label, materialized_capability_label, normalize_catalog_key,
    stable_materialized_id, MaterializedArea, MaterializedCapability, MATERIALIZE_BATCH_SIZE,
};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub(crate) async fn ensure_materialized_product_areas(
    pool: &SqlitePool,
    run_id: &str,
    product_id: &str,
    items: &[AgentWorkItem],
) -> Result<HashMap<String, MaterializedArea>, AppError> {
    let product_area_rows = sqlx::query(
        "SELECT id, name, sort_order FROM product_areas WHERE product_id=? ORDER BY sort_order",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    let mut product_areas_by_key: HashMap<String, String> = HashMap::new();
    let mut max_product_area_sort_order = -1_i64;
    for row in product_area_rows {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let sort_order: i64 = row.get("sort_order");
        max_product_area_sort_order = max_product_area_sort_order.max(sort_order);
        product_areas_by_key.insert(normalize_catalog_key(&id), id.clone());
        product_areas_by_key.insert(normalize_catalog_key(&name), id);
    }

    let mut area_labels = items
        .iter()
        .map(materialized_area_label)
        .collect::<Vec<_>>();
    area_labels.sort_by_key(|label| normalize_catalog_key(label));
    area_labels.dedup_by(|a, b| normalize_catalog_key(a) == normalize_catalog_key(b));

    let mut areas: HashMap<String, MaterializedArea> = HashMap::new();
    let mut area_rows_to_insert = Vec::new();
    let mut next_product_area_sort_order = max_product_area_sort_order + 1;
    for area_label in area_labels {
        let key = normalize_catalog_key(&area_label);
        if let Some(product_area_id) = product_areas_by_key.get(&key) {
            areas.insert(
                key,
                MaterializedArea {
                    id: product_area_id.clone(),
                    created: false,
                },
            );
            continue;
        }
        let product_area_id = stable_materialized_id("agentwork-area", &[product_id, &area_label]);
        areas.insert(
            key.clone(),
            MaterializedArea {
                id: product_area_id.clone(),
                created: true,
            },
        );
        product_areas_by_key.insert(key, product_area_id.clone());
        area_rows_to_insert.push((product_area_id, area_label, next_product_area_sort_order));
        next_product_area_sort_order += 1;
    }

    for chunk in area_rows_to_insert.chunks(MATERIALIZE_BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        for (product_area_id, area_label, sort_order) in chunk {
            sqlx::query(
                "INSERT INTO product_areas (
                    id, product_id, node_kind, name, description, purpose, explanation,
                    examples, implementation_notes, test_guidance, sort_order
                 )
                 VALUES (?, ?, 'product_area', ?, ?, ?, '', '', '', '', ?)
                 ON CONFLICT(id) DO UPDATE SET
                    product_id=excluded.product_id,
                    node_kind='product_area',
                    name=excluded.name,
                    description=excluded.description,
                    purpose=excluded.purpose,
                    sort_order=excluded.sort_order,
                    updated_at=datetime('now')",
            )
            .bind(product_area_id)
            .bind(product_id)
            .bind(area_label)
            .bind(format!(
                "Materialized product area from agent-work run {run_id}."
            ))
            .bind(format!("Agent-work product area {area_label}."))
            .bind(*sort_order)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    Ok(areas)
}

pub(crate) async fn ensure_materialized_capabilities(
    pool: &SqlitePool,
    run_id: &str,
    product_id: &str,
    items: &[AgentWorkItem],
    areas: &HashMap<String, MaterializedArea>,
) -> Result<HashMap<(String, String), MaterializedCapability>, AppError> {
    let existing_capability_rows = sqlx::query(
        "SELECT c.id, c.product_area_id, c.name, c.sort_order
         FROM capabilities c
         JOIN product_areas pa ON pa.id=c.product_area_id
         WHERE pa.product_id=? AND c.parent_capability_id IS NULL",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    let mut capabilities_by_key: HashMap<(String, String), (String, i64)> = HashMap::new();
    let mut max_capability_sort_order_by_product_area: HashMap<String, i64> = HashMap::new();
    for row in existing_capability_rows {
        let id: String = row.get("id");
        let product_area_id: String = row.get("product_area_id");
        let name: String = row.get("name");
        let sort_order: i64 = row.get("sort_order");
        max_capability_sort_order_by_product_area
            .entry(product_area_id.clone())
            .and_modify(|current| *current = (*current).max(sort_order))
            .or_insert(sort_order);
        capabilities_by_key.insert(
            (product_area_id, normalize_catalog_key(&name)),
            (id, sort_order),
        );
    }

    let mut capability_specs = items
        .iter()
        .map(|item| {
            let area_key = normalize_catalog_key(&materialized_area_label(item));
            let product_area_id = areas
                .get(&area_key)
                .expect("area must be materialized")
                .id
                .clone();
            (product_area_id, materialized_capability_label(item))
        })
        .collect::<Vec<_>>();
    capability_specs.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| normalize_catalog_key(&a.1).cmp(&normalize_catalog_key(&b.1)))
    });
    capability_specs
        .dedup_by(|a, b| a.0 == b.0 && normalize_catalog_key(&a.1) == normalize_catalog_key(&b.1));

    let mut capabilities: HashMap<(String, String), MaterializedCapability> = HashMap::new();
    let mut capability_rows_to_upsert = Vec::new();
    for (product_area_id, capability_label) in capability_specs {
        let key = (
            product_area_id.clone(),
            normalize_catalog_key(&capability_label),
        );
        if let Some((capability_id, sort_order)) = capabilities_by_key.get(&key) {
            capabilities.insert(
                key,
                MaterializedCapability {
                    id: capability_id.clone(),
                    created: false,
                },
            );
            capability_rows_to_upsert.push((
                capability_id.clone(),
                product_area_id,
                capability_label,
                *sort_order,
                false,
            ));
            continue;
        }
        let sort_order = max_capability_sort_order_by_product_area
            .entry(product_area_id.clone())
            .and_modify(|current| *current += 1)
            .or_insert(0);
        let capability_id = stable_materialized_id(
            "agentwork-cap",
            &[run_id, &product_area_id, &capability_label],
        );
        capabilities.insert(
            key,
            MaterializedCapability {
                id: capability_id.clone(),
                created: true,
            },
        );
        capability_rows_to_upsert.push((
            capability_id,
            product_area_id,
            capability_label,
            *sort_order,
            true,
        ));
    }

    for chunk in capability_rows_to_upsert.chunks(MATERIALIZE_BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        for (capability_id, product_area_id, capability_label, sort_order, created) in chunk {
            if !*created && !capability_id.starts_with("agentwork-cap-") {
                continue;
            }
            sqlx::query(
                "INSERT INTO capabilities (
                    id, product_area_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, status, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, NULL, 0, 'capability', ?, ?, ?, ?, '', '', 'medium', 'medium', 'draft', ?, '', '')
                 ON CONFLICT(id) DO UPDATE SET
                    product_area_id=excluded.product_area_id,
                    parent_capability_id=NULL,
                    level=0,
                    node_kind='capability',
                    sort_order=excluded.sort_order,
                    name=excluded.name,
                    description=excluded.description,
                    acceptance_criteria=excluded.acceptance_criteria,
                    technical_notes=excluded.technical_notes,
                    updated_at=datetime('now')",
            )
            .bind(capability_id)
            .bind(product_area_id)
            .bind(*sort_order)
            .bind(capability_label)
            .bind(format!(
                "Materialized capability group from agent-work run {run_id}."
            ))
            .bind(format!(
                "Agent-work rows grouped under {capability_label} are visible in the product catalog."
            ))
            .bind(format!(
                "Generated from agent-work run {run_id}; rerunning materialization is idempotent."
            ))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    Ok(capabilities)
}

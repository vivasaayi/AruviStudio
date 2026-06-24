use crate::domain::agent_work::{
    AgentWorkCatalogLinkResult, AgentWorkItem, AgentWorkMaterializationResult,
};
use crate::error::AppError;
use crate::persistence::agent_work_catalog_materialization::{
    map_agent_priority, map_capability_status, map_work_item_status, materialized_area_label,
    materialized_capability_label, materialized_description, materialized_title,
    normalize_catalog_key, stable_materialized_id, MaterializedArea, MaterializedCapability,
    MaterializedFeature, MATERIALIZE_BATCH_SIZE,
};
use crate::persistence::agent_work_repo::{append_event, get_run, AppendAgentWorkEventInput};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub async fn materialize_catalog(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    create_work_items: bool,
) -> Result<AgentWorkMaterializationResult, AppError> {
    let run = get_run(pool, run_id).await?;
    let resolved_product_id = product_id
        .map(str::to_string)
        .or(run.product_id.clone())
        .ok_or_else(|| {
            AppError::Validation(
                "agent work run is not attached to a product; provide productId".to_string(),
            )
        })?;
    if let (Some(run_product_id), Some(request_product_id)) =
        (run.product_id.as_deref(), product_id)
    {
        if run_product_id != request_product_id {
            return Err(AppError::Validation(format!(
                "Run {run_id} is attached to product {run_product_id}, not {request_product_id}."
            )));
        }
    }

    let product_exists: Option<String> = sqlx::query_scalar("SELECT id FROM products WHERE id=?")
        .bind(&resolved_product_id)
        .fetch_optional(pool)
        .await?;
    if product_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Product {resolved_product_id} not found"
        )));
    }
    if run.product_id.is_none() {
        sqlx::query(
            "UPDATE agent_work_runs SET product_id=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&resolved_product_id)
        .bind(run_id)
        .execute(pool)
        .await?;
    }

    let items = sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,product_area,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items
         WHERE run_id=?
         ORDER BY product_area, service_or_domain, release_phase, feature_id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    if items.is_empty() {
        return Ok(AgentWorkMaterializationResult {
            run_id: run_id.to_string(),
            product_id: resolved_product_id,
            total_items: 0,
            product_areas_created: 0,
            product_areas_reused: 0,
            capabilities_created: 0,
            capabilities_reused: 0,
            features_upserted: 0,
            work_items_upserted: 0,
            linked_work_items: 0,
        });
    }

    let product_area_rows = sqlx::query(
        "SELECT id, name, sort_order FROM product_areas WHERE product_id=? ORDER BY sort_order",
    )
    .bind(&resolved_product_id)
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
        let product_area_id =
            stable_materialized_id("agentwork-area", &[&resolved_product_id, &area_label]);
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
            .bind(&resolved_product_id)
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

    let existing_capability_rows = sqlx::query(
        "SELECT c.id, c.product_area_id, c.name, c.sort_order
         FROM capabilities c
         JOIN product_areas pa ON pa.id=c.product_area_id
         WHERE pa.product_id=? AND c.parent_capability_id IS NULL",
    )
    .bind(&resolved_product_id)
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

    let mut feature_sort_by_capability: HashMap<String, i64> = HashMap::new();
    let mut materialized_features = Vec::with_capacity(items.len());
    for item in items {
        let area_key = normalize_catalog_key(&materialized_area_label(&item));
        let product_area_id = areas
            .get(&area_key)
            .expect("area must be materialized")
            .id
            .clone();
        let capability_key = (
            product_area_id.clone(),
            normalize_catalog_key(&materialized_capability_label(&item)),
        );
        let capability_id = capabilities
            .get(&capability_key)
            .expect("capability must be materialized")
            .id
            .clone();
        let sort_order = feature_sort_by_capability
            .entry(capability_id.clone())
            .and_modify(|current| *current += 1)
            .or_insert(0);
        let catalog_feature_id =
            stable_materialized_id("agentwork-feature", &[run_id, &item.feature_id]);
        let work_item_id = item.work_item_id.clone().unwrap_or_else(|| {
            stable_materialized_id("agentwork-work", &[run_id, &item.feature_id])
        });
        materialized_features.push(MaterializedFeature {
            item,
            product_area_id,
            capability_id,
            feature_id: catalog_feature_id,
            work_item_id,
            sort_order: *sort_order,
        });
    }

    for chunk in materialized_features.chunks(MATERIALIZE_BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        for feature in chunk {
            let title = materialized_title(&feature.item);
            let description = materialized_description(&feature.item);
            let priority = map_agent_priority(feature.item.priority.as_deref());
            let status = map_capability_status(&feature.item.status);
            sqlx::query(
                "INSERT INTO capabilities (
                    id, product_area_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, status, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, ?, 1, 'feature', ?, ?, ?, ?, '', '', ?, 'medium', ?, ?, '', '')
                 ON CONFLICT(id) DO UPDATE SET
                    product_area_id=excluded.product_area_id,
                    parent_capability_id=excluded.parent_capability_id,
                    level=1,
                    node_kind='feature',
                    sort_order=excluded.sort_order,
                    name=excluded.name,
                    description=excluded.description,
                    acceptance_criteria=excluded.acceptance_criteria,
                    priority=excluded.priority,
                    status=excluded.status,
                    technical_notes=excluded.technical_notes,
                    updated_at=datetime('now')",
            )
            .bind(&feature.feature_id)
            .bind(&feature.product_area_id)
            .bind(&feature.capability_id)
            .bind(feature.sort_order)
            .bind(&title)
            .bind(&description)
            .bind(format!(
                "Agent-work feature {} is represented in the product catalog.",
                feature.item.feature_id
            ))
            .bind(priority)
            .bind(status)
            .bind(format!(
                "Agent-work run: {}; ledger feature id: {}; status: {}.",
                run_id, feature.item.feature_id, feature.item.status
            ))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    let mut work_items_upserted = 0_i64;
    let mut linked_work_items = 0_i64;
    if create_work_items {
        for chunk in materialized_features.chunks(MATERIALIZE_BATCH_SIZE) {
            let mut tx = pool.begin().await?;
            for feature in chunk {
                let title = materialized_title(&feature.item);
                let description = materialized_description(&feature.item);
                let priority = map_agent_priority(feature.item.priority.as_deref());
                let work_item_status = map_work_item_status(&feature.item.status);
                sqlx::query(
                    "INSERT INTO work_items (
                        id, product_id, product_area_id, capability_id, source_node_id, source_node_type,
                        parent_work_item_id, title, problem_statement, description,
                        acceptance_criteria, constraints, work_item_type, priority, complexity,
                        status, sort_order
                     )
                     VALUES (?, ?, ?, ?, ?, 'capability', NULL, ?, ?, ?, ?, ?, 'story', ?, 'medium', ?, 0)
                     ON CONFLICT(id) DO UPDATE SET
                        product_id=excluded.product_id,
                        product_area_id=excluded.product_area_id,
                        capability_id=excluded.capability_id,
                        source_node_id=excluded.source_node_id,
                        source_node_type='capability',
                        parent_work_item_id=NULL,
                        title=excluded.title,
                        problem_statement=excluded.problem_statement,
                        description=excluded.description,
                        acceptance_criteria=excluded.acceptance_criteria,
                        constraints=excluded.constraints,
                        work_item_type='story',
                        priority=excluded.priority,
                        complexity='medium',
                        status=excluded.status,
                        sort_order=excluded.sort_order,
                        updated_at=datetime('now')",
                )
                .bind(&feature.work_item_id)
                .bind(&resolved_product_id)
                .bind(&feature.product_area_id)
                .bind(&feature.feature_id)
                .bind(&feature.feature_id)
                .bind(&title)
                .bind(format!(
                    "Materialized from agent-work run {run_id}, ledger feature {}.",
                    feature.item.feature_id
                ))
                .bind(&description)
                .bind(format!(
                    "Visible work item remains linked to agent-work feature {}.",
                    feature.item.feature_id
                ))
                .bind(format!(
                    "Preserve agent-work ledger linkage. Original agent-work status: {}.",
                    feature.item.status
                ))
                .bind(priority)
                .bind(work_item_status)
                .execute(&mut *tx)
                .await?;

                sqlx::query(
                    "UPDATE agent_work_items
                     SET work_item_id=?, updated_at=datetime('now')
                     WHERE run_id=? AND feature_id=?",
                )
                .bind(&feature.work_item_id)
                .bind(run_id)
                .bind(&feature.item.feature_id)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            work_items_upserted += i64::try_from(chunk.len()).unwrap_or(i64::MAX);
            linked_work_items += i64::try_from(chunk.len()).unwrap_or(i64::MAX);
        }
    }

    let product_areas_created = areas.values().filter(|area| area.created).count() as i64;
    let product_areas_reused = areas.values().filter(|area| !area.created).count() as i64;
    let capabilities_created = capabilities
        .values()
        .filter(|capability| capability.created)
        .count() as i64;
    let capabilities_reused = capabilities
        .values()
        .filter(|capability| !capability.created)
        .count() as i64;
    let features_upserted = i64::try_from(materialized_features.len()).unwrap_or(i64::MAX);
    let result = AgentWorkMaterializationResult {
        run_id: run_id.to_string(),
        product_id: resolved_product_id,
        total_items: features_upserted,
        product_areas_created,
        product_areas_reused,
        capabilities_created,
        capabilities_reused,
        features_upserted,
        work_items_upserted,
        linked_work_items,
    };

    append_event(
        pool,
        AppendAgentWorkEventInput {
            run_id,
            event_type: "catalog_materialized",
            batch_id: None,
            feature_id: None,
            work_item_id: None,
            agent: None,
            command: None,
            status: Some("completed"),
            details: Some(
                "Agent-work rows materialized into catalog features and visible work items.",
            ),
            metadata: Some(serde_json::to_value(&result)?),
        },
    )
    .await?;

    Ok(result)
}

pub async fn link_catalog_work_items(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    sync_statuses: bool,
) -> Result<AgentWorkCatalogLinkResult, AppError> {
    crate::persistence::agent_work_catalog_link_repo::link_catalog_work_items(
        pool,
        run_id,
        product_id,
        sync_statuses,
    )
    .await
}

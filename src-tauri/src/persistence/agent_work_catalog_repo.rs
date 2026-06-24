use crate::domain::agent_work::{
    AgentWorkCatalogLinkResult, AgentWorkItem, AgentWorkMaterializationResult,
};
use crate::error::AppError;
use crate::persistence::agent_work_catalog_materialization::{
    map_agent_priority, map_capability_status, map_work_item_status, materialized_area_label,
    materialized_capability_label, materialized_description, materialized_title,
    normalize_catalog_key, stable_materialized_id, MaterializedFeature, MATERIALIZE_BATCH_SIZE,
};
use crate::persistence::agent_work_catalog_structure_repo::{
    ensure_materialized_capabilities, ensure_materialized_product_areas,
};
use crate::persistence::agent_work_repo::{append_event, get_run, AppendAgentWorkEventInput};
use sqlx::SqlitePool;
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

    let areas =
        ensure_materialized_product_areas(pool, run_id, &resolved_product_id, &items).await?;
    let capabilities =
        ensure_materialized_capabilities(pool, run_id, &resolved_product_id, &items, &areas)
            .await?;

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

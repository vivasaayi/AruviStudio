use crate::domain::agent_work::AgentWorkCatalogLinkResult;
use crate::error::AppError;
use crate::persistence::agent_work_catalog_repo::{map_capability_status, map_work_item_status};
use crate::persistence::agent_work_repo::{append_event, get_run, AppendAgentWorkEventInput};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct CatalogWorkCandidate {
    work_item_id: String,
    catalog_feature_id: Option<String>,
}

fn add_string_candidate(map: &mut HashMap<String, Vec<String>>, key: &str, value: &str) {
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        return;
    }
    let values = map.entry(key.to_string()).or_default();
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn add_catalog_work_candidate(
    map: &mut HashMap<String, Vec<CatalogWorkCandidate>>,
    key: &str,
    work_item_id: &str,
    catalog_feature_id: Option<&str>,
) {
    let key = key.trim();
    let work_item_id = work_item_id.trim();
    if key.is_empty() || work_item_id.is_empty() {
        return;
    }
    let catalog_feature_id = catalog_feature_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let values = map.entry(key.to_string()).or_default();
    if !values
        .iter()
        .any(|existing| existing.work_item_id == work_item_id)
    {
        values.push(CatalogWorkCandidate {
            work_item_id: work_item_id.to_string(),
            catalog_feature_id,
        });
    }
}

fn extract_roadmap_feature_id(technical_notes: &str) -> Option<String> {
    let marker = "Roadmap feature id:";
    let (_, after_marker) = technical_notes.split_once(marker)?;
    let raw = after_marker
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.');
    (!raw.is_empty()).then(|| raw.to_string())
}

pub async fn link_catalog_work_items(
    pool: &SqlitePool,
    run_id: &str,
    product_id: Option<&str>,
    sync_statuses: bool,
) -> Result<AgentWorkCatalogLinkResult, AppError> {
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

    let total_items: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_work_items WHERE run_id=?")
            .bind(run_id)
            .fetch_one(pool)
            .await?;
    let already_linked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_items WHERE run_id=? AND work_item_id IS NOT NULL",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    let unlinked_rows = sqlx::query(
        "SELECT feature_id, status
         FROM agent_work_items
         WHERE run_id=? AND work_item_id IS NULL
         ORDER BY feature_id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    let work_rows = sqlx::query(
        "SELECT id, capability_id, source_node_id
         FROM work_items
         WHERE product_id=? AND parent_work_item_id IS NULL",
    )
    .bind(&resolved_product_id)
    .fetch_all(pool)
    .await?;
    let mut work_by_key: HashMap<String, Vec<CatalogWorkCandidate>> = HashMap::new();
    for row in work_rows {
        let work_item_id: String = row.get("id");
        let capability_id = row.get::<Option<String>, _>("capability_id");
        add_catalog_work_candidate(
            &mut work_by_key,
            &work_item_id,
            &work_item_id,
            capability_id.as_deref(),
        );
        if let Some(capability_id) = capability_id.as_deref() {
            add_catalog_work_candidate(
                &mut work_by_key,
                capability_id,
                &work_item_id,
                Some(capability_id),
            );
        }
        if let Some(source_node_id) = row.get::<Option<String>, _>("source_node_id") {
            add_catalog_work_candidate(
                &mut work_by_key,
                &source_node_id,
                &work_item_id,
                capability_id.as_deref(),
            );
        }
    }

    let feature_metadata_rows = sqlx::query(
        "SELECT c.id, c.technical_notes
         FROM capabilities c
         JOIN product_areas pa ON pa.id=c.product_area_id
         WHERE pa.product_id=? AND c.node_kind='feature' AND c.technical_notes LIKE '%Roadmap feature id:%'",
    )
    .bind(&resolved_product_id)
    .fetch_all(pool)
    .await?;
    let mut catalog_feature_by_roadmap_id: HashMap<String, Vec<String>> = HashMap::new();
    for row in feature_metadata_rows {
        let catalog_feature_id: String = row.get("id");
        let technical_notes: String = row.get("technical_notes");
        if let Some(roadmap_feature_id) = extract_roadmap_feature_id(&technical_notes) {
            add_string_candidate(
                &mut catalog_feature_by_roadmap_id,
                &roadmap_feature_id,
                &catalog_feature_id,
            );
        }
    }

    let mut rows_to_link = Vec::new();
    let mut missing_work_items = 0_i64;
    let mut ambiguous_work_items = 0_i64;
    for row in unlinked_rows {
        let feature_id: String = row.get("feature_id");
        let status: String = row.get("status");
        let mut candidates: HashMap<String, String> = HashMap::new();
        if let Some(work_item_candidates) = work_by_key.get(&feature_id) {
            for candidate in work_item_candidates {
                candidates.insert(
                    candidate.work_item_id.clone(),
                    candidate
                        .catalog_feature_id
                        .clone()
                        .unwrap_or_else(|| feature_id.clone()),
                );
            }
        }
        if let Some(catalog_feature_ids) = catalog_feature_by_roadmap_id.get(&feature_id) {
            for catalog_feature_id in catalog_feature_ids {
                if let Some(work_item_candidates) = work_by_key.get(catalog_feature_id) {
                    for candidate in work_item_candidates {
                        candidates
                            .insert(candidate.work_item_id.clone(), catalog_feature_id.clone());
                    }
                }
            }
        }

        match candidates.len() {
            0 => missing_work_items += 1,
            1 => {
                let (work_item_id, catalog_feature_id) =
                    candidates.into_iter().next().expect("candidate exists");
                rows_to_link.push((feature_id, status, work_item_id, catalog_feature_id));
            }
            _ => ambiguous_work_items += 1,
        }
    }

    let mut linked_work_items = 0_i64;
    let mut status_synced = 0_i64;
    if !rows_to_link.is_empty() {
        let mut tx = pool.begin().await?;
        for (feature_id, status, work_item_id, catalog_feature_id) in &rows_to_link {
            let result = sqlx::query(
                "UPDATE agent_work_items
                 SET work_item_id=?, updated_at=datetime('now')
                 WHERE run_id=? AND feature_id=? AND work_item_id IS NULL",
            )
            .bind(work_item_id)
            .bind(run_id)
            .bind(feature_id)
            .execute(&mut *tx)
            .await?;
            linked_work_items += i64::try_from(result.rows_affected()).unwrap_or(i64::MAX);

            if sync_statuses {
                let work_item_status = map_work_item_status(status);
                let capability_status = map_capability_status(status);
                let work_result = sqlx::query(
                    "UPDATE work_items
                     SET status=?, updated_at=datetime('now')
                     WHERE id=?",
                )
                .bind(work_item_status)
                .bind(work_item_id)
                .execute(&mut *tx)
                .await?;
                let capability_result = sqlx::query(
                    "UPDATE capabilities
                     SET status=?, updated_at=datetime('now')
                     WHERE id=?",
                )
                .bind(capability_status)
                .bind(catalog_feature_id)
                .execute(&mut *tx)
                .await?;
                if work_result.rows_affected() > 0 || capability_result.rows_affected() > 0 {
                    status_synced += 1;
                }
            }
        }
        tx.commit().await?;
    }

    let result = AgentWorkCatalogLinkResult {
        run_id: run_id.to_string(),
        product_id: resolved_product_id,
        total_items,
        already_linked,
        linked_work_items,
        missing_work_items,
        ambiguous_work_items,
        status_synced,
    };

    append_event(
        pool,
        AppendAgentWorkEventInput {
            run_id,
            event_type: "catalog_work_items_linked",
            batch_id: None,
            feature_id: None,
            work_item_id: None,
            agent: None,
            command: None,
            status: Some("completed"),
            details: Some("Linked existing catalog work items to agent-work rows."),
            metadata: Some(serde_json::to_value(&result)?),
        },
    )
    .await?;

    Ok(result)
}

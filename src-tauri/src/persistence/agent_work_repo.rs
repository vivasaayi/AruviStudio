use crate::domain::agent_work::{
    AgentWorkAgentActivity, AgentWorkBatch, AgentWorkCatalogLinkResult, AgentWorkClaim,
    AgentWorkConflictZoneSummary, AgentWorkDependency, AgentWorkEvent, AgentWorkEvidence,
    AgentWorkItem, AgentWorkLock, AgentWorkMaterializationResult, AgentWorkRun, AgentWorkRunHealth,
    AgentWorkRunSummary, AgentWorkStatusCount,
};
use crate::error::AppError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;

const DEFAULT_LEASE_SECONDS: i64 = 900;
const MAX_CANDIDATES: i64 = 250;
const MATERIALIZE_BATCH_SIZE: usize = 1_000;

#[derive(Debug, Clone)]
struct MaterializedArea {
    id: String,
    created: bool,
}

#[derive(Debug, Clone)]
struct MaterializedCapability {
    id: String,
    created: bool,
}

#[derive(Debug, Clone)]
struct MaterializedFeature {
    item: AgentWorkItem,
    module_id: String,
    capability_id: String,
    feature_id: String,
    work_item_id: String,
    sort_order: i64,
}

#[derive(Debug, Clone)]
struct CatalogWorkCandidate {
    work_item_id: String,
    catalog_feature_id: Option<String>,
}

fn json_object_string(value: Option<Value>) -> Result<String, AppError> {
    match value {
        Some(value @ Value::Object(_)) => serde_json::to_string(&value).map_err(AppError::from),
        Some(Value::Null) | None => Ok("{}".to_string()),
        Some(_) => Err(AppError::Validation(
            "metadata must be a JSON object".to_string(),
        )),
    }
}

fn json_array_string(value: Option<Value>) -> Result<String, AppError> {
    match value {
        Some(value @ Value::Array(_)) => serde_json::to_string(&value).map_err(AppError::from),
        Some(Value::Null) | None => Ok("[]".to_string()),
        Some(Value::String(raw)) => {
            let parsed: Value = serde_json::from_str(&raw)?;
            if !parsed.is_array() {
                return Err(AppError::Validation(
                    "conflict zones must be a JSON array".to_string(),
                ));
            }
            serde_json::to_string(&parsed).map_err(AppError::from)
        }
        Some(_) => Err(AppError::Validation(
            "conflict zones must be a JSON array".to_string(),
        )),
    }
}

fn normalize_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "pending" => Ok("pending"),
        "claimed" => Ok("claimed"),
        "in_progress" | "in-progress" => Ok("in_progress"),
        "implemented" => Ok("implemented"),
        "tests_passed" | "tests-passed" => Ok("tests_passed"),
        "committed" => Ok("committed"),
        "blocked" => Ok("blocked"),
        "skipped" => Ok("skipped"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work status '{other}'."
        ))),
    }
}

fn normalize_run_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" => Ok("active"),
        "paused" => Ok("paused"),
        "completed" => Ok("completed"),
        "blocked" => Ok("blocked"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work run status '{other}'."
        ))),
    }
}

fn normalize_batch_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "claimed" => Ok("claimed"),
        "in_progress" | "in-progress" => Ok("in_progress"),
        "implemented" => Ok("implemented"),
        "tests_passed" | "tests-passed" => Ok("tests_passed"),
        "committed" => Ok("committed"),
        "blocked" => Ok("blocked"),
        "skipped" => Ok("skipped"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work batch status '{other}'."
        ))),
    }
}

fn parse_conflict_zones(item: &AgentWorkItem) -> Result<Vec<String>, AppError> {
    let parsed = serde_json::from_str::<Vec<String>>(&item.conflict_zones_json)?;
    let mut zones = parsed
        .into_iter()
        .map(|zone| zone.trim().to_string())
        .filter(|zone| !zone.is_empty())
        .collect::<Vec<_>>();
    if zones.is_empty() {
        if !item.module.trim().is_empty() {
            zones.push(format!("module:{}", item.module.trim()));
        } else if let Some(service) = item.service_or_domain.as_deref() {
            if !service.trim().is_empty() {
                zones.push(format!("service:{}", service.trim()));
            }
        }
    }
    if zones.is_empty() {
        zones.push(format!("feature:{}", item.feature_id));
    }
    zones.sort();
    zones.dedup();
    Ok(zones)
}

fn lease_modifier(lease_seconds: i64) -> String {
    format!("+{} seconds", lease_seconds.clamp(30, 86_400))
}

fn stable_materialized_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(part.as_bytes());
        hasher.update([0xff]);
    }
    let digest = hasher.finalize();
    let mut suffix = String::new();
    for byte in digest.iter().take(12) {
        let _ = write!(&mut suffix, "{byte:02x}");
    }
    format!("{prefix}-{suffix}")
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

fn normalize_catalog_key(value: &str) -> String {
    value
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn materialized_area_label(item: &AgentWorkItem) -> String {
    let module = item.module.trim();
    if module.is_empty() {
        "Imported Agent Work".to_string()
    } else {
        module.to_string()
    }
}

fn materialized_capability_label(item: &AgentWorkItem) -> String {
    item.service_or_domain
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            item.release_phase
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("Imported Agent Work")
        .to_string()
}

fn materialized_title(item: &AgentWorkItem) -> String {
    let title = item.title.trim();
    if title.is_empty() {
        item.feature_id.clone()
    } else {
        title.to_string()
    }
}

fn materialized_description(item: &AgentWorkItem) -> String {
    let description = item.description.trim();
    if description.is_empty() {
        format!(
            "Materialized from agent-work feature {} in run {}.",
            item.feature_id, item.run_id
        )
    } else {
        description.to_string()
    }
}

fn map_agent_priority(priority: Option<&str>) -> &'static str {
    match priority
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "p0" | "critical" => "critical",
        "p1" | "high" => "high",
        "p2" | "medium" => "medium",
        "p3" | "low" => "low",
        _ => "medium",
    }
}

fn map_capability_status(status: &str) -> &'static str {
    match status {
        "committed" => "done",
        "claimed" | "in_progress" | "implemented" | "tests_passed" => "in_progress",
        "skipped" | "cancelled" => "archived",
        _ => "draft",
    }
}

fn map_work_item_status(status: &str) -> &'static str {
    match status {
        "committed" => "done",
        "claimed" | "in_progress" => "in_progress",
        "implemented" | "tests_passed" => "in_validation",
        "blocked" => "blocked",
        "skipped" | "cancelled" => "cancelled",
        _ => "draft",
    }
}

pub async fn upsert_run(
    pool: &SqlitePool,
    id: &str,
    product_id: Option<&str>,
    repository_id: Option<&str>,
    roadmap_hash: &str,
    status: Option<&str>,
    last_commit_sha: Option<&str>,
    current_batch_id: Option<&str>,
    next_action: Option<&str>,
    metadata: Option<Value>,
) -> Result<AgentWorkRun, AppError> {
    let status = normalize_run_status(status.unwrap_or("active"))?;
    let metadata_json = json_object_string(metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_runs (
            id, product_id, repository_id, roadmap_hash, status, last_commit_sha,
            current_batch_id, next_action, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            product_id=excluded.product_id,
            repository_id=excluded.repository_id,
            roadmap_hash=excluded.roadmap_hash,
            status=excluded.status,
            last_commit_sha=COALESCE(excluded.last_commit_sha, agent_work_runs.last_commit_sha),
            current_batch_id=excluded.current_batch_id,
            next_action=excluded.next_action,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(id)
    .bind(product_id)
    .bind(repository_id)
    .bind(roadmap_hash)
    .bind(status)
    .bind(last_commit_sha)
    .bind(current_batch_id)
    .bind(next_action.unwrap_or(""))
    .bind(metadata_json)
    .execute(pool)
    .await?;
    get_run(pool, id).await
}

pub async fn get_run(pool: &SqlitePool, id: &str) -> Result<AgentWorkRun, AppError> {
    sqlx::query_as::<_, AgentWorkRun>(
        "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
         FROM agent_work_runs WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work run {id} not found")))
}

pub async fn list_runs(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkRun>, AppError> {
    let limit = limit.clamp(1, 500);
    if let Some(status) = status {
        let status = normalize_run_status(status)?;
        sqlx::query_as::<_, AgentWorkRun>(
            "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
             FROM agent_work_runs WHERE status=? ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(status)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, AgentWorkRun>(
            "SELECT id,product_id,repository_id,roadmap_hash,status,last_commit_sha,current_batch_id,next_action,metadata_json,started_at,updated_at
             FROM agent_work_runs ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

pub async fn get_run_summary(
    pool: &SqlitePool,
    run_id: &str,
    event_limit: i64,
) -> Result<AgentWorkRunSummary, AppError> {
    let run = get_run(pool, run_id).await?;
    let status_counts = sqlx::query_as::<_, AgentWorkStatusCount>(
        "SELECT status, COUNT(*) AS count FROM agent_work_items WHERE run_id=? GROUP BY status ORDER BY status",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    let active_locks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_locks WHERE run_id=? AND released_at IS NULL",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let latest_events = list_events(pool, run_id, None, None, event_limit).await?;
    Ok(AgentWorkRunSummary {
        run,
        status_counts,
        active_locks,
        latest_events,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    work_item_id: Option<&str>,
    module: &str,
    service_or_domain: Option<&str>,
    priority: Option<&str>,
    release_phase: Option<&str>,
    title: &str,
    description: &str,
    status: Option<&str>,
    batch_id: Option<&str>,
    agent: Option<&str>,
    commit_sha: Option<&str>,
    conflict_zones: Option<Value>,
    metadata: Option<Value>,
) -> Result<AgentWorkItem, AppError> {
    let status = normalize_status(status.unwrap_or("pending"))?;
    let conflict_zones_json = json_array_string(conflict_zones)?;
    let metadata_json = json_object_string(metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_items (
            id, run_id, feature_id, work_item_id, module, service_or_domain, priority,
            release_phase, title, description, status, batch_id, agent, commit_sha,
            conflict_zones_json, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(run_id, feature_id) DO UPDATE SET
            work_item_id=COALESCE(excluded.work_item_id, agent_work_items.work_item_id),
            module=excluded.module,
            service_or_domain=excluded.service_or_domain,
            priority=excluded.priority,
            release_phase=excluded.release_phase,
            title=excluded.title,
            description=excluded.description,
            status=excluded.status,
            batch_id=excluded.batch_id,
            agent=excluded.agent,
            commit_sha=COALESCE(excluded.commit_sha, agent_work_items.commit_sha),
            conflict_zones_json=excluded.conflict_zones_json,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(module)
    .bind(service_or_domain)
    .bind(priority)
    .bind(release_phase)
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(batch_id)
    .bind(agent)
    .bind(commit_sha)
    .bind(conflict_zones_json)
    .bind(metadata_json)
    .execute(pool)
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub async fn get_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
) -> Result<AgentWorkItem, AppError> {
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work item {feature_id} not found")))
}

pub async fn list_items(
    pool: &SqlitePool,
    run_id: &str,
    status: Option<&str>,
    agent: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    match (status, agent) {
        (Some(status), Some(agent)) => {
            let status = normalize_status(status)?;
            sqlx::query_as::<_, AgentWorkItem>(
                "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
                 FROM agent_work_items WHERE run_id=? AND status=? AND agent=? ORDER BY feature_id LIMIT ? OFFSET ?",
            )
            .bind(run_id)
            .bind(status)
            .bind(agent)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
        }
        (Some(status), None) => {
            let status = normalize_status(status)?;
            sqlx::query_as::<_, AgentWorkItem>(
                "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
                 FROM agent_work_items WHERE run_id=? AND status=? ORDER BY feature_id LIMIT ? OFFSET ?",
            )
            .bind(run_id)
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
        }
        (None, Some(agent)) => sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items WHERE run_id=? AND agent=? ORDER BY feature_id LIMIT ? OFFSET ?",
        )
        .bind(run_id)
        .bind(agent)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None) => sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items WHERE run_id=? ORDER BY feature_id LIMIT ? OFFSET ?",
        )
        .bind(run_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
}

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
        "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items
         WHERE run_id=?
         ORDER BY module, service_or_domain, release_phase, feature_id",
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

    let module_rows = sqlx::query(
        "SELECT id, name, sort_order FROM modules WHERE product_id=? ORDER BY sort_order",
    )
    .bind(&resolved_product_id)
    .fetch_all(pool)
    .await?;
    let mut modules_by_key: HashMap<String, String> = HashMap::new();
    let mut max_module_sort_order = -1_i64;
    for row in module_rows {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let sort_order: i64 = row.get("sort_order");
        max_module_sort_order = max_module_sort_order.max(sort_order);
        modules_by_key.insert(normalize_catalog_key(&id), id.clone());
        modules_by_key.insert(normalize_catalog_key(&name), id);
    }

    let mut area_labels = items
        .iter()
        .map(materialized_area_label)
        .collect::<Vec<_>>();
    area_labels.sort_by_key(|label| normalize_catalog_key(label));
    area_labels.dedup_by(|a, b| normalize_catalog_key(a) == normalize_catalog_key(b));

    let mut areas: HashMap<String, MaterializedArea> = HashMap::new();
    let mut area_rows_to_insert = Vec::new();
    let mut next_module_sort_order = max_module_sort_order + 1;
    for area_label in area_labels {
        let key = normalize_catalog_key(&area_label);
        if let Some(module_id) = modules_by_key.get(&key) {
            areas.insert(
                key,
                MaterializedArea {
                    id: module_id.clone(),
                    created: false,
                },
            );
            continue;
        }
        let module_id =
            stable_materialized_id("agentwork-area", &[&resolved_product_id, &area_label]);
        areas.insert(
            key.clone(),
            MaterializedArea {
                id: module_id.clone(),
                created: true,
            },
        );
        modules_by_key.insert(key, module_id.clone());
        area_rows_to_insert.push((module_id, area_label, next_module_sort_order));
        next_module_sort_order += 1;
    }

    for chunk in area_rows_to_insert.chunks(MATERIALIZE_BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        for (module_id, area_label, sort_order) in chunk {
            sqlx::query(
                "INSERT INTO modules (
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
            .bind(module_id)
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
        "SELECT c.id, c.module_id, c.name, c.sort_order
         FROM capabilities c
         JOIN modules m ON m.id=c.module_id
         WHERE m.product_id=? AND c.parent_capability_id IS NULL",
    )
    .bind(&resolved_product_id)
    .fetch_all(pool)
    .await?;
    let mut capabilities_by_key: HashMap<(String, String), (String, i64)> = HashMap::new();
    let mut max_capability_sort_order_by_module: HashMap<String, i64> = HashMap::new();
    for row in existing_capability_rows {
        let id: String = row.get("id");
        let module_id: String = row.get("module_id");
        let name: String = row.get("name");
        let sort_order: i64 = row.get("sort_order");
        max_capability_sort_order_by_module
            .entry(module_id.clone())
            .and_modify(|current| *current = (*current).max(sort_order))
            .or_insert(sort_order);
        capabilities_by_key.insert((module_id, normalize_catalog_key(&name)), (id, sort_order));
    }

    let mut capability_specs = items
        .iter()
        .map(|item| {
            let area_key = normalize_catalog_key(&materialized_area_label(item));
            let module_id = areas
                .get(&area_key)
                .expect("area must be materialized")
                .id
                .clone();
            (module_id, materialized_capability_label(item))
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
    for (module_id, capability_label) in capability_specs {
        let key = (module_id.clone(), normalize_catalog_key(&capability_label));
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
                module_id,
                capability_label,
                *sort_order,
                false,
            ));
            continue;
        }
        let sort_order = max_capability_sort_order_by_module
            .entry(module_id.clone())
            .and_modify(|current| *current += 1)
            .or_insert(0);
        let capability_id =
            stable_materialized_id("agentwork-cap", &[run_id, &module_id, &capability_label]);
        capabilities.insert(
            key,
            MaterializedCapability {
                id: capability_id.clone(),
                created: true,
            },
        );
        capability_rows_to_upsert.push((
            capability_id,
            module_id,
            capability_label,
            *sort_order,
            true,
        ));
    }

    for chunk in capability_rows_to_upsert.chunks(MATERIALIZE_BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        for (capability_id, module_id, capability_label, sort_order, created) in chunk {
            if !*created && !capability_id.starts_with("agentwork-cap-") {
                continue;
            }
            sqlx::query(
                "INSERT INTO capabilities (
                    id, module_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, status, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, NULL, 0, 'capability', ?, ?, ?, ?, '', '', 'medium', 'medium', 'draft', ?, '', '')
                 ON CONFLICT(id) DO UPDATE SET
                    module_id=excluded.module_id,
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
            .bind(module_id)
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
        let module_id = areas
            .get(&area_key)
            .expect("area must be materialized")
            .id
            .clone();
        let capability_key = (
            module_id.clone(),
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
            module_id,
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
                    id, module_id, parent_capability_id, level, node_kind, sort_order,
                    name, description, acceptance_criteria, explanation, examples,
                    priority, risk, status, technical_notes, implementation_notes, test_guidance
                 )
                 VALUES (?, ?, ?, 1, 'feature', ?, ?, ?, ?, '', '', ?, 'medium', ?, ?, '', '')
                 ON CONFLICT(id) DO UPDATE SET
                    module_id=excluded.module_id,
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
            .bind(&feature.module_id)
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
                        id, product_id, module_id, capability_id, source_node_id, source_node_type,
                        parent_work_item_id, title, problem_statement, description,
                        acceptance_criteria, constraints, work_item_type, priority, complexity,
                        status, sort_order
                     )
                     VALUES (?, ?, ?, ?, ?, 'capability', NULL, ?, ?, ?, ?, ?, 'story', ?, 'medium', ?, 0)
                     ON CONFLICT(id) DO UPDATE SET
                        product_id=excluded.product_id,
                        module_id=excluded.module_id,
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
                .bind(&feature.module_id)
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
        run_id,
        "catalog_materialized",
        None,
        None,
        None,
        None,
        None,
        Some("completed"),
        Some("Agent-work rows materialized into catalog features and visible work items."),
        Some(serde_json::to_value(&result)?),
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
         JOIN modules m ON m.id=c.module_id
         WHERE m.product_id=? AND c.node_kind='feature' AND c.technical_notes LIKE '%Roadmap feature id:%'",
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
        run_id,
        "catalog_work_items_linked",
        None,
        None,
        None,
        None,
        None,
        Some("completed"),
        Some("Linked existing catalog work items to agent-work rows."),
        Some(serde_json::to_value(&result)?),
    )
    .await?;

    Ok(result)
}

async fn ready_item_count(pool: &SqlitePool, run_id: &str) -> Result<i64, AppError> {
    sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM agent_work_items item
         WHERE item.run_id=? AND item.status='pending'
           AND NOT EXISTS (
             SELECT 1
             FROM agent_work_dependencies dep
             LEFT JOIN agent_work_items prereq
               ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
             WHERE dep.run_id=item.run_id
               AND dep.feature_id=item.feature_id
               AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
           )",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_ready_items(
    pool: &SqlitePool,
    run_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items item
         WHERE item.run_id=? AND item.status='pending'
           AND NOT EXISTS (
             SELECT 1
             FROM agent_work_dependencies dep
             LEFT JOIN agent_work_items prereq
               ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
             WHERE dep.run_id=item.run_id
               AND dep.feature_id=item.feature_id
               AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
           )
         ORDER BY
           CASE priority WHEN 'P0' THEN 0 WHEN 'critical' THEN 0 WHEN 'P1' THEN 1 WHEN 'high' THEN 1 WHEN 'P2' THEN 2 WHEN 'medium' THEN 2 ELSE 3 END,
           feature_id
         LIMIT ? OFFSET ?",
    )
    .bind(run_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn upsert_dependency(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    depends_on_feature_id: &str,
    dependency_kind: Option<&str>,
    metadata: Option<Value>,
) -> Result<AgentWorkDependency, AppError> {
    if feature_id == depends_on_feature_id {
        return Err(AppError::Validation(
            "feature_id cannot depend on itself".to_string(),
        ));
    }
    let metadata_json = json_object_string(metadata)?;
    sqlx::query(
        "INSERT INTO agent_work_dependencies (
            id, run_id, feature_id, depends_on_feature_id, dependency_kind, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(run_id, feature_id, depends_on_feature_id) DO UPDATE SET
            dependency_kind=excluded.dependency_kind,
            metadata_json=excluded.metadata_json,
            updated_at=datetime('now')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .bind(dependency_kind.unwrap_or("blocks"))
    .bind(metadata_json)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentWorkDependency>(
        "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
         FROM agent_work_dependencies
         WHERE run_id=? AND feature_id=? AND depends_on_feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn delete_dependency(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    depends_on_feature_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM agent_work_dependencies
         WHERE run_id=? AND feature_id=? AND depends_on_feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(depends_on_feature_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_dependencies(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: Option<&str>,
) -> Result<Vec<AgentWorkDependency>, AppError> {
    if let Some(feature_id) = feature_id {
        sqlx::query_as::<_, AgentWorkDependency>(
            "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
             FROM agent_work_dependencies
             WHERE run_id=? AND feature_id=?
             ORDER BY depends_on_feature_id",
        )
        .bind(run_id)
        .bind(feature_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    } else {
        sqlx::query_as::<_, AgentWorkDependency>(
            "SELECT id,run_id,feature_id,depends_on_feature_id,dependency_kind,metadata_json,created_at,updated_at
             FROM agent_work_dependencies
             WHERE run_id=?
             ORDER BY feature_id, depends_on_feature_id",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }
}

pub async fn claim_next_item(
    pool: &SqlitePool,
    run_id: &str,
    agent: &str,
    batch_id: Option<&str>,
    selection_rule: Option<&str>,
    lease_seconds: Option<i64>,
) -> Result<Option<AgentWorkClaim>, AppError> {
    let lease_seconds = lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS);
    let lease_modifier = lease_modifier(lease_seconds);
    let batch_id = batch_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("batch-{}", uuid::Uuid::new_v4()));
    let claim_token = uuid::Uuid::new_v4().to_string();
    let mut conn = pool.acquire().await?;

    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
    let result = async {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND released_at IS NULL AND lease_expires_at <= datetime('now')",
        )
        .bind(run_id)
        .execute(&mut *conn)
        .await?;

        let candidates = sqlx::query_as::<_, AgentWorkItem>(
            "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
             FROM agent_work_items
             WHERE run_id=? AND status='pending'
               AND NOT EXISTS (
                 SELECT 1
                 FROM agent_work_dependencies dep
                 LEFT JOIN agent_work_items prereq
                   ON prereq.run_id=dep.run_id AND prereq.feature_id=dep.depends_on_feature_id
                 WHERE dep.run_id=agent_work_items.run_id
                   AND dep.feature_id=agent_work_items.feature_id
                   AND COALESCE(prereq.status, 'missing') NOT IN ('committed','skipped')
               )
             ORDER BY
               CASE priority WHEN 'P0' THEN 0 WHEN 'critical' THEN 0 WHEN 'P1' THEN 1 WHEN 'high' THEN 1 WHEN 'P2' THEN 2 WHEN 'medium' THEN 2 ELSE 3 END,
               feature_id
             LIMIT ?",
        )
        .bind(run_id)
        .bind(MAX_CANDIDATES)
        .fetch_all(&mut *conn)
        .await?;

        for candidate in candidates {
            let zones = parse_conflict_zones(&candidate)?;
            let mut blocked = false;
            for zone in &zones {
                let active_lock: Option<i64> = sqlx::query_scalar(
                    "SELECT 1 FROM agent_work_locks
                     WHERE run_id=? AND zone_key=? AND released_at IS NULL
                     LIMIT 1",
                )
                .bind(run_id)
                .bind(zone)
                .fetch_optional(&mut *conn)
                .await?;
                if active_lock.is_some() {
                    blocked = true;
                    break;
                }
            }
            if blocked {
                continue;
            }

            let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
                .bind(&lease_modifier)
                .fetch_one(&mut *conn)
                .await?;

            let updated = sqlx::query(
                "UPDATE agent_work_items
                 SET status='claimed',
                     batch_id=?,
                     agent=?,
                     claim_token=?,
                     lease_expires_at=?,
                     heartbeat_at=datetime('now'),
                     updated_at=datetime('now')
                 WHERE run_id=? AND feature_id=? AND status='pending'",
            )
            .bind(&batch_id)
            .bind(agent)
            .bind(&claim_token)
            .bind(&lease_expires_at)
            .bind(run_id)
            .bind(&candidate.feature_id)
            .execute(&mut *conn)
            .await?;

            if updated.rows_affected() == 0 {
                continue;
            }

            sqlx::query(
                "INSERT INTO agent_work_batches (id, run_id, status, selection_rule, agent)
                 VALUES (?, ?, 'claimed', ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    status='claimed',
                    selection_rule=COALESCE(excluded.selection_rule, agent_work_batches.selection_rule),
                    agent=excluded.agent,
                    updated_at=datetime('now')",
            )
            .bind(&batch_id)
            .bind(run_id)
            .bind(selection_rule)
            .bind(agent)
            .execute(&mut *conn)
            .await?;

            for zone in &zones {
                sqlx::query(
                    "INSERT INTO agent_work_locks (
                        id, run_id, zone_key, batch_id, feature_id, agent, claim_token, lease_expires_at
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(run_id)
                .bind(zone)
                .bind(&batch_id)
                .bind(&candidate.feature_id)
                .bind(agent)
                .bind(&claim_token)
                .bind(&lease_expires_at)
                .execute(&mut *conn)
                .await?;
            }

            sqlx::query(
                "UPDATE agent_work_runs
                 SET current_batch_id=?, next_action=?, updated_at=datetime('now')
                 WHERE id=?",
            )
            .bind(&batch_id)
            .bind(format!("Implement {}", candidate.feature_id))
            .bind(run_id)
            .execute(&mut *conn)
            .await?;

            sqlx::query(
                "INSERT INTO agent_work_events (
                    run_id, event_type, batch_id, feature_id, work_item_id, agent, status, details
                 ) VALUES (?, 'claim', ?, ?, ?, ?, 'claimed', ?)",
            )
            .bind(run_id)
            .bind(&batch_id)
            .bind(&candidate.feature_id)
            .bind(&candidate.work_item_id)
            .bind(agent)
            .bind(selection_rule.unwrap_or("claimed next pending feature"))
            .execute(&mut *conn)
            .await?;

            let item = get_item_with_conn(&mut conn, run_id, &candidate.feature_id).await?;
            let batch = get_batch_with_conn(&mut conn, &batch_id).await?;
            return Ok(Some(AgentWorkClaim {
                item,
                batch,
                claim_token,
                lease_expires_at,
                conflict_zones: zones,
            }));
        }

        Ok(None)
    }
    .await;

    match result {
        Ok(value) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(value)
        }
        Err(error) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(error)
        }
    }
}

async fn get_item_with_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    run_id: &str,
    feature_id: &str,
) -> Result<AgentWorkItem, AppError> {
    sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .fetch_one(&mut **conn)
    .await
    .map_err(AppError::from)
}

async fn get_batch_with_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    batch_id: &str,
) -> Result<AgentWorkBatch, AppError> {
    sqlx::query_as::<_, AgentWorkBatch>(
        "SELECT id,run_id,status,selection_rule,agent,commit_sha,metadata_json,started_at,completed_at,updated_at
         FROM agent_work_batches WHERE id=?",
    )
    .bind(batch_id)
    .fetch_one(&mut **conn)
    .await
    .map_err(AppError::from)
}

pub async fn heartbeat_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    claim_token: &str,
    lease_seconds: Option<i64>,
) -> Result<AgentWorkItem, AppError> {
    let lease_modifier = lease_modifier(lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS));
    let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
        .bind(&lease_modifier)
        .fetch_one(pool)
        .await?;
    let result = sqlx::query(
        "UPDATE agent_work_items
         SET lease_expires_at=?, heartbeat_at=datetime('now'), updated_at=datetime('now')
         WHERE run_id=? AND feature_id=? AND claim_token=? AND status IN ('claimed','in_progress','implemented')",
    )
    .bind(&lease_expires_at)
    .bind(run_id)
    .bind(feature_id)
    .bind(claim_token)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Validation(
            "No active claim matched run_id, feature_id, and claim_token.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE agent_work_locks
         SET lease_expires_at=?, updated_at=datetime('now')
         WHERE run_id=? AND feature_id=? AND claim_token=? AND released_at IS NULL",
    )
    .bind(&lease_expires_at)
    .bind(run_id)
    .bind(feature_id)
    .bind(claim_token)
    .execute(pool)
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub async fn update_item_status(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    status: &str,
    agent: Option<&str>,
    batch_id: Option<&str>,
    claim_token: Option<&str>,
    commit_sha: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkItem, AppError> {
    let status = normalize_status(status)?;
    if let Some(claim_token) = claim_token {
        let current: Option<String> = sqlx::query_scalar(
            "SELECT claim_token FROM agent_work_items WHERE run_id=? AND feature_id=?",
        )
        .bind(run_id)
        .bind(feature_id)
        .fetch_optional(pool)
        .await?;
        if current.as_deref() != Some(claim_token) {
            return Err(AppError::Validation(
                "claim_token does not match the current claim.".to_string(),
            ));
        }
    }

    sqlx::query(
        "UPDATE agent_work_items
         SET status=?,
             agent=COALESCE(?, agent),
             batch_id=COALESCE(?, batch_id),
             commit_sha=COALESCE(?, commit_sha),
             updated_at=datetime('now')
         WHERE run_id=? AND feature_id=?",
    )
    .bind(status)
    .bind(agent)
    .bind(batch_id)
    .bind(commit_sha)
    .bind(run_id)
    .bind(feature_id)
    .execute(pool)
    .await?;

    if matches!(status, "committed" | "blocked" | "skipped" | "cancelled") {
        release_item_locks(pool, run_id, feature_id, claim_token).await?;
    }

    append_event(
        pool,
        run_id,
        "status",
        batch_id,
        Some(feature_id),
        None,
        agent,
        None,
        Some(status),
        details,
        None,
    )
    .await?;

    get_item(pool, run_id, feature_id).await
}

pub async fn release_item_locks(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    claim_token: Option<&str>,
) -> Result<(), AppError> {
    if let Some(claim_token) = claim_token {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND feature_id=? AND claim_token=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(claim_token)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND feature_id=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(feature_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn requeue_item(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkItem, AppError> {
    let current = get_item(pool, run_id, feature_id).await?;
    release_item_locks(pool, run_id, feature_id, current.claim_token.as_deref()).await?;
    sqlx::query(
        "UPDATE agent_work_items
         SET status='pending',
             batch_id=NULL,
             agent=NULL,
             claim_token=NULL,
             lease_expires_at=NULL,
             heartbeat_at=NULL,
             updated_at=datetime('now')
         WHERE run_id=? AND feature_id=?",
    )
    .bind(run_id)
    .bind(feature_id)
    .execute(pool)
    .await?;
    append_event(
        pool,
        run_id,
        "requeue",
        current.batch_id.as_deref(),
        Some(feature_id),
        current.work_item_id.as_deref(),
        agent,
        None,
        Some("pending"),
        details,
        None,
    )
    .await?;
    get_item(pool, run_id, feature_id).await
}

pub async fn requeue_expired_items(
    pool: &SqlitePool,
    run_id: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<Vec<AgentWorkItem>, AppError> {
    let expired = sqlx::query_as::<_, AgentWorkItem>(
        "SELECT id,run_id,feature_id,work_item_id,module,service_or_domain,priority,release_phase,title,description,status,batch_id,agent,commit_sha,claim_token,lease_expires_at,heartbeat_at,conflict_zones_json,metadata_json,created_at,updated_at
         FROM agent_work_items
         WHERE run_id=?
           AND status IN ('claimed','in_progress')
           AND lease_expires_at IS NOT NULL
           AND lease_expires_at <= datetime('now')
         ORDER BY lease_expires_at",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    let mut requeued = Vec::new();
    for item in expired {
        requeued.push(requeue_item(pool, run_id, &item.feature_id, agent, details).await?);
    }
    Ok(requeued)
}

pub async fn list_active_locks(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkLock>, AppError> {
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks
         WHERE run_id=? AND released_at IS NULL
         ORDER BY zone_key",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_conflict_zones(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkConflictZoneSummary>, AppError> {
    sqlx::query_as::<_, AgentWorkConflictZoneSummary>(
        "SELECT
            zone_key,
            COUNT(*) AS active_locks,
            COALESCE(GROUP_CONCAT(DISTINCT agent), '') AS agents,
            COALESCE(GROUP_CONCAT(DISTINCT feature_id), '') AS feature_ids,
            MIN(lease_expires_at) AS earliest_expiry
         FROM agent_work_locks
         WHERE run_id=? AND released_at IS NULL
         GROUP BY zone_key
         ORDER BY zone_key",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn inspect_conflict_zone(
    pool: &SqlitePool,
    run_id: &str,
    zone_key: &str,
) -> Result<Vec<AgentWorkLock>, AppError> {
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks
         WHERE run_id=? AND zone_key=? AND released_at IS NULL
         ORDER BY lease_expires_at",
    )
    .bind(run_id)
    .bind(zone_key)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn reserve_conflict_zone(
    pool: &SqlitePool,
    run_id: &str,
    zone_key: &str,
    agent: &str,
    batch_id: Option<&str>,
    feature_id: Option<&str>,
    claim_token: Option<&str>,
    lease_seconds: Option<i64>,
) -> Result<AgentWorkLock, AppError> {
    let existing = inspect_conflict_zone(pool, run_id, zone_key).await?;
    if !existing.is_empty() {
        return Err(AppError::Validation(format!(
            "Conflict zone '{zone_key}' is already reserved."
        )));
    }
    let claim_token = claim_token
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let lease_expires_at: String = sqlx::query_scalar("SELECT datetime('now', ?)")
        .bind(lease_modifier(
            lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS),
        ))
        .fetch_one(pool)
        .await?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agent_work_locks (
            id, run_id, zone_key, batch_id, feature_id, agent, claim_token, lease_expires_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(run_id)
    .bind(zone_key)
    .bind(batch_id)
    .bind(feature_id)
    .bind(agent)
    .bind(&claim_token)
    .bind(&lease_expires_at)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentWorkLock>(
        "SELECT id,run_id,zone_key,batch_id,feature_id,agent,claim_token,lease_expires_at,released_at,created_at,updated_at
         FROM agent_work_locks WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn release_conflict_zone(
    pool: &SqlitePool,
    run_id: &str,
    zone_key: &str,
    claim_token: Option<&str>,
) -> Result<(), AppError> {
    if let Some(claim_token) = claim_token {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND zone_key=? AND claim_token=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(zone_key)
        .bind(claim_token)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE agent_work_locks
             SET released_at=datetime('now'), updated_at=datetime('now')
             WHERE run_id=? AND zone_key=? AND released_at IS NULL",
        )
        .bind(run_id)
        .bind(zone_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn complete_batch(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: &str,
    status: &str,
    agent: Option<&str>,
    commit_sha: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkBatch, AppError> {
    let status = normalize_batch_status(status)?;
    sqlx::query(
        "INSERT INTO agent_work_batches (id, run_id, status, agent, commit_sha, completed_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            status=excluded.status,
            agent=COALESCE(excluded.agent, agent_work_batches.agent),
            commit_sha=COALESCE(excluded.commit_sha, agent_work_batches.commit_sha),
            completed_at=CASE
                WHEN excluded.status IN ('committed','blocked','skipped','cancelled') THEN datetime('now')
                ELSE agent_work_batches.completed_at
            END,
            updated_at=datetime('now')",
    )
    .bind(batch_id)
    .bind(run_id)
    .bind(status)
    .bind(agent)
    .bind(commit_sha)
    .execute(pool)
    .await?;
    append_event(
        pool,
        run_id,
        "batch_status",
        Some(batch_id),
        None,
        None,
        agent,
        None,
        Some(status),
        details,
        None,
    )
    .await?;
    get_batch(pool, batch_id).await
}

pub async fn get_batch(pool: &SqlitePool, batch_id: &str) -> Result<AgentWorkBatch, AppError> {
    sqlx::query_as::<_, AgentWorkBatch>(
        "SELECT id,run_id,status,selection_rule,agent,commit_sha,metadata_json,started_at,completed_at,updated_at
         FROM agent_work_batches WHERE id=?",
    )
    .bind(batch_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work batch {batch_id} not found")))
}

pub async fn link_commit(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: &str,
    feature_ids: &[String],
    commit_sha: &str,
    agent: Option<&str>,
    details: Option<&str>,
) -> Result<AgentWorkBatch, AppError> {
    for feature_id in feature_ids {
        sqlx::query(
            "UPDATE agent_work_items
             SET status='committed',
                 batch_id=?,
                 agent=COALESCE(?, agent),
                 commit_sha=?,
                 updated_at=datetime('now')
             WHERE run_id=? AND feature_id=?",
        )
        .bind(batch_id)
        .bind(agent)
        .bind(commit_sha)
        .bind(run_id)
        .bind(feature_id)
        .execute(pool)
        .await?;
        release_item_locks(pool, run_id, feature_id, None).await?;
        append_event(
            pool,
            run_id,
            "commit",
            Some(batch_id),
            Some(feature_id),
            None,
            agent,
            Some("git commit"),
            Some("committed"),
            details,
            None,
        )
        .await?;
    }
    sqlx::query(
        "UPDATE agent_work_runs
         SET last_commit_sha=?, current_batch_id=NULL, updated_at=datetime('now')
         WHERE id=?",
    )
    .bind(commit_sha)
    .bind(run_id)
    .execute(pool)
    .await?;
    complete_batch(
        pool,
        run_id,
        batch_id,
        "committed",
        agent,
        Some(commit_sha),
        details,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn append_event(
    pool: &SqlitePool,
    run_id: &str,
    event_type: &str,
    batch_id: Option<&str>,
    feature_id: Option<&str>,
    work_item_id: Option<&str>,
    agent: Option<&str>,
    command: Option<&str>,
    status: Option<&str>,
    details: Option<&str>,
    metadata: Option<Value>,
) -> Result<AgentWorkEvent, AppError> {
    let metadata_json = json_object_string(metadata)?;
    let id = sqlx::query(
        "INSERT INTO agent_work_events (
            run_id, event_type, batch_id, feature_id, work_item_id, agent, command, status, details, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(run_id)
    .bind(event_type)
    .bind(batch_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(agent)
    .bind(command)
    .bind(status)
    .bind(details)
    .bind(metadata_json)
    .execute(pool)
    .await?
    .last_insert_rowid();
    sqlx::query_as::<_, AgentWorkEvent>(
        "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
         FROM agent_work_events WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_events(
    pool: &SqlitePool,
    run_id: &str,
    after_id: Option<i64>,
    feature_id: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvent>, AppError> {
    let limit = limit.clamp(1, 1000);
    match (after_id, feature_id) {
        (Some(after_id), Some(feature_id)) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND id>? AND feature_id=?
             ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(after_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (Some(after_id), None) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND id>? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, Some(feature_id)) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? AND feature_id=? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None) => sqlx::query_as::<_, AgentWorkEvent>(
            "SELECT id,run_id,ts,event_type,batch_id,feature_id,work_item_id,agent,command,status,details,metadata_json
             FROM agent_work_events WHERE run_id=? ORDER BY id DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn append_evidence(
    pool: &SqlitePool,
    run_id: &str,
    batch_id: Option<&str>,
    feature_id: Option<&str>,
    work_item_id: Option<&str>,
    agent: Option<&str>,
    evidence_type: &str,
    command: Option<&str>,
    exit_code: Option<i64>,
    status: Option<&str>,
    summary: &str,
    details: &str,
    changed_files: Option<Value>,
    artifact_refs: Option<Value>,
    metadata: Option<Value>,
) -> Result<AgentWorkEvidence, AppError> {
    let changed_files_json = json_array_string(changed_files)?;
    let artifact_refs_json = json_array_string(artifact_refs)?;
    let metadata_json = json_object_string(metadata)?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agent_work_evidence (
            id, run_id, batch_id, feature_id, work_item_id, agent, evidence_type,
            command, exit_code, status, summary, details, changed_files_json,
            artifact_refs_json, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(run_id)
    .bind(batch_id)
    .bind(feature_id)
    .bind(work_item_id)
    .bind(agent)
    .bind(evidence_type)
    .bind(command)
    .bind(exit_code)
    .bind(status)
    .bind(summary)
    .bind(details)
    .bind(changed_files_json)
    .bind(artifact_refs_json)
    .bind(metadata_json)
    .execute(pool)
    .await?;
    append_event(
        pool,
        run_id,
        "evidence",
        batch_id,
        feature_id,
        work_item_id,
        agent,
        command,
        status,
        Some(summary),
        None,
    )
    .await?;
    get_evidence(pool, &id).await
}

pub async fn get_evidence(pool: &SqlitePool, id: &str) -> Result<AgentWorkEvidence, AppError> {
    sqlx::query_as::<_, AgentWorkEvidence>(
        "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
         FROM agent_work_evidence WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Agent work evidence {id} not found")))
}

pub async fn list_evidence(
    pool: &SqlitePool,
    run_id: &str,
    feature_id: Option<&str>,
    batch_id: Option<&str>,
    agent: Option<&str>,
    limit: i64,
) -> Result<Vec<AgentWorkEvidence>, AppError> {
    let limit = limit.clamp(1, 1000);
    match (feature_id, batch_id, agent) {
        (Some(feature_id), _, _) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND feature_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(feature_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, Some(batch_id), _) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND batch_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(batch_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None, Some(agent)) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? AND agent=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(agent)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
        (None, None, None) => sqlx::query_as::<_, AgentWorkEvidence>(
            "SELECT id,run_id,batch_id,feature_id,work_item_id,agent,evidence_type,command,exit_code,status,summary,details,changed_files_json,artifact_refs_json,metadata_json,created_at
             FROM agent_work_evidence WHERE run_id=? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from),
    }
}

pub async fn get_run_health(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<AgentWorkRunHealth, AppError> {
    let summary = get_run_summary(pool, run_id, 20).await?;
    let ready_items = ready_item_count(pool, run_id).await?;
    let expired_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_items
         WHERE run_id=?
           AND status IN ('claimed','in_progress')
           AND lease_expires_at IS NOT NULL
           AND lease_expires_at <= datetime('now')",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let blocked_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_work_items WHERE run_id=? AND status='blocked'",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let active_agents: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT agent) FROM agent_work_items
         WHERE run_id=? AND agent IS NOT NULL AND status IN ('claimed','in_progress','implemented')",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let active_conflict_zones: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT zone_key) FROM agent_work_locks WHERE run_id=? AND released_at IS NULL",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let latest_evidence = list_evidence(pool, run_id, None, None, None, 20).await?;
    Ok(AgentWorkRunHealth {
        summary,
        ready_items,
        expired_claims,
        blocked_items,
        active_agents,
        active_conflict_zones,
        latest_evidence,
    })
}

pub async fn list_agent_activity(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<AgentWorkAgentActivity>, AppError> {
    sqlx::query_as::<_, AgentWorkAgentActivity>(
        "SELECT
            item.agent AS agent,
            SUM(CASE WHEN item.status IN ('claimed','in_progress','implemented') THEN 1 ELSE 0 END) AS active_items,
            SUM(CASE WHEN item.status='claimed' THEN 1 ELSE 0 END) AS claimed_items,
            SUM(CASE WHEN item.status='in_progress' THEN 1 ELSE 0 END) AS in_progress_items,
            SUM(CASE WHEN item.status='implemented' THEN 1 ELSE 0 END) AS implemented_items,
            MAX(item.heartbeat_at) AS latest_heartbeat_at,
            (
                SELECT MAX(event.ts)
                FROM agent_work_events event
                WHERE event.run_id=item.run_id AND event.agent=item.agent
            ) AS latest_event_at
         FROM agent_work_items item
         WHERE item.run_id=? AND item.agent IS NOT NULL
         GROUP BY item.agent
         ORDER BY latest_heartbeat_at DESC, latest_event_at DESC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn import_legacy_checkpoint(
    pool: &SqlitePool,
    checkpoint_path: &str,
    run_id: Option<&str>,
    source_label: Option<&str>,
) -> Result<Value, AppError> {
    if !Path::new(checkpoint_path).exists() {
        return Err(AppError::NotFound(format!(
            "Legacy checkpoint {checkpoint_path} not found"
        )));
    }

    let db_url = format!("sqlite:{checkpoint_path}");
    let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(false);
    let legacy_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    let legacy_run = sqlx::query(
        "SELECT id, roadmap_hash, last_commit, current_batch_id, next_action FROM runs ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_optional(&legacy_pool)
    .await?;
    let imported_run_id = run_id
        .map(str::to_string)
        .or_else(|| legacy_run.as_ref().map(|row| row.get::<String, _>("id")))
        .unwrap_or_else(|| format!("import-{}", uuid::Uuid::new_v4()));

    if let Some(row) = legacy_run {
        let roadmap_hash: String = row.get("roadmap_hash");
        let last_commit: Option<String> = row.get("last_commit");
        let current_batch_id: Option<String> = row.get("current_batch_id");
        let next_action: Option<String> = row.get("next_action");
        upsert_run(
            pool,
            &imported_run_id,
            None,
            None,
            &roadmap_hash,
            Some("active"),
            last_commit.as_deref(),
            current_batch_id.as_deref(),
            next_action.as_deref(),
            Some(serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint"),
                "checkpointPath": checkpoint_path
            })),
        )
        .await?;
    } else {
        upsert_run(
            pool,
            &imported_run_id,
            None,
            None,
            "",
            Some("active"),
            None,
            None,
            None,
            Some(serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint"),
                "checkpointPath": checkpoint_path
            })),
        )
        .await?;
    }

    let feature_rows = sqlx::query(
        "SELECT feature_id, module, service_or_domain, priority, release_phase, status, batch_id, agent, commit_sha
         FROM feature_progress",
    )
    .fetch_all(&legacy_pool)
    .await?;
    for row in &feature_rows {
        let feature_id: String = row.get("feature_id");
        let module: String = row.get("module");
        let service_or_domain: Option<String> = row.get("service_or_domain");
        let priority: Option<String> = row.get("priority");
        let release_phase: Option<String> = row.get("release_phase");
        let status: String = row.get("status");
        let batch_id: Option<String> = row.get("batch_id");
        let agent: Option<String> = row.get("agent");
        let commit_sha: Option<String> = row.get("commit_sha");
        upsert_item(
            pool,
            &imported_run_id,
            &feature_id,
            None,
            &module,
            service_or_domain.as_deref(),
            priority.as_deref(),
            release_phase.as_deref(),
            &feature_id,
            "",
            Some(&status),
            batch_id.as_deref(),
            agent.as_deref(),
            commit_sha.as_deref(),
            None,
            Some(serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint"),
                "checkpointPath": checkpoint_path
            })),
        )
        .await?;
    }

    let batch_rows = sqlx::query(
        "SELECT id, status, selection_rule, agent, completed_at, commit_sha FROM batches",
    )
    .fetch_all(&legacy_pool)
    .await?;
    for row in &batch_rows {
        let raw_status: String = row.get("status");
        let status = if raw_status.trim().eq_ignore_ascii_case("pending") {
            "claimed"
        } else {
            normalize_batch_status(&raw_status)?
        };
        sqlx::query(
            "INSERT INTO agent_work_batches (id, run_id, status, selection_rule, agent, completed_at, commit_sha, metadata_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                run_id=excluded.run_id,
                status=excluded.status,
                selection_rule=excluded.selection_rule,
                agent=excluded.agent,
                completed_at=excluded.completed_at,
                commit_sha=excluded.commit_sha,
                metadata_json=excluded.metadata_json,
                updated_at=datetime('now')",
        )
        .bind(row.get::<String, _>("id"))
        .bind(&imported_run_id)
        .bind(status)
        .bind(row.get::<Option<String>, _>("selection_rule"))
        .bind(row.get::<Option<String>, _>("agent"))
        .bind(row.get::<Option<String>, _>("completed_at"))
        .bind(row.get::<Option<String>, _>("commit_sha"))
        .bind(serde_json::json!({
            "source": source_label.unwrap_or("legacy_checkpoint"),
            "checkpointPath": checkpoint_path
        }).to_string())
        .execute(pool)
        .await?;
    }

    let event_rows = sqlx::query(
        "SELECT ts, event_type, batch_id, feature_id, agent, command, status, details FROM events",
    )
    .fetch_all(&legacy_pool)
    .await?;
    for row in &event_rows {
        sqlx::query(
            "INSERT INTO agent_work_events (
                run_id, ts, event_type, batch_id, feature_id, agent, command, status, details, metadata_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&imported_run_id)
        .bind(row.get::<String, _>("ts"))
        .bind(row.get::<String, _>("event_type"))
        .bind(row.get::<Option<String>, _>("batch_id"))
        .bind(row.get::<Option<String>, _>("feature_id"))
        .bind(row.get::<Option<String>, _>("agent"))
        .bind(row.get::<Option<String>, _>("command"))
        .bind(row.get::<Option<String>, _>("status"))
        .bind(row.get::<Option<String>, _>("details"))
        .bind(serde_json::json!({
            "source": source_label.unwrap_or("legacy_checkpoint"),
            "checkpointPath": checkpoint_path
        }).to_string())
        .execute(pool)
        .await?;
    }

    append_event(
        pool,
        &imported_run_id,
        "legacy_import",
        None,
        None,
        None,
        None,
        None,
        Some("imported"),
        Some(checkpoint_path),
        Some(serde_json::json!({
            "source": source_label.unwrap_or("legacy_checkpoint")
        })),
    )
    .await?;

    Ok(serde_json::json!({
        "runId": imported_run_id,
        "featuresImported": feature_rows.len(),
        "batchesImported": batch_rows.len(),
        "eventsImported": event_rows.len()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{db as db_service, product_repo, work_item_repo};

    fn make_temp_dir(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aruvi_agent_work_repo_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    async fn create_test_pool(name: &str) -> SqlitePool {
        let temp_root = make_temp_dir(name);
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool")
    }

    #[tokio::test]
    async fn claim_next_item_holds_conflict_zone_until_commit_releases_it() {
        let pool = create_test_pool("claim_locks").await;

        upsert_run(
            &pool,
            "run-test",
            None,
            None,
            "roadmap-hash",
            None,
            None,
            None,
            Some("Start"),
            None,
        )
        .await
        .expect("run should be created");

        upsert_item(
            &pool,
            "run-test",
            "01-01",
            None,
            "calculator-core",
            None,
            Some("P0"),
            None,
            "Expression parser",
            "",
            None,
            None,
            None,
            None,
            Some(serde_json::json!(["module:calculator-core"])),
            None,
        )
        .await
        .expect("first item should be created");
        upsert_item(
            &pool,
            "run-test",
            "01-02",
            None,
            "calculator-core",
            None,
            Some("P0"),
            None,
            "Operator precedence",
            "",
            None,
            None,
            None,
            None,
            Some(serde_json::json!(["module:calculator-core"])),
            None,
        )
        .await
        .expect("second item should be created");

        let first_claim = claim_next_item(
            &pool,
            "run-test",
            "agent-a",
            Some("batch-a"),
            Some("first pass"),
            Some(300),
        )
        .await
        .expect("claim should succeed")
        .expect("an item should be claimed");

        assert_eq!(first_claim.item.feature_id, "01-01");
        assert_eq!(
            first_claim.conflict_zones,
            vec!["module:calculator-core".to_string()]
        );

        let blocked_claim = claim_next_item(
            &pool,
            "run-test",
            "agent-b",
            Some("batch-b"),
            Some("same module"),
            Some(300),
        )
        .await
        .expect("claim attempt should not error");
        assert!(blocked_claim.is_none());

        link_commit(
            &pool,
            "run-test",
            "batch-a",
            &[first_claim.item.feature_id.clone()],
            "abc123",
            Some("agent-a"),
            Some("implemented parser"),
        )
        .await
        .expect("commit should release locks");

        let second_claim = claim_next_item(
            &pool,
            "run-test",
            "agent-b",
            Some("batch-b"),
            Some("after commit"),
            Some(300),
        )
        .await
        .expect("claim should succeed after lock release")
        .expect("second item should be claimable");

        assert_eq!(second_claim.item.feature_id, "01-02");
        assert_eq!(list_active_locks(&pool, "run-test").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn claim_next_item_skips_rows_with_unmet_dependencies() {
        let pool = create_test_pool("claim_dependencies").await;

        upsert_run(
            &pool,
            "run-deps",
            None,
            None,
            "roadmap-hash",
            None,
            None,
            None,
            Some("Start"),
            None,
        )
        .await
        .expect("run should be created");

        upsert_item(
            &pool,
            "run-deps",
            "01-foundation",
            None,
            "calculator-core",
            None,
            Some("P0"),
            None,
            "Foundation",
            "",
            None,
            None,
            None,
            None,
            Some(serde_json::json!(["feature:01-foundation"])),
            None,
        )
        .await
        .expect("foundation should be created");
        upsert_item(
            &pool,
            "run-deps",
            "02-dependent",
            None,
            "calculator-core",
            None,
            Some("P0"),
            None,
            "Dependent",
            "",
            None,
            None,
            None,
            None,
            Some(serde_json::json!(["feature:02-dependent"])),
            None,
        )
        .await
        .expect("dependent should be created");
        upsert_dependency(
            &pool,
            "run-deps",
            "02-dependent",
            "01-foundation",
            None,
            None,
        )
        .await
        .expect("dependency should be created");

        let ready = list_ready_items(&pool, "run-deps", 10, 0)
            .await
            .expect("ready items should list");
        assert_eq!(
            ready
                .iter()
                .map(|item| item.feature_id.as_str())
                .collect::<Vec<_>>(),
            vec!["01-foundation"]
        );

        let first_claim = claim_next_item(
            &pool,
            "run-deps",
            "agent-a",
            Some("batch-foundation"),
            Some("dependency test"),
            Some(300),
        )
        .await
        .expect("claim should succeed")
        .expect("foundation should be claimed");
        assert_eq!(first_claim.item.feature_id, "01-foundation");

        link_commit(
            &pool,
            "run-deps",
            "batch-foundation",
            &[first_claim.item.feature_id],
            "def456",
            Some("agent-a"),
            Some("foundation complete"),
        )
        .await
        .expect("commit should complete dependency");

        let second_claim = claim_next_item(
            &pool,
            "run-deps",
            "agent-b",
            Some("batch-dependent"),
            Some("dependency ready"),
            Some(300),
        )
        .await
        .expect("claim should succeed")
        .expect("dependent should be claimable");
        assert_eq!(second_claim.item.feature_id, "02-dependent");
    }

    #[tokio::test]
    async fn materialize_catalog_creates_visible_product_tree_and_work_items() {
        let pool = create_test_pool("materialize_catalog").await;
        product_repo::create_product(
            &pool,
            "product-mayyam-test",
            "Mayyam",
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
        product_repo::create_module(
            &pool,
            "existing-area",
            "product-mayyam-test",
            "Payments",
            "",
            "",
            Some("product_area"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("existing product area should be created");
        upsert_run(
            &pool,
            "run-materialize",
            Some("product-mayyam-test"),
            None,
            "roadmap-hash",
            None,
            None,
            None,
            Some("Start"),
            None,
        )
        .await
        .expect("run should be created");
        upsert_item(
            &pool,
            "run-materialize",
            "legacy-feature-001",
            None,
            "Payments",
            Some("Checkout"),
            Some("P1"),
            Some("M1"),
            "Add checkout reconciliation",
            "Reconcile checkout events against settlements.",
            Some("pending"),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("item should be created");

        let result = materialize_catalog(&pool, "run-materialize", None, true)
            .await
            .expect("catalog materialization should succeed");

        assert_eq!(result.total_items, 1);
        assert_eq!(result.product_areas_created, 0);
        assert_eq!(result.product_areas_reused, 1);
        assert_eq!(result.capabilities_created, 1);
        assert_eq!(result.features_upserted, 1);
        assert_eq!(result.work_items_upserted, 1);

        let tree = product_repo::get_product_tree(&pool, "product-mayyam-test")
            .await
            .expect("tree should load");
        assert_eq!(tree.modules.len(), 1);
        assert_eq!(tree.modules[0].features.len(), 1);
        assert_eq!(tree.modules[0].features[0].children.len(), 1);

        let visible_work_items = work_item_repo::list_work_items(
            &pool,
            Some("product-mayyam-test"),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("work items should list");
        assert_eq!(visible_work_items.len(), 1);
        assert_eq!(visible_work_items[0].title, "Add checkout reconciliation");

        let linked_item = get_item(&pool, "run-materialize", "legacy-feature-001")
            .await
            .expect("agent work item should load");
        assert_eq!(
            linked_item.work_item_id.as_deref(),
            Some(visible_work_items[0].id.as_str())
        );
    }

    #[tokio::test]
    async fn link_catalog_work_items_reconciles_existing_imported_rows() {
        let pool = create_test_pool("link_catalog_work_items").await;
        product_repo::create_product(
            &pool,
            "product-link-test",
            "Mayyam",
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
        product_repo::create_module(
            &pool,
            "area-link-test",
            "product-link-test",
            "Payments",
            "",
            "",
            Some("product_area"),
            "",
            "",
            "",
            "",
        )
        .await
        .expect("area should be created");
        sqlx::query(
            "INSERT INTO capabilities (
                id, module_id, parent_capability_id, level, node_kind, sort_order, name, status
             )
             VALUES ('cap-link-test', 'area-link-test', NULL, 0, 'capability', 0, 'Checkout', 'draft')",
        )
        .execute(&pool)
        .await
        .expect("capability should be inserted");
        sqlx::query(
            "INSERT INTO capabilities (
                id, module_id, parent_capability_id, level, node_kind, sort_order, name, status,
                technical_notes
             )
             VALUES (
                'mayyam-feature-link-test', 'area-link-test', 'cap-link-test', 1, 'feature', 0,
                'Reconcile checkout', 'draft', 'Roadmap feature id: legacy-feature-link-test.'
             )",
        )
        .execute(&pool)
        .await
        .expect("feature should be inserted");
        sqlx::query(
            "INSERT INTO work_items (
                id, product_id, module_id, capability_id, source_node_id, source_node_type,
                title, work_item_type, status
             )
             VALUES (
                'mayyam-work-link-test', 'product-link-test', 'area-link-test', 'mayyam-feature-link-test',
                'mayyam-feature-link-test', 'capability', 'Reconcile checkout', 'story', 'draft'
             )",
        )
        .execute(&pool)
        .await
        .expect("work item should be inserted");
        upsert_run(
            &pool,
            "run-link-test",
            Some("product-link-test"),
            None,
            "roadmap-hash",
            None,
            None,
            None,
            Some("Start"),
            None,
        )
        .await
        .expect("run should be created");
        upsert_item(
            &pool,
            "run-link-test",
            "legacy-feature-link-test",
            None,
            "Payments",
            Some("Checkout"),
            Some("P1"),
            Some("M1"),
            "Reconcile checkout",
            "",
            Some("committed"),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("item should be created");

        let result = link_catalog_work_items(&pool, "run-link-test", None, true)
            .await
            .expect("linking should succeed");

        assert_eq!(result.total_items, 1);
        assert_eq!(result.already_linked, 0);
        assert_eq!(result.linked_work_items, 1);
        assert_eq!(result.missing_work_items, 0);
        assert_eq!(result.ambiguous_work_items, 0);
        assert_eq!(result.status_synced, 1);

        let linked_item = get_item(&pool, "run-link-test", "legacy-feature-link-test")
            .await
            .expect("agent work item should load");
        assert_eq!(
            linked_item.work_item_id.as_deref(),
            Some("mayyam-work-link-test")
        );

        let work_status: String =
            sqlx::query_scalar("SELECT status FROM work_items WHERE id='mayyam-work-link-test'")
                .fetch_one(&pool)
                .await
                .expect("work status should load");
        assert_eq!(work_status, "done");

        let feature_status: String = sqlx::query_scalar(
            "SELECT status FROM capabilities WHERE id='mayyam-feature-link-test'",
        )
        .fetch_one(&pool)
        .await
        .expect("feature status should load");
        assert_eq!(feature_status, "done");

        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_work_events
             WHERE run_id='run-link-test' AND event_type='catalog_work_items_linked'",
        )
        .fetch_one(&pool)
        .await
        .expect("event count should load");
        assert_eq!(event_count, 1);
    }
}

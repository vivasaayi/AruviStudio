use crate::error::AppError;
use crate::persistence::agent_work_repo::{
    append_event, normalize_batch_status, upsert_item, upsert_run, AppendAgentWorkEventInput,
    UpsertAgentWorkItemInput, UpsertAgentWorkRunInput,
};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;

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
            UpsertAgentWorkRunInput {
                id: &imported_run_id,
                product_id: None,
                repository_id: None,
                roadmap_hash: &roadmap_hash,
                status: Some("active"),
                last_commit_sha: last_commit.as_deref(),
                current_batch_id: current_batch_id.as_deref(),
                next_action: next_action.as_deref(),
                metadata: Some(serde_json::json!({
                    "source": source_label.unwrap_or("legacy_checkpoint"),
                    "checkpointPath": checkpoint_path
                })),
            },
        )
        .await?;
    } else {
        upsert_run(
            pool,
            UpsertAgentWorkRunInput {
                id: &imported_run_id,
                product_id: None,
                repository_id: None,
                roadmap_hash: "",
                status: Some("active"),
                last_commit_sha: None,
                current_batch_id: None,
                next_action: None,
                metadata: Some(serde_json::json!({
                    "source": source_label.unwrap_or("legacy_checkpoint"),
                    "checkpointPath": checkpoint_path
                })),
            },
        )
        .await?;
    }

    let feature_rows = sqlx::query(
        "SELECT feature_id, product_area, service_or_domain, priority, release_phase, status, batch_id, agent, commit_sha
         FROM feature_progress",
    )
    .fetch_all(&legacy_pool)
    .await?;
    for row in &feature_rows {
        let feature_id: String = row.get("feature_id");
        let product_area: String = row.get("product_area");
        let service_or_domain: Option<String> = row.get("service_or_domain");
        let priority: Option<String> = row.get("priority");
        let release_phase: Option<String> = row.get("release_phase");
        let status: String = row.get("status");
        let batch_id: Option<String> = row.get("batch_id");
        let agent: Option<String> = row.get("agent");
        let commit_sha: Option<String> = row.get("commit_sha");
        upsert_item(
            pool,
            UpsertAgentWorkItemInput {
                run_id: &imported_run_id,
                feature_id: &feature_id,
                work_item_id: None,
                product_area: &product_area,
                service_or_domain: service_or_domain.as_deref(),
                priority: priority.as_deref(),
                release_phase: release_phase.as_deref(),
                title: &feature_id,
                description: "",
                status: Some(&status),
                batch_id: batch_id.as_deref(),
                agent: agent.as_deref(),
                commit_sha: commit_sha.as_deref(),
                conflict_zones: None,
                metadata: Some(serde_json::json!({
                    "source": source_label.unwrap_or("legacy_checkpoint"),
                    "checkpointPath": checkpoint_path
                })),
            },
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
        .bind(
            serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint"),
                "checkpointPath": checkpoint_path
            })
            .to_string(),
        )
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
        .bind(
            serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint"),
                "checkpointPath": checkpoint_path
            })
            .to_string(),
        )
        .execute(pool)
        .await?;
    }

    append_event(
        pool,
        AppendAgentWorkEventInput {
            run_id: &imported_run_id,
            event_type: "legacy_import",
            batch_id: None,
            feature_id: None,
            work_item_id: None,
            agent: None,
            command: None,
            status: Some("imported"),
            details: Some(checkpoint_path),
            metadata: Some(serde_json::json!({
                "source": source_label.unwrap_or("legacy_checkpoint")
            })),
        },
    )
    .await?;

    Ok(serde_json::json!({
        "runId": imported_run_id,
        "featuresImported": feature_rows.len(),
        "batchesImported": batch_rows.len(),
        "eventsImported": event_rows.len()
    }))
}

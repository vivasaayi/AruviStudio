use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkRun {
    pub id: String,
    pub product_id: Option<String>,
    pub repository_id: Option<String>,
    pub roadmap_hash: String,
    pub status: String,
    pub last_commit_sha: Option<String>,
    pub current_batch_id: Option<String>,
    pub next_action: String,
    pub metadata_json: String,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkItem {
    pub id: String,
    pub run_id: String,
    pub feature_id: String,
    pub work_item_id: Option<String>,
    pub product_area: String,
    pub service_or_domain: Option<String>,
    pub priority: Option<String>,
    pub release_phase: Option<String>,
    pub title: String,
    pub description: String,
    pub status: String,
    pub batch_id: Option<String>,
    pub agent: Option<String>,
    pub commit_sha: Option<String>,
    pub claim_token: Option<String>,
    pub lease_expires_at: Option<String>,
    pub heartbeat_at: Option<String>,
    pub conflict_zones_json: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkBatch {
    pub id: String,
    pub run_id: String,
    pub status: String,
    pub selection_rule: Option<String>,
    pub agent: Option<String>,
    pub commit_sha: Option<String>,
    pub metadata_json: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkEvent {
    pub id: i64,
    pub run_id: String,
    pub ts: String,
    pub event_type: String,
    pub batch_id: Option<String>,
    pub feature_id: Option<String>,
    pub work_item_id: Option<String>,
    pub agent: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub details: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkLock {
    pub id: String,
    pub run_id: String,
    pub zone_key: String,
    pub batch_id: Option<String>,
    pub feature_id: Option<String>,
    pub agent: String,
    pub claim_token: String,
    pub lease_expires_at: String,
    pub released_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkDependency {
    pub id: String,
    pub run_id: String,
    pub feature_id: String,
    pub depends_on_feature_id: String,
    pub dependency_kind: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkEvidence {
    pub id: String,
    pub run_id: String,
    pub batch_id: Option<String>,
    pub feature_id: Option<String>,
    pub work_item_id: Option<String>,
    pub agent: Option<String>,
    pub evidence_type: String,
    pub command: Option<String>,
    pub exit_code: Option<i64>,
    pub status: Option<String>,
    pub summary: String,
    pub details: String,
    pub changed_files_json: String,
    pub artifact_refs_json: String,
    pub metadata_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkRunSummary {
    pub run: AgentWorkRun,
    pub status_counts: Vec<AgentWorkStatusCount>,
    pub active_locks: i64,
    pub latest_events: Vec<AgentWorkEvent>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkAgentActivity {
    pub agent: String,
    pub active_items: i64,
    pub claimed_items: i64,
    pub in_progress_items: i64,
    pub implemented_items: i64,
    pub latest_heartbeat_at: Option<String>,
    pub latest_event_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkRunHealth {
    pub summary: AgentWorkRunSummary,
    pub ready_items: i64,
    pub expired_claims: i64,
    pub blocked_items: i64,
    pub active_agents: i64,
    pub active_conflict_zones: i64,
    pub latest_evidence: Vec<AgentWorkEvidence>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkConflictZoneSummary {
    pub zone_key: String,
    pub active_locks: i64,
    pub agents: String,
    pub feature_ids: String,
    pub earliest_expiry: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkClaim {
    pub item: AgentWorkItem,
    pub batch: AgentWorkBatch,
    pub claim_token: String,
    pub lease_expires_at: String,
    pub conflict_zones: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkMaterializationResult {
    pub run_id: String,
    pub product_id: String,
    pub total_items: i64,
    pub product_areas_created: i64,
    pub product_areas_reused: i64,
    pub capabilities_created: i64,
    pub capabilities_reused: i64,
    pub features_upserted: i64,
    pub work_items_upserted: i64,
    pub linked_work_items: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkCatalogLinkResult {
    pub run_id: String,
    pub product_id: String,
    pub total_items: i64,
    pub already_linked: i64,
    pub linked_work_items: i64,
    pub missing_work_items: i64,
    pub ambiguous_work_items: i64,
    pub status_synced: i64,
}

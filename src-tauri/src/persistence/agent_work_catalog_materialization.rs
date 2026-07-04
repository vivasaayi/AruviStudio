use crate::domain::agent_work::AgentWorkItem;
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) const MATERIALIZE_BATCH_SIZE: usize = 1_000;

#[derive(Debug, Clone)]
pub(crate) struct MaterializedArea {
    pub(crate) id: String,
    pub(crate) created: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedCapability {
    pub(crate) id: String,
    pub(crate) created: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedFeature {
    pub(crate) item: AgentWorkItem,
    pub(crate) product_area_id: String,
    pub(crate) capability_id: String,
    pub(crate) feature_id: String,
    pub(crate) work_item_id: String,
    pub(crate) sort_order: i64,
}

pub(crate) fn stable_materialized_id(prefix: &str, parts: &[&str]) -> String {
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

pub(crate) fn normalize_catalog_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub(crate) fn materialized_area_label(item: &AgentWorkItem) -> String {
    let product_area = item.product_area.trim();
    if product_area.is_empty() {
        "Imported Agent Work".to_string()
    } else {
        product_area.to_string()
    }
}

pub(crate) fn materialized_capability_label(item: &AgentWorkItem) -> String {
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

pub(crate) fn materialized_title(item: &AgentWorkItem) -> String {
    let title = item.title.trim();
    if title.is_empty() {
        item.feature_id.clone()
    } else {
        title.to_string()
    }
}

pub(crate) fn materialized_description(item: &AgentWorkItem) -> String {
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

pub(crate) fn map_agent_priority(priority: Option<&str>) -> &'static str {
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

pub(crate) fn map_capability_status(status: &str) -> &'static str {
    match status {
        "committed" => "done",
        "claimed" | "in_progress" | "implemented" | "tests_passed" => "in_progress",
        "skipped" | "cancelled" => "archived",
        _ => "draft",
    }
}

pub(crate) fn map_work_item_status(status: &str) -> &'static str {
    match status {
        "committed" => "done",
        "claimed" | "in_progress" => "in_progress",
        "implemented" | "tests_passed" => "in_validation",
        "blocked" => "blocked",
        "skipped" | "cancelled" => "cancelled",
        _ => "draft",
    }
}

use crate::error::AppError;
use std::collections::HashMap;

pub(crate) const PRODUCT_LIFECYCLES: &[&str] = &[
    "idea",
    "incubating",
    "active",
    "maturing",
    "sunsetting",
    "retired",
];
pub(crate) const PRODUCT_HEALTHS: &[&str] = &["unknown", "healthy", "watch", "at_risk", "blocked"];
pub(crate) const PRODUCT_INVESTMENT_STATUSES: &[&str] =
    &["evaluate", "invest", "maintain", "pause", "retire"];
pub(crate) const PRIORITIES: &[&str] = &["critical", "high", "medium", "low"];
pub(crate) const RISKS: &[&str] = &["high", "medium", "low"];
pub(crate) const COMPLEXITIES: &[&str] = &["trivial", "low", "medium", "high", "very_high"];
pub(crate) const WORK_ITEM_STATUSES: &[&str] = &[
    "draft",
    "ready_for_review",
    "approved",
    "in_planning",
    "in_progress",
    "in_validation",
    "waiting_human_review",
    "done",
    "blocked",
    "failed",
    "cancelled",
];

const NODE_KINDS: &[&str] = &["product_area", "capability", "feature"];

pub(crate) fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(crate) fn clean_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn clean_ref(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn required_clean(value: Option<String>, label: &str) -> Result<String, AppError> {
    clean_option(value).ok_or_else(|| AppError::Validation(format!("missing {label}")))
}

pub(crate) fn next_sort(counters: &mut HashMap<String, i64>, key: &str) -> i64 {
    let entry = counters.entry(key.to_string()).or_insert(0);
    let value = *entry;
    *entry += 1;
    value
}

pub(crate) fn capability_sort_key(
    product_area_id: &str,
    parent_capability_id: Option<&str>,
) -> String {
    format!(
        "{}\u{1f}{}",
        product_area_id,
        parent_capability_id.unwrap_or_default()
    )
}

pub(crate) fn work_item_sort_key(
    product_id: &str,
    source_node_type: Option<&str>,
    source_node_id: Option<&str>,
    parent_work_item_id: Option<&str>,
) -> String {
    if let Some(parent_id) = parent_work_item_id {
        return format!("parent\u{1f}{parent_id}");
    }
    format!(
        "source\u{1f}{}\u{1f}{}\u{1f}{}",
        product_id,
        source_node_type.unwrap_or_default(),
        source_node_id.unwrap_or_default()
    )
}

pub(crate) fn normalize_node_kind(
    value: Option<&str>,
    default_value: &str,
) -> Result<String, AppError> {
    normalize_value(value, default_value, NODE_KINDS, "nodeKind")
}

pub(crate) fn normalize_source_node_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "product_area" => Ok("product_area".to_string()),
        "capability" | "feature" => Ok("capability".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported sourceNodeType '{other}'. Use product_area, capability, or feature."
        ))),
    }
}

pub(crate) fn normalize_work_item_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "" | "story" => Ok("story".to_string()),
        "task" => Ok("task".to_string()),
        "setup" => Ok("setup".to_string()),
        "bug" => Ok("bug".to_string()),
        "refactor" => Ok("refactor".to_string()),
        "test" => Ok("test".to_string()),
        "review" => Ok("review".to_string()),
        "security_fix" => Ok("security_fix".to_string()),
        "performance_improvement" => Ok("performance_improvement".to_string()),
        other => Err(AppError::Validation(format!(
            "Unsupported workItemType '{other}'. Use story, task, setup, bug, refactor, test, review, security_fix, or performance_improvement."
        ))),
    }
}

pub(crate) fn normalize_value(
    value: Option<&str>,
    default_value: &str,
    allowed: &[&str],
    label: &str,
) -> Result<String, AppError> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .to_ascii_lowercase()
        .replace('-', "_");
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(AppError::Validation(format!(
            "Unsupported {label} '{normalized}'. Use one of: {}.",
            allowed.join(", ")
        )))
    }
}

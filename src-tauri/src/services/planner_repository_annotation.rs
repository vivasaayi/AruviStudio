use crate::services::planner_repository_analysis::RepositoryAnalysisSnapshot;
use crate::services::planner_service::PlannerPlan;
use serde_json::{json, Value};

fn normalize_identifier_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| !ch.is_alphanumeric())
        .to_lowercase()
}

fn tokenize_for_match(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .map(normalize_identifier_token)
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>()
}

fn string_field(action: &Value, key: &str) -> Option<String> {
    action
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn target_field<'a>(action: &'a Value, key: &str) -> Option<&'a str> {
    action
        .get("target")
        .and_then(|target| match target {
            Value::Object(map) => map.get(key).and_then(Value::as_str),
            _ => None,
        })
        .or_else(|| action.get(key).and_then(Value::as_str))
}

fn action_primary_label(action: &Value) -> Option<String> {
    string_field(action, "name")
        .or_else(|| string_field(action, "title"))
        .or_else(|| string_field(action, "product_area_name"))
        .or_else(|| string_field(action, "capability_name"))
        .or_else(|| string_field(action, "work_item_name"))
        .or_else(|| target_field(action, "productName").map(ToString::to_string))
        .or_else(|| target_field(action, "productAreaName").map(ToString::to_string))
        .or_else(|| target_field(action, "capabilityName").map(ToString::to_string))
        .or_else(|| target_field(action, "workItemTitle").map(ToString::to_string))
}

fn score_evidence_match(target: &str, evidence: &str) -> usize {
    let target_tokens = tokenize_for_match(target);
    if target_tokens.is_empty() {
        return 0;
    }
    let evidence_lower = evidence.to_lowercase();
    target_tokens
        .into_iter()
        .filter(|token| evidence_lower.contains(token))
        .count()
}

fn annotate_repository_analysis_action(snapshot: &RepositoryAnalysisSnapshot, action: &mut Value) {
    let Some(primary_label) = action_primary_label(action) else {
        return;
    };

    let mut evidence_candidates = vec![];
    for doc in &snapshot.docs {
        for heading in &doc.headings {
            let line = format!("doc: {} -> {}", doc.path, heading);
            let score = score_evidence_match(&primary_label, &line);
            if score > 0 {
                evidence_candidates.push((score, line));
            }
        }
    }
    for manifest in &snapshot.manifests {
        let package_name = manifest
            .package_name
            .clone()
            .unwrap_or_else(|| manifest.path.clone());
        let line = format!(
            "manifest: {} -> {} ({})",
            manifest.path,
            package_name,
            manifest.framework_hints.join(", ")
        );
        let score = score_evidence_match(&primary_label, &line);
        if score > 0 || action.get("type").and_then(Value::as_str) == Some("create_product") {
            evidence_candidates.push((score.max(1), line));
        }
    }
    for area in &snapshot.candidate_areas {
        let line = format!(
            "candidate product area: {} -> {}",
            area.name,
            area.evidence.join(" | ")
        );
        let score = score_evidence_match(&primary_label, &line);
        if score > 0 {
            evidence_candidates.push((score + 1, line));
        }
    }
    for route in &snapshot.routes {
        let line = format!("route: {} ({})", route.route, route.path);
        let score = score_evidence_match(&primary_label, &line);
        if score > 0 {
            evidence_candidates.push((score, line));
        }
    }
    for test in &snapshot.tests {
        let line = format!(
            "test: {}{}",
            test.path,
            test.framework_hint
                .as_deref()
                .map(|hint| format!(" ({hint})"))
                .unwrap_or_default()
        );
        let score = score_evidence_match(&primary_label, &line);
        if score > 0 {
            evidence_candidates.push((score, line));
        }
    }

    evidence_candidates.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    let mut evidence = evidence_candidates
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>();
    evidence.dedup();
    if evidence.is_empty() {
        if let Some(first_doc) = snapshot.docs.first() {
            evidence.push(format!("doc: {} -> repository overview", first_doc.path));
        }
        if let Some(first_manifest) = snapshot.manifests.first() {
            evidence.push(format!("manifest: {}", first_manifest.path));
        }
    }
    evidence.truncate(5);
    let confidence = if evidence.len() >= 3 {
        "high"
    } else if evidence.len() >= 2 {
        "medium"
    } else {
        "low"
    };

    if let Some(object) = action.as_object_mut() {
        object.insert(
            "analysis".to_string(),
            json!({
                "source": "repository_analysis",
                "confidence": confidence,
                "evidence": evidence,
            }),
        );
    }
}

pub(crate) fn annotate_repository_analysis_plan(
    snapshot: &RepositoryAnalysisSnapshot,
    plan: &mut PlannerPlan,
) {
    for action in &mut plan.actions {
        annotate_repository_analysis_action(snapshot, action);
    }
}

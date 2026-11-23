//! Native rendering of caller-supplied Planner review snapshots.
use crate::error::AppError;
use crate::services::planner_review_packet::DesignReviewPacket;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::fmt::Write;
use std::time::{Duration, Instant};

const CSS: &str = include_str!("../../resources/planner_snapshot.css");
// A valid node needs at least 26 bytes in JSON; this is above what the 8 MiB input limit can contain.
const MAX_NODES: usize = 350_000;
const MAX_OUTPUT: usize = 64 * 1024 * 1024;

fn invalid() -> AppError {
    AppError::Validation("Planner packet snapshot could not be rendered".into())
}
fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("")
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn js_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value))
            if value.is_f64()
                && value
                    .as_f64()
                    .is_some_and(|n| n.fract() == 0.0 && n.abs() < 1e21) =>
        {
            let number = value.as_f64().unwrap_or_default();
            if number == 0.0 {
                "0".into()
            } else {
                format!("{number:.0}")
            }
        }
        Some(Value::Object(_)) => "[object Object]".into(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| js_string(Some(value)))
            .collect::<Vec<_>>()
            .join(","),
        Some(value) => value.to_string(),
    }
}
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Number(value)) => value.as_f64().is_some_and(|n| n != 0.0),
        Some(_) => true,
    }
}
fn template_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        _ => js_string(value),
    }
}
fn first_nonempty<'a>(values: impl IntoIterator<Item = &'a str>) -> &'a str {
    values
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("")
}
fn nullish<'a>(value: Option<&'a Value>, fallback: &'a str) -> String {
    match value {
        None | Some(Value::Null) => fallback.into(),
        value => js_string(value),
    }
}
// JavaScript's JSON.stringify emits integral floating-point values without a
// decimal fraction, including negative zero. Preserve that packet formatting.
struct JsJson<'a>(&'a Value);
struct JsSlice<'a>(&'a [Value]);
impl Serialize for JsSlice<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            seq.serialize_element(&JsJson(value))?;
        }
        seq.end()
    }
}
impl Serialize for JsJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Number(n)
                if n.is_f64()
                    && n.as_f64()
                        .is_some_and(|n| n.fract() == 0.0 && n.abs() < 1e21) =>
            {
                let value = n.as_f64().unwrap_or_default();
                if value == 0.0 {
                    serializer.serialize_i64(0)
                } else if value >= i64::MIN as f64 && value < i64::MAX as f64 {
                    serializer.serialize_i64(value as i64)
                } else if value > 0.0 && value < u64::MAX as f64 {
                    serializer.serialize_u64(value as u64)
                } else {
                    n.serialize(serializer)
                }
            }
            Value::Array(values) => JsSlice(values).serialize(serializer),
            Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, &JsJson(value))?;
                }
                map.end()
            }
            value => value.serialize(serializer),
        }
    }
}
fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(&JsJson(value)).unwrap_or_default()
}
fn pretty_member<T: Serialize + ?Sized>(value: &T) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(value)?.replace('\n', "\n  "))
}

struct Summary {
    symbol: &'static str,
    title: String,
    detail: String,
}
fn summarize(action: &Value) -> Summary {
    if !action.is_object() && !action.is_array() {
        return Summary {
            symbol: "?",
            title: "Unknown planner action".into(),
            detail: "The planner returned an empty or invalid action payload.".into(),
        };
    }
    let kind = action
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown_action");
    let target = action.get("target");
    let target_value = |key| target.and_then(|value| value.get(key));
    let scope = |include_capability: bool| {
        let mut values = vec![target_value("productName"), target_value("productAreaName")];
        if include_capability {
            values.push(target_value("capabilityName"));
        }
        values
            .into_iter()
            .filter(|v| truthy(*v))
            .map(js_string)
            .collect::<Vec<_>>()
            .join(" / ")
    };
    let name_or = |key, fallback| {
        action
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| nullish(target_value(key), fallback))
    };
    let description = text(action.get("description"));
    let (symbol, title, detail) = match kind {
        "create_product_area" => (
            "+",
            format!(
                "Create product area {}",
                name_or("productAreaName", "unnamed product area")
            ),
            if truthy(target_value("productName")) {
                format!("Product: {}", js_string(target_value("productName")))
            } else {
                "Attach to selected product.".into()
            },
        ),
        "create_capability" => (
            "+",
            format!(
                "Create capability {}",
                name_or("capabilityName", "unnamed capability")
            ),
            first_nonempty([scope(false).as_str(), "Attach to selected scope."]).to_owned(),
        ),
        "apply_capability_template" => (
            "+",
            format!(
                "Apply template {} to {}",
                nullish(action.get("templateKind"), "chapter"),
                action
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed topic")
            ),
            first_nonempty([
                scope(true).as_str(),
                description,
                "Create a product-design scaffold.",
            ])
            .to_owned(),
        ),
        "convert_capability_kind" => (
            "~",
            format!(
                "Convert capability {}",
                nullish(target_value("capabilityName"), "")
            )
            .trim()
            .to_owned(),
            format!(
                "nodeKind={} childStrategy={}",
                nullish(action.get("nodeKind"), "unknown"),
                nullish(action.get("childStrategy"), "reject")
            ),
        ),
        "create_work_item" => (
            "+",
            format!(
                "Create story/task {}",
                action
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| nullish(
                        target_value("workItemTitle"),
                        "untitled work item"
                    ))
            ),
            first_nonempty([
                scope(true).as_str(),
                description,
                "New delivery work proposal.",
            ])
            .to_owned(),
        ),
        "update_product" | "update_product_area" | "update_capability" | "update_work_item" => {
            let (label, key) = match kind {
                "update_product" => ("product", "productName"),
                "update_product_area" => ("product area", "productAreaName"),
                "update_capability" => ("capability", "capabilityName"),
                _ => ("story/task", "workItemTitle"),
            };
            (
                "~",
                format!("Update {label} {}", nullish(target_value(key), ""))
                    .trim()
                    .to_owned(),
                action
                    .get("fields")
                    .filter(|v| !v.is_null())
                    .map(pretty)
                    .unwrap_or_default(),
            )
        }
        "report_tree" => (
            "i",
            "Tree report".into(),
            if truthy(target_value("productName")) {
                js_string(target_value("productName"))
            } else {
                "All products".into()
            },
        ),
        "report_status" => (
            "i",
            "Status report".into(),
            if truthy(target_value("productName")) {
                js_string(target_value("productName"))
            } else if truthy(target_value("workItemTitle")) {
                js_string(target_value("workItemTitle"))
            } else {
                "Current scope".into()
            },
        ),
        _ => {
            let warned = matches!(
                kind,
                "approve_work_item"
                    | "approve_work_item_plan"
                    | "approve_work_item_test_review"
                    | "start_workflow"
                    | "workflow_action"
                    | "reject_work_item"
                    | "reject_work_item_plan"
                    | "archive_product"
                    | "delete_product_area"
                    | "delete_capability"
                    | "delete_work_item"
            );
            (
                if warned { "!" } else { "?" },
                kind.replace('_', " "),
                pretty(action),
            )
        }
    };
    Summary {
        symbol,
        title,
        detail,
    }
}

fn list<I, S>(items: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = String::from("<ul>");
    for item in items {
        let _ = write!(out, "<li>{}</li>", escape(item.as_ref()));
    }
    out.push_str("</ul>");
    out
}
fn section(number: usize, title: &str, body: &str) -> String {
    format!(
        "<section id=\"section-{number}\" class=\"section\"><h2>{number}. {}</h2>{body}</section>",
        escape(title)
    )
}
fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut dash = false;
    for c in value.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !result.is_empty() {
                result.push('-');
            }
            result.push(c);
            dash = false;
        } else {
            dash = true;
        }
    }
    if result.is_empty() {
        "design-review-packet".into()
    } else {
        result
    }
}

fn tree(nodes: &[Value], deadline: Instant) -> Result<String, AppError> {
    tree_with_limit(nodes, deadline, MAX_NODES, MAX_OUTPUT)
}
fn tree_with_limit(
    nodes: &[Value],
    deadline: Instant,
    max_nodes: usize,
    max_output: usize,
) -> Result<String, AppError> {
    struct Row<'a> {
        node: &'a Value,
        depth: usize,
        parent: Option<usize>,
    }
    let mut rows = Vec::new();
    let mut pending: Vec<Row<'_>> = nodes
        .iter()
        .rev()
        .map(|node| Row {
            node,
            depth: 0,
            parent: None,
        })
        .collect();
    while let Some(row) = pending.pop() {
        if rows.len() >= max_nodes || (rows.len() % 1024 == 0 && Instant::now() >= deadline) {
            return Err(invalid());
        }
        let children = row
            .node
            .get("children")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        if row.node.get("label").and_then(Value::as_str).is_none() {
            return Err(invalid());
        }
        let depth = row.depth;
        let index = rows.len();
        rows.push(row);
        for node in children.iter().rev() {
            pending.push(Row {
                node,
                depth: depth + 1,
                parent: Some(index),
            });
        }
    }
    if rows.is_empty() {
        return Ok("<div class=\"diagram\">No staged design yet.</div>".into());
    }
    let shown = &rows[..rows.len().min(100)];
    let width = 240 + shown.iter().map(|row| row.depth.min(8)).max().unwrap_or(0) * 160;
    let height = shown.len() * 50 + 20;
    let mut out = format!("<div class=\"diagram\"><svg role=\"img\" aria-label=\"Staged design hierarchy\" viewBox=\"0 0 {width} {height}\" xmlns=\"http://www.w3.org/2000/svg\">");
    for (index, row) in shown.iter().enumerate() {
        let x = 10 + row.depth.min(8) * 160;
        let y = 10 + index * 50;
        if let Some(parent) = row.parent {
            let px = 150 + shown[parent].depth.min(8) * 160;
            let py = 26 + parent * 50;
            let _ = write!(
                out,
                "<path d=\"M {px} {py} H {x} V {}\" fill=\"none\" stroke=\"#94a3b8\"/>",
                y + 16
            );
        }
        let label = text(row.node.get("label"));
        let short: String = label.chars().take(20).collect();
        let _=write!(out,"<g><title>{}</title><rect x=\"{x}\" y=\"{y}\" width=\"140\" height=\"32\" rx=\"5\" fill=\"#eff6ff\" stroke=\"#2563eb\"/><text x=\"{}\" y=\"{}\" font-size=\"11\">{}</text></g>",escape(label),x+6,y+21,escape(&short));
        if out.len() > max_output {
            return Err(invalid());
        }
    }
    out.push_str("</svg>");
    if rows.len() > 100 {
        out.push_str("<p>Diagram shows the first 100 nodes; the complete tree follows.</p>");
    }
    out.push_str("<div role=\"list\" aria-label=\"Complete staged design hierarchy\">");
    for (index, row) in rows.into_iter().enumerate() {
        if index % 1024 == 0 && Instant::now() >= deadline {
            return Err(invalid());
        }
        let _=write!(out,"<div class=\"tree-row\" role=\"listitem\" aria-level=\"{}\" style=\"padding-left:{}px\">{}</div>",row.depth+1,row.depth.min(8)*20,escape(text(row.node.get("label"))));
        if out.len() > max_output {
            return Err(invalid());
        }
    }
    out.push_str("</div></div>");
    Ok(out)
}

pub(super) fn render(snapshot: &Value) -> Result<DesignReviewPacket, AppError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let title = text(snapshot.get("title"));
    let generated = text(snapshot.get("generatedAt"));
    let products = snapshot["currentProducts"].as_array().ok_or_else(invalid)?;
    let trees = snapshot["currentProductTrees"]
        .as_array()
        .ok_or_else(invalid)?;
    let work = snapshot["currentWorkItems"]
        .as_array()
        .ok_or_else(invalid)?;
    let nodes = snapshot["draftTreeNodes"].as_array().ok_or_else(invalid)?;
    let validation = snapshot.get("validation").ok_or_else(invalid)?;
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    let plan = snapshot.get("plan").filter(|v| !v.is_null());
    let empty = Vec::new();
    let actions = match plan {
        Some(p) => p
            .get("actions")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?,
        None => &empty,
    };
    if actions.len() > MAX_NODES || issues.len() > MAX_NODES {
        return Err(invalid());
    }
    let mut summaries = Vec::with_capacity(actions.len());
    let mut summary_bytes = 0usize;
    for (index, action) in actions.iter().enumerate() {
        if index % 1024 == 0 && Instant::now() >= deadline {
            return Err(invalid());
        }
        let summary = summarize(action);
        summary_bytes = summary_bytes
            .saturating_add(summary.title.len())
            .saturating_add(summary.detail.len());
        if summary_bytes > MAX_OUTPUT / 2 {
            return Err(invalid());
        }
        summaries.push(summary);
    }
    let feature: Vec<_> = summaries
        .iter()
        .filter(|s| {
            let t = s.title.to_lowercase();
            ["create", "update", "apply", "convert"]
                .iter()
                .any(|v| t.contains(v))
        })
        .map(|s| s.title.as_str())
        .collect();
    let work_actions: Vec<_> = summaries
        .iter()
        .filter(|s| {
            let t = s.title.to_lowercase();
            t.contains("work item") || t.contains("task")
        })
        .map(|s| s.title.as_str())
        .collect();
    let mut risks: Vec<_> = issues
        .iter()
        .filter(|issue| text(issue.get("tone")) == "warn")
        .map(|issue| {
            format!(
                "{}: {}",
                template_string(issue.get("title")),
                template_string(issue.get("detail"))
            )
        })
        .collect();
    if let Some(question) = plan
        .and_then(|p| p.get("clarification_question"))
        .filter(|v| truthy(Some(v)))
    {
        risks.push(format!("Open question: {}", js_string(Some(question))));
    }
    let selected = snapshot
        .get("selectedNode")
        .and_then(|v| v.get("label"))
        .cloned()
        .unwrap_or(Value::Null);
    // Keep the legacy key order in the displayed JSON, without cloning the snapshot.
    let change_set = format!(
        "{{\n  \"generatedAt\": {},\n  \"title\": {},\n  \"selectedNode\": {},\n  \"draftTree\": {},\n  \"actions\": {},\n  \"validation\": {}\n}}",
        pretty_member(generated)?, pretty_member(title)?, pretty_member(&JsJson(&selected))?,
        pretty_member(&JsSlice(nodes))?, pretty_member(&JsSlice(actions))?, pretty_member(&JsJson(validation))?
    );
    let has_active_product = truthy(snapshot.get("activeProductName"));
    let selected_product = if has_active_product {
        js_string(snapshot.get("activeProductName"))
    } else {
        String::new()
    };
    let root_count = trees.iter().try_fold(0usize, |sum, t| {
        t.get("roots")
            .and_then(Value::as_array)
            .and_then(|roots| sum.checked_add(roots.len()))
            .ok_or_else(invalid)
    })?;
    let roots = nodes
        .iter()
        .map(|n| n.get("label").and_then(Value::as_str).ok_or_else(invalid))
        .collect::<Result<Vec<_>, _>>()?;
    let note = if snapshot.get("snapshotKind").and_then(Value::as_str) == Some("proposal") {
        "This packet uses a caller-supplied proposal snapshot. The proposal and any recorded draft are from the selected message; catalog context was gathered at export time and may have changed."
    } else {
        "This packet uses a caller-supplied snapshot, which may be stale and has not been checked against the current catalog."
    };
    let titles = [
        "Executive Summary",
        "Current State",
        "Proposed Architecture",
        "Change Diff",
        "Feature Specification",
        "UX / Design Proposal",
        "Implementation Plan",
        "Risk Review",
        "Work Breakdown",
        "Approval Section",
    ];
    let nav = titles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                "<a href=\"#section-{}\">{}. {}</a>",
                i + 1,
                i + 1,
                escape(t)
            )
        })
        .collect::<String>();
    let active = if !has_active_product {
        String::new()
    } else {
        format!(" · Active product: {}", escape(&selected_product))
    };
    let mut html=format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"/><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/><title>{} - Design Review Packet</title><style>{CSS}</style></head><body><main class=\"shell\"><p role=\"note\">{}</p><section class=\"hero\"><div class=\"eyebrow\">Design Review Packet</div><h1>{}</h1><p class=\"meta\">Generated {}{active}</p><div class=\"grid\">",escape(title),escape(note),escape(title),escape(generated));
    for (label, value) in [
        ("Current Products", products.len().to_string()),
        ("Draft Roots", nodes.len().to_string()),
        ("Proposed Changes", actions.len().to_string()),
        ("Readiness", js_string(validation.get("score"))),
    ] {
        let _=write!(html,"<div class=\"metric\"><div class=\"metric-label\">{label}</div><div class=\"metric-value\">{}</div></div>",escape(&value));
    }
    let _ = write!(html, "</div><nav class=\"toc\">{nav}</nav></section>");
    let latest = if truthy(snapshot.get("latestAssistantText")) {
        js_string(snapshot.get("latestAssistantText"))
    } else if truthy(plan.and_then(|p| p.get("assistant_response"))) {
        js_string(plan.and_then(|p| p.get("assistant_response")))
    } else {
        "This packet captures the proposed product design before the catalog is changed.".into()
    };
    let primary = if roots.is_empty() {
        "No staged structure is currently available.".into()
    } else {
        format!("Primary proposed structure: {}.", roots.join(", "))
    };
    let exec = format!(
        "<p>{}</p>{}",
        escape(&latest),
        list([
            primary,
            format!(
                "The packet includes {} proposed catalog operation(s).",
                actions.len()
            ),
            "No catalog changes should be applied until this packet is reviewed and approved."
                .into()
        ])
    );
    html.push_str(&section(1, titles[0], &exec));
    let count = if snapshot["currentWorkItemsHasMore"]
        .as_bool()
        .unwrap_or(false)
    {
        format!("The planner context is capped at the first {} story/task item(s); use Work Items for full paged delivery browsing.",work.len())
    } else {
        format!(
            "{} story/task item(s) are currently visible to the planner.",
            work.len()
        )
    };
    let context = if !has_active_product {
        "No active product context was selected.".into()
    } else {
        format!("Current active product context: {selected_product}.")
    };
    html.push_str(&section(
        2,
        titles[1],
        &list([
            format!(
                "{} product(s) exist in the current workspace.",
                products.len()
            ),
            format!("{root_count} root section(s) are loaded across current products."),
            count,
            context,
        ]),
    ));
    html.push_str(&section(3,titles[2],&format!("{}<p>The architecture diagram is generated from the staged design tree. Use it to inspect hierarchy, ownership, and major boundaries before applying changes.</p>",tree(nodes,deadline)?)));
    let diff: String = if summaries.is_empty() {
        "<p>No structured change actions are currently available.</p>".into()
    } else {
        summaries.iter().map(|s|format!("<div class=\"diff\"><div class=\"symbol\">{}</div><div><strong>{}</strong>{}</div></div>",escape(s.symbol),escape(&s.title),if s.detail.is_empty() {String::new()} else {format!("<div class=\"meta\">{}</div>",escape(&s.detail))})).collect()
    };
    html.push_str(&section(4, titles[3], &diff));
    let features = if feature.is_empty() {
        vec!["No explicit feature additions or modifications were found in the latest plan."]
    } else {
        feature
    };
    html.push_str(&section(5,titles[4],&format!("{}<p>Each feature should be reviewed for user value, acceptance criteria, edge cases, and whether it belongs in this product boundary.</p>",list(features))));
    html.push_str(&section(6,titles[5],&list(["Identify the primary user workflows affected by this design.","Review first-screen information hierarchy, navigation, empty states, loading states, error states, success states, and conflict states.","Check whether the proposed screens are operationally useful, not just visually complete.","Confirm accessibility expectations: keyboard access, readable contrast, clear focus states, and screen-reader labels."])));
    html.push_str(&section(7,titles[6],&list(["Phase 1: Apply the approved product area/capability/feature structure.","Phase 2: Generate or refine stories and tasks with acceptance criteria and dependencies.","Phase 3: Build UI/data/API changes behind reviewable stories and tasks.","Phase 4: Validate with focused tests, walkthroughs, and user review before release."])));
    if risks.is_empty() {
        risks = vec![
            "No blocking structural risks were detected by the current validation pass.".into(),
            "Review assumptions manually before applying the design.".into(),
        ];
    }
    html.push_str(&section(8, titles[7], &list(risks)));
    let breakdown = if work_actions.is_empty() {
        vec![
            "Convert approved features into implementation-ready stories and tasks.".into(),
            "Add priorities, dependencies, complexity, and acceptance criteria before delivery."
                .into(),
        ]
    } else {
        work_actions
    };
    html.push_str(&section(9, titles[8], &list(breakdown)));
    let approval=format!("<div class=\"section approval\"><h3>Recommended approval options</h3>{}<h3>Structured change set</h3><pre>{}</pre></div>",list(["Approve all proposed changes and apply them to the product catalog.","Approve selected changes only.","Request revision and regenerate this packet.","Export/share this packet for stakeholder review.","Archive this packet without applying changes."]),escape(&change_set));
    html.push_str(&section(10, titles[9], &approval));
    html.push_str("</main></body></html>");
    if html.len() > MAX_OUTPUT || Instant::now() >= deadline {
        return Err(invalid());
    }
    Ok(DesignReviewPacket {
        filename: format!("{}-design-review-packet.html", slug(title)),
        html,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_overlarge_and_malformed_hierarchies() {
        let deadline = Instant::now() + Duration::from_secs(20);
        assert!(tree(&[json!({"label":"invalid"})], deadline).is_err());
        let nodes = vec![json!({"label":"x","children":[]}); 2];
        assert!(tree_with_limit(&nodes, deadline, 1, MAX_OUTPUT).is_err());
        assert!(tree_with_limit(&nodes, deadline, 10, 100).is_err());
    }
    #[test]
    fn escapes_input_and_preserves_unicode_tree_labels() {
        let rendered = tree(
            &[json!({"label":"🦀 A <&>","children":[]})],
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert!(rendered.contains("🦀 A &lt;&amp;&gt;"));
        assert!(!rendered.contains("🦀 A <&>"));
    }
}

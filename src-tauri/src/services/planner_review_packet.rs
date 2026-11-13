use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_session::{get_or_load_session, lock_planner_session, PlannerService};
use crate::services::planner_types::PlannerTreeNode;
use crate::services::planner_validation::validate_planner_tree;
use serde::Serialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct DesignReviewPacket {
    pub filename: String,
    pub html: String,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn tree_html(nodes: &[PlannerTreeNode]) -> String {
    if nodes.is_empty() {
        return String::new();
    }
    let mut html = String::from("<ul>");
    for node in nodes {
        html.push_str(&format!("<li><strong>{}</strong>", escape(&node.label)));
        if let Some(summary) = node.summary.as_deref().filter(|value| !value.is_empty()) {
            html.push_str(&format!(" — {}", escape(summary)));
        }
        html.push_str(&tree_html(&node.children));
        html.push_str("</li>");
    }
    html.push_str("</ul>");
    html
}
fn section(number: usize, title: &str, content: &str) -> String {
    format!(
        "<section id=\"section-{number}\"><h2>{number}. {}</h2>{content}</section>",
        escape(title)
    )
}
fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 80 {
            break;
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "design-review".into()
    } else {
        out.into()
    }
}

pub async fn build_planner_review_packet(
    service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
) -> Result<DesignReviewPacket, AppError> {
    let _guard = lock_planner_session(&service, &session_id).await;
    let session = get_or_load_session(&service, db, &session_id).await?;
    let draft = session
        .draft_plan
        .as_ref()
        .ok_or_else(|| AppError::Validation("No staged design is available".into()))?;
    let nodes = build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref());
    if nodes.is_empty() {
        return Err(AppError::Validation("No staged design is available".into()));
    }
    let title = nodes[0].label.as_str();
    let products = product_repo::list_products(db).await?;
    let root_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .map(|node| node.id.as_str());
    let selected_product = root_id.and_then(|id| products.iter().find(|product| product.id == id));
    let (root_count, work_count, has_more) = if let Some(product) = selected_product {
        let tree = product_repo::summarize_product_tree(db, &product.id).await?;
        let work = work_item_repo::list_work_items_page(
            db,
            work_item_repo::WorkItemListQuery {
                product_id: Some(&product.id),
                limit: Some(51),
                ..Default::default()
            },
        )
        .await?;
        (tree.product_area_count, work.len().min(50), work.len() > 50)
    } else {
        (0, 0, false)
    };
    let validation = serde_json::to_value(validate_planner_tree(&nodes))?;
    let actions = session
        .pending_plan
        .as_ref()
        .map(|plan| plan.actions.as_slice())
        .unwrap_or(&[]);
    let latest = session
        .conversation
        .iter()
        .rev()
        .find(|entry| entry.role == "assistant")
        .map(|entry| entry.content.as_str())
        .or_else(|| {
            session
                .pending_plan
                .as_ref()
                .map(|plan| plan.assistant_response.as_str())
        })
        .unwrap_or("Review the staged design before applying changes.");
    let generated = chrono::Utc::now().to_rfc3339();
    let change_set = json!({"generatedAt":generated,"title":title,"selectedNodeId":session.selected_draft_node_id,"draftTree":nodes,"actions":actions,"validation":validation});
    let action_html = if actions.is_empty() {
        "<p>No pending structured actions are available; the staged tree remains the authoritative design.</p>".into()
    } else {
        format!(
            "<ul>{}</ul>",
            actions
                .iter()
                .map(|action| {
                    format!(
                        "<li><pre>{}</pre></li>",
                        escape(&serde_json::to_string_pretty(action).unwrap_or_default())
                    )
                })
                .collect::<String>()
        )
    };
    let issue_html = validation["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|issue| {
            format!(
                "<li>{}: {}</li>",
                escape(issue["title"].as_str().unwrap_or("Issue")),
                escape(issue["detail"].as_str().unwrap_or(""))
            )
        })
        .collect::<String>();
    let context = if selected_product.is_some() {
        format!("<p>{} product(s) exist. {} root section(s) are present in the selected product. {} story/task item(s) are visible{}.</p>", products.len(), root_count, work_count, if has_more { " (first 50; more available)" } else { "" })
    } else {
        format!("<p>{} product(s) exist. The staged root is not currently linked to a catalog product; current product details are unavailable.</p>", products.len())
    };
    let selected_actions = |filter: fn(&str) -> bool| {
        actions
            .iter()
            .filter_map(|action| {
                let kind = action.get("type").and_then(|value| value.as_str())?;
                if !filter(kind) {
                    return None;
                }
                let name = action
                    .get("name")
                    .or_else(|| action.get("title"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                Some(format!("<li>{} {}</li>", escape(kind), escape(name)))
            })
            .collect::<String>()
    };
    let features =
        selected_actions(|kind| kind.contains("capability") || kind.contains("product_area"));
    let work = selected_actions(|kind| kind.contains("work_item"));
    let sections = [
        section(1,"Executive Summary", &format!("<p>Last planner response (which may precede manual edits): {}</p><p>Review and approve this proposal before applying it.</p>",escape(latest))),
        section(2,"Current State", &context),
        section(3,"Proposed Architecture", &tree_html(&nodes)),
        section(4,"Change Diff", &action_html),
        section(5,"Feature Specification", &format!("<ul>{features}</ul><p>Review staged features for user value, acceptance criteria, edge cases and product boundaries.</p>")),
        section(6,"UX / Design Proposal", "<p>Review user flows, navigation, empty and error states, keyboard access and accessibility.</p>"),
        section(7,"Implementation Plan", "<p>Apply the approved design, refine delivery work, implement changes and validate them before release.</p>"),
        section(8,"Risk Review", &format!("<p>Readiness: {}/100</p><ul>{issue_html}</ul>", validation["score"])),
        section(9,"Work Breakdown", &format!("<ul>{work}</ul><p>Refine approved features into stories and tasks with priorities, dependencies and acceptance criteria.</p>")),
        section(10,"Approval Section", &format!("<p>Approve, request revision or archive this packet. Export does not apply changes.</p><h3>Structured change set</h3><pre>{}</pre>", escape(&serde_json::to_string_pretty(&change_set)?))),
    ].join("\n");
    let html = format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{} - Design Review Packet</title><style>body{{font-family:system-ui,sans-serif;color:#172033;background:#f8fafc;line-height:1.5}}main{{max-width:1000px;margin:auto;padding:24px}}section{{background:white;border:1px solid #d8dee8;border-radius:10px;padding:16px;margin:12px 0}}pre{{white-space:pre-wrap;overflow-wrap:anywhere;background:#f1f5f9;padding:12px}}@media print{{body{{background:white}}section{{break-inside:avoid}}}}</style></head><body><main><h1>{}</h1><p>Design Review Packet · Generated {}</p>{}</main></body></html>", escape(title), escape(title), escape(&generated), sections);
    Ok(DesignReviewPacket {
        filename: format!("{}-design-review-packet.html", slug(title)),
        html,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_escapes_untrusted_html_and_sanitizes_filename() {
        let node = PlannerTreeNode {
            id: "x".into(),
            label: "<script>alert('x')</script>".into(),
            meta: None,
            node_type: Some("product".into()),
            summary: Some("<img src=x onerror=alert(1)>".into()),
            source: None,
            confidence: None,
            evidence: vec![],
            children: vec![],
        };
        let rendered = tree_html(&[node]);
        assert!(!rendered.contains("<script>"));
        assert!(!rendered.contains("<img"));
        assert!(rendered.contains("&lt;script&gt;"));
        assert_eq!(slug("../../My Project<script>"), "my-project-script");
    }
}

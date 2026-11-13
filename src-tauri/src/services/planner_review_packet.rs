use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use crate::services::planner_draft::build_draft_tree_nodes;
use crate::services::planner_session::{get_or_load_session, lock_planner_session, PlannerService};
use crate::services::planner_types::PlannerTreeNode;
use crate::services::planner_validation::validate_planner_tree;
use serde::Serialize;
use serde_json::{json, Value};
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
fn summarize_action(action: &Value) -> (String, String, String) {
    let kind = action
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown_action");
    let field = |name: &str| action.get(name).and_then(Value::as_str).unwrap_or("");
    let target = |name: &str| {
        action
            .get("target")
            .and_then(|value| value.get(name))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let scoped = || {
        [
            target("productName"),
            target("productAreaName"),
            target("capabilityName"),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
    };
    let name = |fallback: &str| {
        if field("name").is_empty() {
            fallback.to_string()
        } else {
            field("name").to_string()
        }
    };
    match kind {
        "create_product_area" => (
            "+".into(),
            format!(
                "Create product area {}",
                name(if target("productAreaName").is_empty() {
                    "unnamed product area"
                } else {
                    target("productAreaName")
                })
            ),
            if target("productName").is_empty() {
                "Attach to selected product.".into()
            } else {
                format!("Product: {}", target("productName"))
            },
        ),
        "create_capability" => (
            "+".into(),
            format!(
                "Create capability {}",
                name(if target("capabilityName").is_empty() {
                    "unnamed capability"
                } else {
                    target("capabilityName")
                })
            ),
            scoped(),
        ),
        "create_work_item" => (
            "+".into(),
            format!(
                "Create story/task {}",
                if field("title").is_empty() {
                    if target("workItemTitle").is_empty() {
                        "untitled work item"
                    } else {
                        target("workItemTitle")
                    }
                } else {
                    field("title")
                }
            ),
            scoped(),
        ),
        "apply_capability_template" => (
            "+".into(),
            format!(
                "Apply template {} to {}",
                if field("templateKind").is_empty() {
                    "chapter"
                } else {
                    field("templateKind")
                },
                name("unnamed topic")
            ),
            scoped(),
        ),
        "convert_capability_kind" => (
            "~".into(),
            format!("Convert capability {}", target("capabilityName"))
                .trim()
                .into(),
            format!(
                "nodeKind={} childStrategy={}",
                field("nodeKind"),
                field("childStrategy")
            ),
        ),
        "update_product" | "update_product_area" | "update_capability" | "update_work_item" => {
            let (label, key) = match kind {
                "update_product" => ("product", "productName"),
                "update_product_area" => ("product area", "productAreaName"),
                "update_capability" => ("capability", "capabilityName"),
                _ => ("story/task", "workItemTitle"),
            };
            (
                "~".into(),
                format!("Update {label} {}", target(key)).trim().into(),
                action
                    .get("fields")
                    .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
                    .unwrap_or_default(),
            )
        }
        "report_tree" => (
            "i".into(),
            "Tree report".into(),
            target("productName").into(),
        ),
        "report_status" => ("i".into(), "Status report".into(), scoped()),
        _ => (
            "!".into(),
            kind.replace('_', " "),
            serde_json::to_string_pretty(action).unwrap_or_default(),
        ),
    }
}

// An offline diagram avoids external scripts in a document containing user data.
// The full accessible tree is also rendered if the visual limit is reached.
fn tree_diagram(nodes: &[PlannerTreeNode]) -> String {
    fn collect<'a>(
        nodes: &'a [PlannerTreeNode],
        depth: usize,
        parent: Option<usize>,
        rows: &mut Vec<(&'a PlannerTreeNode, usize, Option<usize>)>,
    ) {
        for node in nodes {
            if rows.len() >= 100 {
                return;
            }
            let index = rows.len();
            rows.push((node, depth.min(8), parent));
            collect(&node.children, depth + 1, Some(index), rows);
        }
    }
    let mut rows = Vec::new();
    collect(nodes, 0, None, &mut rows);
    let width = 240 + rows.iter().map(|row| row.1).max().unwrap_or(0) * 160;
    let height = rows.len() * 50 + 20;
    let mut svg = format!("<svg role=\"img\" aria-label=\"Staged design hierarchy\" viewBox=\"0 0 {width} {height}\" xmlns=\"http://www.w3.org/2000/svg\">");
    for (index, (node, depth, parent)) in rows.iter().enumerate() {
        let x = 10 + depth * 160;
        let y = 10 + index * 50;
        if let Some(parent) = parent {
            let px = 10 + rows[*parent].1 * 160 + 140;
            let py = 10 + parent * 50 + 16;
            svg.push_str(&format!(
                "<path d=\"M {px} {py} H {x} V {}\" fill=\"none\" stroke=\"#94a3b8\"/>",
                y + 16
            ));
        }
        let short: String = node.label.chars().take(20).collect();
        svg.push_str(&format!("<g><title>{}</title><rect x=\"{x}\" y=\"{y}\" width=\"140\" height=\"32\" rx=\"5\" fill=\"#eff6ff\" stroke=\"#2563eb\"/><text x=\"{}\" y=\"{}\" font-size=\"11\">{}</text></g>", escape(&node.label), x + 6, y + 21, escape(&short)));
    }
    svg.push_str("</svg>");
    if rows.len() == 100 {
        svg.push_str("<p>Diagram shows up to 100 nodes; the complete tree follows.</p>");
    }
    svg
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
    build_packet(service, db, session_id, None, None).await
}

pub async fn build_desktop_review_packet(
    service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    expected_nodes: Vec<PlannerTreeNode>,
    selected_node_id: Option<String>,
) -> Result<DesignReviewPacket, AppError> {
    build_packet(
        service,
        db,
        session_id,
        Some(expected_nodes),
        Some(selected_node_id),
    )
    .await
}

fn same_tree(left: &[PlannerTreeNode], right: &[PlannerTreeNode]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.id == b.id
                && a.label == b.label
                && a.node_type == b.node_type
                && a.summary == b.summary
                && a.source == b.source
                && a.confidence == b.confidence
                && a.evidence == b.evidence
                && same_tree(&a.children, &b.children)
        })
}

async fn build_packet(
    service: Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: String,
    expected_nodes: Option<Vec<PlannerTreeNode>>,
    selected_override: Option<Option<String>>,
) -> Result<DesignReviewPacket, AppError> {
    let _guard = lock_planner_session(&service, db, &session_id).await?;
    let session = get_or_load_session(&service, db, &session_id).await?;
    let draft = session
        .draft_plan
        .as_ref()
        .ok_or_else(|| AppError::Validation("No staged design is available".into()))?;
    let nodes = build_draft_tree_nodes(draft, session.selected_draft_node_id.as_deref());
    if nodes.is_empty() {
        return Err(AppError::Validation("No staged design is available".into()));
    }
    if expected_nodes
        .as_ref()
        .is_some_and(|expected| !same_tree(expected, &nodes))
    {
        return Err(AppError::Validation(
            "The staged design changed; refresh it before exporting".into(),
        ));
    }
    let selected_id = selected_override
        .as_ref()
        .map(|value| value.as_deref())
        .unwrap_or(session.selected_draft_node_id.as_deref());
    if selected_id.is_some_and(|id| !draft.nodes.iter().any(|node| node.id == id)) {
        return Err(AppError::Validation(
            "The selected design node no longer exists".into(),
        ));
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
    let selected_node = selected_id
        .and_then(|id| draft.nodes.iter().find(|node| node.id == id))
        .map(|node| node.name.as_str());
    let change_set = json!({"generatedAt":generated,"title":title,"selectedNode":selected_node,"selectedNodeId":selected_id,"draftTree":nodes,"draftNodes":draft.nodes,"actions":actions,"validation":validation});
    let summaries = actions.iter().map(summarize_action).collect::<Vec<_>>();
    let action_html = if actions.is_empty() {
        "<p>No pending structured actions are available; the staged tree remains the authoritative design.</p>".into()
    } else {
        format!(
            "<ul>{}</ul>",
            summaries
                .iter()
                .map(|(symbol, title, detail)| format!(
                    "<li><strong>{} {}</strong><div>{}</div></li>",
                    escape(symbol),
                    escape(title),
                    escape(detail)
                ))
                .collect::<String>()
        )
    };
    // The planner draft is not a complete catalog snapshot. Show its actual
    // staged nodes and details without claiming that omitted catalog nodes
    // are deletions or that pending model actions are a complete diff.
    let staged_html = draft
        .nodes
        .iter()
        .map(|node| {
            let details = serde_json::to_string_pretty(&node.details).unwrap_or_default();
            format!(
                "<li><strong>{}: {}</strong><div>{}</div><pre>{}</pre></li>",
                escape(&node.node_type.replace('_', " ")),
                escape(&node.name),
                escape(node.summary.as_deref().unwrap_or("")),
                escape(&details)
            )
        })
        .collect::<String>();
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
    let features = summaries
        .iter()
        .filter(|(_, title, _)| {
            ["Create", "Update", "Apply", "Convert"]
                .iter()
                .any(|prefix| title.starts_with(prefix))
        })
        .map(|(_, title, _)| format!("<li>{}</li>", escape(title)))
        .collect::<String>();
    let work = summaries
        .iter()
        .filter(|(_, title, _)| {
            title.to_lowercase().contains("story/task")
                || title.to_lowercase().contains("work item")
        })
        .map(|(_, title, _)| format!("<li>{}</li>", escape(title)))
        .collect::<String>();
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
    let navigation = titles
        .iter()
        .enumerate()
        .map(|(index, title)| {
            format!(
                "<a href=\"#section-{}\">{}. {}</a>",
                index + 1,
                index + 1,
                escape(title)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let sections = [
        section(1,"Executive Summary", &format!("<p>Last planner response (which may precede manual edits): {}</p><p>Review and approve this proposal before applying it.</p>",escape(latest))),
        section(2,"Current State", &context),
        section(3,"Proposed Architecture", &format!("{}{}", tree_diagram(&nodes), tree_html(&nodes))),
        section(4,"Change Diff", &format!("<p>Pending model actions are not a complete catalog diff. The draft does not include every existing catalog node; absence does not imply deletion.</p><h3>Pending model actions</h3>{action_html}<h3>Staged design details</h3><ul>{staged_html}</ul>")),
        section(5,"Feature Specification", &format!("<ul>{features}</ul><p>Review staged features for user value, acceptance criteria, edge cases and product boundaries.</p>")),
        section(6,"UX / Design Proposal", "<p>Review user flows, navigation, empty and error states, keyboard access and accessibility.</p>"),
        section(7,"Implementation Plan", "<p>Apply the approved design, refine delivery work, implement changes and validate them before release.</p>"),
        section(8,"Risk Review", &format!("<p>Readiness: {}/100</p><ul>{issue_html}</ul>", validation["score"])),
        section(9,"Work Breakdown", &format!("<ul>{work}</ul><p>Refine approved features into stories and tasks with priorities, dependencies and acceptance criteria.</p>")),
        section(10,"Approval Section", &format!("<p>Approve, request revision or archive this packet. Export does not apply changes.</p><h3>Structured change set</h3><pre>{}</pre>", escape(&serde_json::to_string_pretty(&change_set)?))),
    ].join("\n");
    let html = format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{} - Design Review Packet</title><style>body{{font-family:system-ui,sans-serif;color:#172033;background:#f8fafc;line-height:1.5}}main{{max-width:1000px;margin:auto;padding:24px}}section{{background:white;border:1px solid #d8dee8;border-radius:10px;padding:16px;margin:12px 0}}pre{{white-space:pre-wrap;overflow-wrap:anywhere;background:#f1f5f9;padding:12px}}svg{{max-width:100%;height:auto}}@media print{{body{{background:white}}section{{break-inside:avoid}}}}</style></head><body><main><h1>{}</h1><p>Design Review Packet · Generated {}</p><nav aria-label=\"Packet sections\">{}</nav>{}</main></body></html>", escape(title), escape(title), escape(&generated), navigation, sections);
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
        let diagram = tree_diagram(&[node.clone()]);
        assert!(diagram.contains("&lt;script&gt;"));
        assert!(!diagram.contains("<script>"));
        let rendered = tree_html(&[node]);
        assert!(!rendered.contains("<script>"));
        assert!(!rendered.contains("<img"));
        assert!(rendered.contains("&lt;script&gt;"));
        assert_eq!(slug("../../My Project<script>"), "my-project-script");
        let (_, title, detail) = summarize_action(
            &json!({"type":"create_product_area","name":"Inventory", "target":{"productName":"Store"}}),
        );
        assert_eq!(title, "Create product area Inventory");
        assert_eq!(detail, "Product: Store");
    }
}

use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo, workflow_repo};
use crate::services::planner_action_fields::target_field;
use crate::services::planner_catalog::{build_tree_nodes, find_product, find_work_item};
use crate::services::planner_types::PlannerTreeNode;
use crate::state::AppState;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) async fn execute_report_action(
    state: &AppState,
    action_type: &str,
    action: &Value,
) -> Result<Vec<String>, AppError> {
    match action_type {
        "report_status" => report_status(state, action).await,
        "report_tree" => report_tree(state, action).await,
        other => Err(AppError::Validation(format!(
            "Unsupported planner action {}",
            other
        ))),
    }
}

async fn report_status(state: &AppState, action: &Value) -> Result<Vec<String>, AppError> {
    if let Some(work_item_title) = target_field(action, "workItemTitle") {
        let work_item = find_work_item(
            &state.db,
            Some(work_item_title),
            target_field(action, "productName"),
        )
        .await?;
        let run =
            workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item.id).await?;
        let product_name = if let Some(product_id) = work_item.product_id.as_deref() {
            product_repo::get_product(&state.db, product_id)
                .await
                .ok()
                .map(|p| p.name)
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };
        let mut lines = vec![
            format!("Status for \"{}\": {}.", work_item.title, work_item.status),
            format!("Product: {}.", product_name),
        ];
        if let Some(run) = run {
            lines.push(format!(
                "Workflow: {} at {}.",
                run.status, run.current_stage
            ));
        } else {
            lines.push("Workflow: not started.".to_string());
        }
        Ok(lines)
    } else {
        let product = find_product(&state.db, target_field(action, "productName")).await?;
        let summaries =
            work_item_repo::summarize_work_items_by_scope(&state.db, Some(&product.id)).await?;
        let mut counts: HashMap<String, i64> = HashMap::new();
        for summary in summaries {
            *counts.entry(summary.status).or_insert(0) += summary.total_count;
        }
        let mut lines = vec![format!("Status for \"{}\".", product.name)];
        let mut entries = counts.into_iter().collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (status, count) in entries {
            lines.push(format!("{}: {}", status, count));
        }
        Ok(lines)
    }
}

async fn report_tree(state: &AppState, action: &Value) -> Result<Vec<String>, AppError> {
    let nodes = build_tree_nodes(&state.db, target_field(action, "productName")).await?;
    let mut lines = vec![];
    for node in nodes {
        walk_tree(&node, 0, &mut lines);
    }
    Ok(lines)
}

fn walk_tree(node: &PlannerTreeNode, depth: usize, lines: &mut Vec<String>) {
    lines.push(format!("{}{}", "  ".repeat(depth), node.label));
    for child in &node.children {
        walk_tree(child, depth + 1, lines);
    }
}

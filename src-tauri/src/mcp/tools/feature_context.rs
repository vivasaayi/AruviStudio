use super::feature_context_queries::{
    capability_ancestors, get_product_area_by_id, list_capability_children,
    list_capability_siblings, top_level_work_items_for_feature, work_item_parent_chain,
    work_item_siblings,
};
use crate::error::AppError;
use crate::persistence::{agent_work_repo, product_repo, work_item_repo};
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::Path;

pub(super) async fn build_feature_context(
    state: &AppState,
    product_id: Option<String>,
    feature_id: Option<String>,
    work_item_id: Option<String>,
    run_id: Option<String>,
    include_product_tree: bool,
    sibling_limit: i64,
) -> Result<Value, AppError> {
    if product_id.is_none() && feature_id.is_none() && work_item_id.is_none() {
        return Err(AppError::Validation(
            "Provide productId, featureId, or workItemId for feature context.".to_string(),
        ));
    }

    let selected_work_item = if let Some(work_item_id) = work_item_id.as_deref() {
        Some(work_item_repo::get_work_item(&state.db, work_item_id).await?)
    } else {
        None
    };

    let resolved_feature_id = feature_id
        .or_else(|| {
            selected_work_item
                .as_ref()
                .and_then(|item| item.capability_id.clone())
        })
        .or_else(|| {
            selected_work_item.as_ref().and_then(|item| {
                item.source_node_type.as_ref().and_then(|source_type| {
                    (source_type.to_string() == "capability")
                        .then(|| item.source_node_id.clone())
                        .flatten()
                })
            })
        });

    let feature = if let Some(feature_id) = resolved_feature_id.as_deref() {
        Some(product_repo::get_capability(&state.db, feature_id).await?)
    } else {
        None
    };

    let product_area = if let Some(feature) = feature.as_ref() {
        Some(get_product_area_by_id(&state.db, &feature.product_area_id).await?)
    } else if let Some(product_area_id) = selected_work_item
        .as_ref()
        .and_then(|item| item.product_area_id.as_deref())
    {
        Some(get_product_area_by_id(&state.db, product_area_id).await?)
    } else {
        None
    };

    let resolved_product_id = product_id
        .or_else(|| {
            selected_work_item
                .as_ref()
                .and_then(|item| item.product_id.clone())
        })
        .or_else(|| product_area.as_ref().map(|area| area.product_id.clone()));
    let product = if let Some(product_id) = resolved_product_id.as_deref() {
        Some(product_repo::get_product(&state.db, product_id).await?)
    } else {
        None
    };

    let ancestors = if let Some(feature) = feature.as_ref() {
        capability_ancestors(&state.db, feature).await?
    } else {
        Vec::new()
    };
    let feature_children = if let Some(feature) = feature.as_ref() {
        list_capability_children(
            &state.db,
            &feature.product_area_id,
            Some(&feature.id),
            sibling_limit,
        )
        .await?
    } else {
        Vec::new()
    };
    let feature_siblings = if let Some(feature) = feature.as_ref() {
        list_capability_siblings(&state.db, feature, sibling_limit).await?
    } else {
        Vec::new()
    };

    let stories = top_level_work_items_for_feature(
        &state.db,
        product.as_ref().map(|product| product.id.as_str()),
        product_area.as_ref().map(|area| area.id.as_str()),
        feature.as_ref().map(|feature| feature.id.as_str()),
        sibling_limit,
    )
    .await?;
    let mut story_contexts = Vec::new();
    for story in stories {
        let children = work_item_repo::get_sub_work_items_page(
            &state.db,
            &story.id,
            Some(sibling_limit),
            Some(0),
        )
        .await?;
        story_contexts.push(json!({
            "story": story,
            "children": children
        }));
    }

    let work_item_parents = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_parent_chain(&state.db, work_item).await?
    } else {
        Vec::new()
    };
    let work_item_siblings = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_siblings(&state.db, work_item, sibling_limit).await?
    } else {
        Vec::new()
    };
    let selected_work_item_children = if let Some(work_item) = selected_work_item.as_ref() {
        work_item_repo::get_sub_work_items_page(
            &state.db,
            &work_item.id,
            Some(sibling_limit),
            Some(0),
        )
        .await?
    } else {
        Vec::new()
    };

    let product_references = if let Some(product) = product.as_ref() {
        product_repo::list_product_references(&state.db, Some("product"), Some(&product.id)).await?
    } else {
        Vec::new()
    };
    let area_references = if let Some(area) = product_area.as_ref() {
        product_repo::list_product_references(&state.db, Some("product_area"), Some(&area.id))
            .await?
    } else {
        Vec::new()
    };
    let mut feature_references = Vec::new();
    if let Some(feature) = feature.as_ref() {
        feature_references.extend(
            product_repo::list_product_references(&state.db, Some("feature"), Some(&feature.id))
                .await?,
        );
        feature_references.extend(
            product_repo::list_product_references(&state.db, Some("capability"), Some(&feature.id))
                .await?,
        );
    }
    let work_item_references = if let Some(work_item) = selected_work_item.as_ref() {
        product_repo::list_product_references(&state.db, Some("delivery_item"), Some(&work_item.id))
            .await?
    } else {
        Vec::new()
    };

    let agent_work = if let (Some(run_id), Some(feature)) = (run_id.as_deref(), feature.as_ref()) {
        let item = match agent_work_repo::get_item(&state.db, run_id, &feature.id).await {
            Ok(item) => Some(item),
            Err(AppError::NotFound(_)) => None,
            Err(error) => return Err(error),
        };
        json!({
            "item": item,
            "dependencies": agent_work_repo::list_dependencies(&state.db, run_id, Some(&feature.id)).await?,
            "evidence": agent_work_repo::list_evidence(&state.db, run_id, Some(&feature.id), None, None, 50).await?
        })
    } else {
        Value::Null
    };

    let product_tree = if include_product_tree {
        if let Some(product) = product.as_ref() {
            Some(product_repo::get_product_tree(&state.db, &product.id).await?)
        } else {
            None
        }
    } else {
        None
    };

    Ok(json!({
        "product": product,
        "productArea": product_area,
        "feature": feature,
        "featureAncestors": ancestors,
        "featureChildren": feature_children,
        "featureSiblings": feature_siblings,
        "stories": story_contexts,
        "selectedWorkItem": selected_work_item,
        "selectedWorkItemParents": work_item_parents,
        "selectedWorkItemSiblings": work_item_siblings,
        "selectedWorkItemChildren": selected_work_item_children,
        "references": {
            "product": product_references,
            "productArea": area_references,
            "feature": feature_references,
            "workItem": work_item_references
        },
        "agentWork": agent_work,
        "productTree": product_tree
    }))
}

fn feature_context_markdown(context: &Value) -> Result<String, AppError> {
    let title = context
        .pointer("/feature/name")
        .and_then(Value::as_str)
        .or_else(|| {
            context
                .pointer("/selectedWorkItem/title")
                .and_then(Value::as_str)
        })
        .or_else(|| context.pointer("/product/name").and_then(Value::as_str))
        .unwrap_or("Feature Context");
    Ok(format!(
        "# {title}\n\nGenerated Aruvi implementation context.\n\n```json\n{}\n```\n",
        serde_json::to_string_pretty(context)?
    ))
}

pub(super) async fn export_feature_context_to_file(
    context: &Value,
    output_path: &str,
    format: &str,
) -> Result<Value, AppError> {
    let content = match format {
        "markdown" | "md" => feature_context_markdown(context)?,
        "json" | "" => serde_json::to_string_pretty(context)?,
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported context export format '{other}'. Use json or markdown."
            )))
        }
    };
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::write(path, content.as_bytes()).await?;
    Ok(json!({
        "outputPath": output_path,
        "bytesWritten": content.len(),
        "format": format
    }))
}

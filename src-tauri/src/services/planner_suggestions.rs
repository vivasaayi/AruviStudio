use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct SuggestionNode {
    pub label: String,
    pub node_type: Option<String>,
    pub meta: Option<String>,
    #[serde(default)]
    pub ancestors: Vec<SuggestionAncestor>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestionAncestor {
    pub label: String,
    pub node_type: Option<String>,
}

fn node_kind(node: &SuggestionNode) -> String {
    node.node_type.as_deref().filter(|value| !value.is_empty()).map(|value| value.replacen('_', " ", 1)).unwrap_or_else(|| {
        let meta = node.meta.as_deref().unwrap_or("");
        if meta.contains("product area") || meta.contains("product_area") { "product area".into() }
        else if meta.contains("product") { "product".into() }
        else if meta.contains("capability") { "capability".into() }
        else if meta.contains("work item") { "work item".into() }
        else { "node".into() }
    })
}

pub fn suggested_prompts(node: Option<&SuggestionNode>) -> Vec<String> {
    let Some(node) = node else { return vec![
        "Design the selected product's product areas, capabilities, features, and starter stories in one review packet.".into(),
        "Show me what is missing in this design before I apply it.".into(),
    ]; };
    let kind = node_kind(node);
    let name = &node.label;
    match kind.as_str() {
        "product" => vec![format!("Expand {name} with missing capabilities and operational areas."), format!("What is missing under {name} before I apply it?"), format!("Add notification, reporting, and integration capabilities under {name}.")],
        "product area" => vec![format!("Enhance {name} with 3 concrete capabilities."), format!("Break {name} into capabilities, features, and starter stories."), format!("What risks or missing features exist under {name}?")],
        "capability" => vec![format!("Add implementation stories and tasks under {name}."), format!("Revise {name} to be more concrete and execution-ready."), format!("What acceptance criteria or technical notes are missing for {name}?")],
        "work item" => vec![format!("Revise {name} to be more specific and testable."), format!("Split {name} into smaller stories or tasks if needed."), format!("Add risks, constraints, and acceptance criteria to {name}.")],
        _ => vec![format!("Expand {name}."), format!("What is missing under {name}?")],
    }
}

/// Match the current page's latest plan, not a durable or complete audit log.
pub fn relevant_actions(actions: &[serde_json::Value], node: Option<&SuggestionNode>) -> Vec<serde_json::Value> {
    let Some(node) = node else { return vec![]; };
    let kind = node_kind(node);
    let ancestor = |kind: &str| node.ancestors.iter().rev().find(|entry| entry.node_type.as_deref() == Some(kind)).map(|entry| entry.label.as_str());
    let product = ancestor("product");
    let product_area = ancestor("product_area");
    let parent_capability = ancestor("capability");
    actions.iter().filter(|action| {
        let target = action.get("target");
        let target_name = |key: &str| target.and_then(|value| value.get(key)).and_then(serde_json::Value::as_str);
        let action_type = action.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
        let named = |key: &str| action.get(key).and_then(serde_json::Value::as_str) == Some(node.label.as_str());
        let compatible = |key: &str, scope: Option<&str>| match (target_name(key).filter(|name| !name.trim().is_empty()), scope) {
            (Some(target), Some(current)) => target.trim().to_lowercase() == current.trim().to_lowercase(),
            _ => true,
        };
        if !compatible("productName", product) || !compatible("productAreaName", product_area) { return false; }
        match kind.as_str() {
            "product" => target_name("productName") == Some(node.label.as_str()),
            "product area" => target_name("productAreaName") == Some(node.label.as_str()) || (action_type == "create_product_area" && named("name")),
            "capability" => target_name("capabilityName") == Some(node.label.as_str()) || (matches!(action_type, "create_capability" | "apply_capability_template") && named("name") && compatible("capabilityName", parent_capability)),
            "work item" => target_name("workItemTitle") == Some(node.label.as_str()) || (action_type == "create_work_item" && named("title") && compatible("capabilityName", parent_capability)),
            _ => false,
        }
    }).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_choices_match_node_scope() {
        assert_eq!(suggested_prompts(None).len(), 2);
        let node = SuggestionNode { label: "Inventory".into(), node_type: Some("product_area".into()), meta: None, ..Default::default() };
        assert_eq!(suggested_prompts(Some(&node))[0], "Enhance Inventory with 3 concrete capabilities.");
        let legacy = SuggestionNode { label: "Inventory".into(), node_type: None, meta: Some("proposed product area".into()), ..Default::default() };
        assert_eq!(suggested_prompts(Some(&legacy)), suggested_prompts(Some(&node)));
    }
    #[test]
    fn relevant_actions_match_existing_node_rules() {
        let node = SuggestionNode { label: "Inventory".into(), node_type: Some("product_area".into()), meta: None, ..Default::default() };
        let actions = vec![serde_json::json!({"type":"create_product_area","name":"Inventory"}), serde_json::json!({"type":"update_product_area","target":{"productAreaName":"Inventory"}}), serde_json::json!({"type":"create_product_area","name":"Other"})];
        assert_eq!(relevant_actions(&actions, Some(&node)), actions[..2]);
        assert!(relevant_actions(&actions, None).is_empty());
        for (kind, action) in [
            ("product", serde_json::json!({"type":"update_product","target":{"productName":"Inventory"}})),
            ("capability", serde_json::json!({"type":"apply_capability_template","name":"Inventory"})),
            ("work_item", serde_json::json!({"type":"create_work_item","title":"Inventory"})),
        ] {
            let node = SuggestionNode { label: "Inventory".into(), node_type: Some(kind.into()), meta: None, ..Default::default() };
            assert_eq!(relevant_actions(std::slice::from_ref(&action), Some(&node)), vec![action]);
        }
    }

    #[test]
    fn relevant_actions_exclude_conflicting_known_ancestors_but_keep_ambiguous_legacy_actions() {
        let node: SuggestionNode = serde_json::from_value(serde_json::json!({
            "label":"Inventory", "node_type":"capability", "ancestors":[
                {"label":"Store", "node_type":"product"},
                {"label":"Warehouse", "node_type":"product_area"},
                {"label":"Receiving", "node_type":"capability"}
            ]
        })).unwrap();
        let same = serde_json::json!({"type":"create_capability","name":"Inventory","target":{"productName":"Store","productAreaName":"Warehouse","capabilityName":"Receiving"}});
        let wrong_product = serde_json::json!({"type":"create_capability","name":"Inventory","target":{"productName":"Other","productAreaName":"Warehouse","capabilityName":"Receiving"}});
        let wrong_area = serde_json::json!({"type":"create_capability","name":"Inventory","target":{"productName":"Store","productAreaName":"Sales","capabilityName":"Receiving"}});
        let wrong_parent = serde_json::json!({"type":"create_capability","name":"Inventory","target":{"productName":"Store","productAreaName":"Warehouse","capabilityName":"Shipping"}});
        let direct = serde_json::json!({"type":"update_capability","target":{"productName":"Store","productAreaName":"Warehouse","capabilityName":"Inventory"}});
        let normalized = serde_json::json!({"type":"update_capability","target":{"productName":" store ","productAreaName":"WAREHOUSE","capabilityName":"Inventory"}});
        let unknown = serde_json::json!({"type":"create_capability","name":"Inventory"});
        let actions = vec![same.clone(), wrong_product, wrong_area, wrong_parent, direct.clone(), normalized.clone(), unknown.clone()];
        assert_eq!(relevant_actions(&actions, Some(&node)), vec![same, direct, normalized, unknown]);

        let legacy: SuggestionNode = serde_json::from_value(serde_json::json!({"label":"Inventory","node_type":"capability"})).unwrap();
        assert_eq!(relevant_actions(&actions, Some(&legacy)).len(), actions.len());
    }
}

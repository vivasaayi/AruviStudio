use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SuggestionNode {
    pub label: String,
    pub node_type: Option<String>,
    pub meta: Option<String>,
}

pub fn suggested_prompts(node: Option<&SuggestionNode>) -> Vec<String> {
    let Some(node) = node else { return vec![
        "Design the selected product's product areas, capabilities, features, and starter stories in one review packet.".into(),
        "Show me what is missing in this design before I apply it.".into(),
    ]; };
    let kind = node.node_type.as_deref().filter(|value| !value.is_empty()).map(|value| value.replacen('_', " ", 1)).unwrap_or_else(|| {
        let meta = node.meta.as_deref().unwrap_or("");
        if meta.contains("product area") || meta.contains("product_area") { "product area".into() }
        else if meta.contains("product") { "product".into() }
        else if meta.contains("capability") { "capability".into() }
        else if meta.contains("work item") { "work item".into() }
        else { "node".into() }
    });
    let name = &node.label;
    match kind.as_str() {
        "product" => vec![format!("Expand {name} with missing capabilities and operational areas."), format!("What is missing under {name} before I apply it?"), format!("Add notification, reporting, and integration capabilities under {name}.")],
        "product area" => vec![format!("Enhance {name} with 3 concrete capabilities."), format!("Break {name} into capabilities, features, and starter stories."), format!("What risks or missing features exist under {name}?")],
        "capability" => vec![format!("Add implementation stories and tasks under {name}."), format!("Revise {name} to be more concrete and execution-ready."), format!("What acceptance criteria or technical notes are missing for {name}?")],
        "work item" => vec![format!("Revise {name} to be more specific and testable."), format!("Split {name} into smaller stories or tasks if needed."), format!("Add risks, constraints, and acceptance criteria to {name}.")],
        _ => vec![format!("Expand {name}."), format!("What is missing under {name}?")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_choices_match_node_scope() {
        assert_eq!(suggested_prompts(None).len(), 2);
        let node = SuggestionNode { label: "Inventory".into(), node_type: Some("product_area".into()), meta: None };
        assert_eq!(suggested_prompts(Some(&node))[0], "Enhance Inventory with 3 concrete capabilities.");
        let legacy = SuggestionNode { label: "Inventory".into(), node_type: None, meta: Some("proposed product area".into()) };
        assert_eq!(suggested_prompts(Some(&legacy)), suggested_prompts(Some(&node)));
    }
}

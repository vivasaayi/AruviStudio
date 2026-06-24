use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportDocument {
    #[serde(default)]
    pub(crate) product: Option<ImportProduct>,
    #[serde(default, alias = "product_areas")]
    pub(crate) product_areas: Vec<ImportProductArea>,
    #[serde(default, alias = "work_items")]
    pub(crate) work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportProduct {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) vision: Option<String>,
    #[serde(default)]
    pub(crate) goals: Vec<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) lifecycle: Option<String>,
    #[serde(default)]
    pub(crate) health: Option<String>,
    #[serde(default, alias = "owner_label")]
    pub(crate) owner_label: Option<String>,
    #[serde(default, alias = "investment_status")]
    pub(crate) investment_status: Option<String>,
    #[serde(default)]
    pub(crate) roadmap: Option<String>,
    #[serde(default)]
    pub(crate) evidence: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportProductArea {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) purpose: Option<String>,
    #[serde(default)]
    pub(crate) explanation: Option<String>,
    #[serde(default)]
    pub(crate) examples: Option<String>,
    #[serde(default, alias = "implementation_notes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(default, alias = "test_guidance")]
    pub(crate) test_guidance: Option<String>,
    #[serde(default)]
    pub(crate) capabilities: Vec<ImportCapability>,
    #[serde(default, alias = "work_items")]
    pub(crate) work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportCapability {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default, alias = "acceptance_criteria")]
    pub(crate) acceptance_criteria: Option<String>,
    #[serde(default)]
    pub(crate) explanation: Option<String>,
    #[serde(default)]
    pub(crate) examples: Option<String>,
    #[serde(default)]
    pub(crate) priority: Option<String>,
    #[serde(default)]
    pub(crate) risk: Option<String>,
    #[serde(default, alias = "technical_notes")]
    pub(crate) technical_notes: Option<String>,
    #[serde(default, alias = "implementation_notes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(default, alias = "test_guidance")]
    pub(crate) test_guidance: Option<String>,
    #[serde(default, alias = "node_kind")]
    pub(crate) node_kind: Option<String>,
    #[serde(default)]
    pub(crate) features: Vec<ImportCapability>,
    #[serde(default, alias = "children")]
    pub(crate) capabilities: Vec<ImportCapability>,
    #[serde(default, alias = "work_items")]
    pub(crate) work_items: Vec<ImportWorkItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportWorkItem {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default, alias = "product_id")]
    pub(crate) product_id: Option<String>,
    #[serde(default, alias = "product_area_id")]
    pub(crate) product_area_id: Option<String>,
    #[serde(default, alias = "capability_id")]
    pub(crate) capability_id: Option<String>,
    #[serde(default, alias = "feature_id")]
    pub(crate) feature_id: Option<String>,
    #[serde(default, alias = "source_node_id")]
    pub(crate) source_node_id: Option<String>,
    #[serde(default, alias = "source_node_type")]
    pub(crate) source_node_type: Option<String>,
    #[serde(default, alias = "parent_work_item_id")]
    pub(crate) parent_work_item_id: Option<String>,
    #[serde(default, alias = "problem_statement")]
    pub(crate) problem_statement: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default, alias = "acceptance_criteria")]
    pub(crate) acceptance_criteria: Option<String>,
    #[serde(default)]
    pub(crate) constraints: Option<String>,
    #[serde(default, alias = "work_item_type")]
    pub(crate) work_item_type: Option<String>,
    #[serde(default)]
    pub(crate) priority: Option<String>,
    #[serde(default)]
    pub(crate) complexity: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default, alias = "subtasks", alias = "children")]
    pub(crate) tasks: Vec<ImportWorkItem>,
}

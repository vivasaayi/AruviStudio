use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateProductCommand {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) vision: String,
    pub(crate) goals: String,
    pub(crate) tags: String,
    pub(crate) lifecycle: Option<String>,
    pub(crate) health: Option<String>,
    #[serde(alias = "ownerLabel")]
    pub(crate) owner_label: Option<String>,
    #[serde(alias = "investmentStatus")]
    pub(crate) investment_status: Option<String>,
    pub(crate) roadmap: Option<String>,
    pub(crate) evidence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductCommand {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) vision: Option<String>,
    pub(crate) goals: Option<String>,
    pub(crate) tags: Option<String>,
    pub(crate) lifecycle: Option<String>,
    pub(crate) health: Option<String>,
    #[serde(alias = "ownerLabel")]
    pub(crate) owner_label: Option<String>,
    #[serde(alias = "investmentStatus")]
    pub(crate) investment_status: Option<String>,
    pub(crate) roadmap: Option<String>,
    pub(crate) evidence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductAreaCommand {
    #[serde(alias = "productId")]
    pub(crate) product_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) purpose: String,
    #[serde(alias = "nodeKind")]
    pub(crate) node_kind: Option<String>,
    pub(crate) explanation: Option<String>,
    pub(crate) examples: Option<String>,
    #[serde(alias = "implementationNotes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(alias = "testGuidance")]
    pub(crate) test_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductAreaCommand {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) purpose: Option<String>,
    #[serde(alias = "nodeKind")]
    pub(crate) node_kind: Option<String>,
    pub(crate) explanation: Option<String>,
    pub(crate) examples: Option<String>,
    #[serde(alias = "implementationNotes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(alias = "testGuidance")]
    pub(crate) test_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCapabilityCommand {
    #[serde(alias = "productAreaId")]
    pub(crate) product_area_id: String,
    #[serde(alias = "parentCapabilityId")]
    pub(crate) parent_capability_id: Option<String>,
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(alias = "acceptanceCriteria")]
    pub(crate) acceptance_criteria: String,
    pub(crate) priority: String,
    pub(crate) risk: String,
    #[serde(alias = "technicalNotes")]
    pub(crate) technical_notes: String,
    #[serde(alias = "nodeKind")]
    pub(crate) node_kind: Option<String>,
    pub(crate) explanation: Option<String>,
    pub(crate) examples: Option<String>,
    #[serde(alias = "implementationNotes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(alias = "testGuidance")]
    pub(crate) test_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCapabilityCommand {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(alias = "acceptanceCriteria")]
    pub(crate) acceptance_criteria: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) risk: Option<String>,
    #[serde(alias = "technicalNotes")]
    pub(crate) technical_notes: Option<String>,
    #[serde(alias = "nodeKind")]
    pub(crate) node_kind: Option<String>,
    pub(crate) explanation: Option<String>,
    pub(crate) examples: Option<String>,
    #[serde(alias = "implementationNotes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(alias = "testGuidance")]
    pub(crate) test_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplySemanticTemplateCommand {
    #[serde(alias = "productAreaId")]
    pub(crate) product_area_id: Option<String>,
    #[serde(alias = "parentCapabilityId")]
    pub(crate) parent_capability_id: Option<String>,
    #[serde(alias = "templateKind")]
    pub(crate) template_kind: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) risk: Option<String>,
    pub(crate) explanation: Option<String>,
    pub(crate) examples: Option<String>,
    #[serde(alias = "implementationNotes")]
    pub(crate) implementation_notes: Option<String>,
    #[serde(alias = "testGuidance")]
    pub(crate) test_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductReferenceCommand {
    #[serde(alias = "scopeType")]
    pub(crate) scope_type: String,
    #[serde(alias = "scopeId")]
    pub(crate) scope_id: String,
    pub(crate) title: String,
    #[serde(alias = "referenceKind")]
    pub(crate) reference_kind: String,
    pub(crate) uri: Option<String>,
    pub(crate) content: Option<String>,
}

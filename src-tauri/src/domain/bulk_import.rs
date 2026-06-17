use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportJob {
    pub id: String,
    pub source_path: String,
    pub import_format: String,
    pub status: String,
    pub total_records: i64,
    pub processed_records: i64,
    pub product_count: i64,
    pub product_area_count: i64,
    pub capability_count: i64,
    pub feature_count: i64,
    pub work_item_count: i64,
    pub failed_records: i64,
    pub error_message: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportJobError {
    pub id: String,
    pub job_id: String,
    pub row_index: Option<i64>,
    pub record_type: String,
    pub record_id: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportJobStatus {
    pub job: BulkImportJob,
    pub errors: Vec<BulkImportJobError>,
}

#[derive(Debug, Clone)]
pub struct BulkImportProductRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub goals_json: String,
    pub tags_json: String,
    pub lifecycle: String,
    pub health: String,
    pub owner_label: String,
    pub investment_status: String,
    pub roadmap: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct BulkImportProductAreaRow {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub explanation: String,
    pub examples: String,
    pub implementation_notes: String,
    pub test_guidance: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone)]
pub struct BulkImportCapabilityRow {
    pub id: String,
    pub module_id: String,
    pub parent_capability_id: Option<String>,
    pub level: i64,
    pub node_kind: String,
    pub sort_order: i64,
    pub name: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub explanation: String,
    pub examples: String,
    pub priority: String,
    pub risk: String,
    pub technical_notes: String,
    pub implementation_notes: String,
    pub test_guidance: String,
}

#[derive(Debug, Clone)]
pub struct BulkImportWorkItemRow {
    pub id: String,
    pub product_id: String,
    pub module_id: Option<String>,
    pub capability_id: Option<String>,
    pub source_node_id: Option<String>,
    pub source_node_type: Option<String>,
    pub parent_work_item_id: Option<String>,
    pub title: String,
    pub problem_statement: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub constraints: String,
    pub work_item_type: String,
    pub priority: String,
    pub complexity: String,
    pub status: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Default)]
pub struct BulkImportRows {
    pub products: Vec<BulkImportProductRow>,
    pub product_areas: Vec<BulkImportProductAreaRow>,
    pub capabilities: Vec<BulkImportCapabilityRow>,
    pub work_items: Vec<BulkImportWorkItemRow>,
}

impl BulkImportRows {
    pub fn total_records(&self) -> i64 {
        i64::try_from(
            self.products.len()
                + self.product_areas.len()
                + self.capabilities.len()
                + self.work_items.len(),
        )
        .unwrap_or(i64::MAX)
    }
}

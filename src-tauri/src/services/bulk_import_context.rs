use crate::domain::bulk_import::BulkImportRows;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(super) struct ProductAreaScope {
    pub(super) product_id: String,
    pub(super) product_area_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct CapabilityScope {
    pub(super) product_id: String,
    pub(super) product_area_id: String,
    pub(super) capability_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct WorkItemScope {
    pub(super) product_id: String,
    pub(super) product_area_id: Option<String>,
    pub(super) capability_id: Option<String>,
    pub(super) source_node_id: Option<String>,
    pub(super) source_node_type: Option<String>,
}

#[derive(Default)]
pub(super) struct ImportBuildContext {
    pub(super) rows: BulkImportRows,
    pub(super) referenced_product_ids: HashSet<String>,
    pub(super) imported_product_ids: HashSet<String>,
    pub(super) product_areas: HashMap<String, ProductAreaScope>,
    pub(super) capabilities: HashMap<String, CapabilityScope>,
    pub(super) work_items: HashMap<String, WorkItemScope>,
    pub(super) product_area_sort: HashMap<String, i64>,
    pub(super) capability_sort: HashMap<String, i64>,
    pub(super) work_item_sort: HashMap<String, i64>,
}

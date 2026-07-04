CREATE INDEX IF NOT EXISTS idx_work_items_product_area_status_summary
ON work_items(product_id, product_area_id, status, parent_work_item_id);

CREATE INDEX IF NOT EXISTS idx_work_items_capability_status_summary
ON work_items(product_id, capability_id, status, parent_work_item_id);

CREATE INDEX IF NOT EXISTS idx_work_items_source_status_summary
ON work_items(product_id, source_node_type, source_node_id, status, parent_work_item_id);

CREATE INDEX IF NOT EXISTS idx_work_items_product_status_list
ON work_items(product_id, status, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_product_source_list
ON work_items(product_id, source_node_type, source_node_id, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_product_source_status_list
ON work_items(product_id, source_node_type, source_node_id, status, sort_order, created_at DESC);

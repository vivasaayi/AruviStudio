CREATE INDEX IF NOT EXISTS idx_work_items_list_all
ON work_items(sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_product_list
ON work_items(product_id, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_module_list
ON work_items(module_id, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_capability_list
ON work_items(capability_id, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_source_list
ON work_items(source_node_type, source_node_id, sort_order, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_items_status_list
ON work_items(status, sort_order, created_at DESC);

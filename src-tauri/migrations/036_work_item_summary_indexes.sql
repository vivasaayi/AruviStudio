CREATE INDEX IF NOT EXISTS idx_work_items_product_status_summary
ON work_items(product_id, status);

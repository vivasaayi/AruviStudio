ALTER TABLE planner_channel_bindings ADD COLUMN product_id TEXT REFERENCES products(id) ON DELETE SET NULL;

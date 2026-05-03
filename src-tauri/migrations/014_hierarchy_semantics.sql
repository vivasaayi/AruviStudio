ALTER TABLE modules ADD COLUMN node_kind TEXT NOT NULL DEFAULT 'area';

ALTER TABLE capabilities ADD COLUMN node_kind TEXT NOT NULL DEFAULT 'capability';
UPDATE capabilities
SET node_kind = CASE
    WHEN level >= 1 THEN 'rollout'
    ELSE 'capability'
END;

ALTER TABLE work_items ADD COLUMN source_node_id TEXT;
ALTER TABLE work_items ADD COLUMN source_node_type TEXT;

UPDATE work_items
SET
    source_node_id = CASE
        WHEN capability_id IS NOT NULL THEN capability_id
        WHEN module_id IS NOT NULL THEN module_id
        ELSE NULL
    END,
    source_node_type = CASE
        WHEN capability_id IS NOT NULL THEN 'capability'
        WHEN module_id IS NOT NULL THEN 'module'
        ELSE NULL
    END
WHERE source_node_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_modules_kind ON modules(node_kind);
CREATE INDEX IF NOT EXISTS idx_capabilities_kind ON capabilities(node_kind);
CREATE INDEX IF NOT EXISTS idx_work_items_source_node ON work_items(source_node_type, source_node_id);

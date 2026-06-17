-- Finish node-kind normalization for labels discovered after migration 033
-- had already been applied in local databases.

UPDATE modules
SET node_kind = 'area'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'system'
);

UPDATE modules
SET node_kind = 'area'
WHERE lower(replace(node_kind, '-', '_')) NOT IN ('area');

UPDATE capabilities
SET node_kind = 'capability'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'system'
);

UPDATE capabilities
SET node_kind = CASE
    WHEN parent_capability_id IS NULL OR level <= 0 THEN 'capability'
    ELSE 'feature'
END
WHERE lower(replace(node_kind, '-', '_')) NOT IN ('capability', 'feature');

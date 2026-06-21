-- Canonical product hierarchy terminology:
-- Product > Product Area > Capability > Feature, with Story > Task delivery items.

UPDATE modules
SET node_kind = 'product_area'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'area',
    'productarea',
    'product_area',
    'module',
    'strategic_area',
    'domain',
    'subdomain',
    'sub_domain',
    'system',
    'capability',
    'feature_set',
    'feature_group'
);

UPDATE modules
SET node_kind = 'product_area'
WHERE lower(replace(node_kind, '-', '_')) <> 'product_area';

UPDATE capabilities
SET node_kind = CASE
    WHEN lower(replace(node_kind, '-', '_')) = 'feature' THEN 'feature'
    WHEN parent_capability_id IS NULL OR level <= 0 THEN 'capability'
    ELSE 'feature'
END
WHERE lower(replace(node_kind, '-', '_')) NOT IN ('capability', 'feature');

ALTER TABLE modules RENAME TO modules_product_hierarchy_legacy;

CREATE TABLE modules (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    purpose TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    node_kind TEXT NOT NULL DEFAULT 'product_area' CHECK(node_kind IN ('product_area')),
    explanation TEXT NOT NULL DEFAULT '',
    examples TEXT NOT NULL DEFAULT '',
    implementation_notes TEXT NOT NULL DEFAULT '',
    test_guidance TEXT NOT NULL DEFAULT ''
);

INSERT INTO modules (
    id, product_id, name, description, purpose, sort_order, created_at, updated_at,
    node_kind, explanation, examples, implementation_notes, test_guidance
)
SELECT
    id, product_id, name, description, purpose, sort_order, created_at, updated_at,
    'product_area', explanation, examples, implementation_notes, test_guidance
FROM modules_product_hierarchy_legacy;

ALTER TABLE capabilities RENAME TO capabilities_product_hierarchy_legacy;

CREATE TABLE capabilities (
    id TEXT PRIMARY KEY NOT NULL,
    module_id TEXT NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    parent_capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    level INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    risk TEXT NOT NULL DEFAULT 'low' CHECK(risk IN ('high','medium','low')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','in_progress','done','archived')),
    technical_notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    node_kind TEXT NOT NULL DEFAULT 'capability' CHECK(node_kind IN ('capability','feature')),
    explanation TEXT NOT NULL DEFAULT '',
    examples TEXT NOT NULL DEFAULT '',
    implementation_notes TEXT NOT NULL DEFAULT '',
    test_guidance TEXT NOT NULL DEFAULT ''
);

INSERT INTO capabilities (
    id, module_id, parent_capability_id, level, sort_order, name, description,
    acceptance_criteria, priority, risk, status, technical_notes, created_at,
    updated_at, node_kind, explanation, examples, implementation_notes, test_guidance
)
SELECT
    id, module_id, parent_capability_id, level, sort_order, name, description,
    acceptance_criteria, priority, risk, status, technical_notes, created_at,
    updated_at,
    CASE
        WHEN lower(replace(node_kind, '-', '_')) = 'feature' THEN 'feature'
        WHEN parent_capability_id IS NULL OR level <= 0 THEN 'capability'
        ELSE 'feature'
    END,
    explanation, examples, implementation_notes, test_guidance
FROM capabilities_product_hierarchy_legacy;

ALTER TABLE product_dependencies RENAME TO product_dependencies_product_hierarchy_legacy;

CREATE TABLE product_dependencies (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    depends_on_product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    depends_on_capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    dependency_kind TEXT NOT NULL DEFAULT 'platform' CHECK(dependency_kind IN ('platform','capability','data','integration','operational','other')),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','planned','blocked','retired')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(product_id <> depends_on_product_id),
    UNIQUE(product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind)
);

INSERT INTO product_dependencies SELECT * FROM product_dependencies_product_hierarchy_legacy;

ALTER TABLE capability_slices RENAME TO capability_slices_product_hierarchy_legacy;

CREATE TABLE capability_slices (
    id TEXT PRIMARY KEY NOT NULL,
    capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','in_progress','done','archived')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO capability_slices SELECT * FROM capability_slices_product_hierarchy_legacy;

DROP TABLE capability_slices_product_hierarchy_legacy;
DROP TABLE product_dependencies_product_hierarchy_legacy;
DROP TABLE capabilities_product_hierarchy_legacy;
DROP TABLE modules_product_hierarchy_legacy;

CREATE INDEX IF NOT EXISTS idx_product_dependencies_product ON product_dependencies(product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_capability ON product_dependencies(capability_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on ON product_dependencies(depends_on_product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on_capability ON product_dependencies(depends_on_capability_id);
CREATE INDEX IF NOT EXISTS idx_capability_slices_capability ON capability_slices(capability_id);

UPDATE work_items
SET source_node_type = CASE lower(replace(source_node_type, '-', '_'))
    WHEN 'module' THEN 'product_area'
    WHEN 'area' THEN 'product_area'
    WHEN 'productarea' THEN 'product_area'
    WHEN 'product_area' THEN 'product_area'
    WHEN 'feature' THEN 'capability'
    ELSE source_node_type
END
WHERE source_node_type IS NOT NULL;

UPDATE work_items
SET
    source_node_id = module_id,
    source_node_type = 'product_area'
WHERE source_node_id IS NULL
  AND source_node_type IS NULL
  AND module_id IS NOT NULL
  AND capability_id IS NULL;

UPDATE work_items
SET
    source_node_id = capability_id,
    source_node_type = 'capability'
WHERE source_node_id IS NULL
  AND source_node_type IS NULL
  AND capability_id IS NOT NULL;

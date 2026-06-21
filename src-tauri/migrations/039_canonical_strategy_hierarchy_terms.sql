-- Canonical portfolio terminology:
-- Strategic Product Area > Domain > Sub Domain.

PRAGMA foreign_keys=off;
PRAGMA legacy_alter_table=ON;

CREATE TABLE strategy_nodes_canonical (
    id TEXT PRIMARY KEY NOT NULL,
    parent_node_id TEXT REFERENCES strategy_nodes(id) ON DELETE CASCADE,
    node_kind TEXT NOT NULL CHECK(node_kind IN ('strategic_product_area','domain','sub_domain')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner_label TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO strategy_nodes_canonical (
    id,
    parent_node_id,
    node_kind,
    name,
    description,
    owner_label,
    sort_order,
    created_at,
    updated_at
)
SELECT
    id,
    parent_node_id,
    CASE lower(replace(node_kind, '-', '_'))
        WHEN 'strategic_area' THEN 'strategic_product_area'
        WHEN 'strategic_product_area' THEN 'strategic_product_area'
        WHEN 'domain' THEN 'domain'
        WHEN 'subdomain' THEN 'sub_domain'
        WHEN 'sub_domain' THEN 'sub_domain'
        ELSE 'domain'
    END,
    name,
    description,
    owner_label,
    sort_order,
    created_at,
    updated_at
FROM strategy_nodes;

DROP TABLE strategy_nodes;
ALTER TABLE strategy_nodes_canonical RENAME TO strategy_nodes;

CREATE INDEX IF NOT EXISTS idx_strategy_nodes_parent ON strategy_nodes(parent_node_id);
CREATE INDEX IF NOT EXISTS idx_strategy_nodes_kind ON strategy_nodes(node_kind);

PRAGMA foreign_keys=on;
PRAGMA legacy_alter_table=OFF;

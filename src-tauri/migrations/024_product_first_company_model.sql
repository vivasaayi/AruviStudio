-- Product-first company model.
-- Strategy is slow-moving and separate from the product design/delivery tree.

CREATE TABLE IF NOT EXISTS strategy_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    parent_node_id TEXT REFERENCES strategy_nodes(id) ON DELETE CASCADE,
    node_kind TEXT NOT NULL CHECK(node_kind IN ('strategic_area','domain','subdomain')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner_label TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_strategy_nodes_parent ON strategy_nodes(parent_node_id);
CREATE INDEX IF NOT EXISTS idx_strategy_nodes_kind ON strategy_nodes(node_kind);

CREATE TABLE IF NOT EXISTS product_strategy_links (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    strategy_node_id TEXT NOT NULL REFERENCES strategy_nodes(id) ON DELETE CASCADE,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(product_id, strategy_node_id)
);
CREATE INDEX IF NOT EXISTS idx_product_strategy_links_product ON product_strategy_links(product_id);
CREATE INDEX IF NOT EXISTS idx_product_strategy_links_strategy ON product_strategy_links(strategy_node_id);

CREATE TABLE IF NOT EXISTS product_dependencies (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    depends_on_product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    dependency_kind TEXT NOT NULL DEFAULT 'platform' CHECK(dependency_kind IN ('platform','capability','data','integration','operational','other')),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','planned','blocked','retired')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(product_id <> depends_on_product_id),
    UNIQUE(product_id, depends_on_product_id, dependency_kind)
);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_product ON product_dependencies(product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on ON product_dependencies(depends_on_product_id);

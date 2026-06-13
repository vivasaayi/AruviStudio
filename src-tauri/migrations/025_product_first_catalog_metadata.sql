-- Product-first catalog metadata and scoped references.
-- Keep 024 immutable because it may already be applied in local databases.

ALTER TABLE products ADD COLUMN lifecycle TEXT NOT NULL DEFAULT 'incubating'
    CHECK(lifecycle IN ('idea','incubating','active','maturing','sunsetting','retired'));
ALTER TABLE products ADD COLUMN health TEXT NOT NULL DEFAULT 'unknown'
    CHECK(health IN ('unknown','healthy','watch','at_risk','blocked'));
ALTER TABLE products ADD COLUMN owner_label TEXT NOT NULL DEFAULT '';
ALTER TABLE products ADD COLUMN investment_status TEXT NOT NULL DEFAULT 'evaluate'
    CHECK(investment_status IN ('evaluate','invest','maintain','pause','retire'));
ALTER TABLE products ADD COLUMN roadmap TEXT NOT NULL DEFAULT '';
ALTER TABLE products ADD COLUMN evidence TEXT NOT NULL DEFAULT '';

CREATE TABLE product_dependencies_next (
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
INSERT INTO product_dependencies_next (
    id,
    product_id,
    depends_on_product_id,
    dependency_kind,
    description,
    status,
    created_at,
    updated_at
)
SELECT
    id,
    product_id,
    depends_on_product_id,
    dependency_kind,
    description,
    status,
    created_at,
    updated_at
FROM product_dependencies;
DROP TABLE product_dependencies;
ALTER TABLE product_dependencies_next RENAME TO product_dependencies;
CREATE INDEX IF NOT EXISTS idx_product_dependencies_product ON product_dependencies(product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_capability ON product_dependencies(capability_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on ON product_dependencies(depends_on_product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on_capability ON product_dependencies(depends_on_capability_id);

CREATE TABLE IF NOT EXISTS capability_slices (
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
CREATE INDEX IF NOT EXISTS idx_capability_slices_capability ON capability_slices(capability_id);

CREATE TABLE IF NOT EXISTS delivery_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    capability_slice_id TEXT REFERENCES capability_slices(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    delivery_kind TEXT NOT NULL DEFAULT 'implementation' CHECK(delivery_kind IN ('implementation','test','review','documentation','release','other')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready','in_progress','review','done','blocked','cancelled')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_delivery_items_product ON delivery_items(product_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_capability ON delivery_items(capability_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_slice ON delivery_items(capability_slice_id);

CREATE TABLE IF NOT EXISTS "references" (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('strategy_node','product','capability','capability_slice','delivery_item')),
    scope_id TEXT NOT NULL,
    title TEXT NOT NULL,
    reference_kind TEXT NOT NULL DEFAULT 'note' CHECK(reference_kind IN ('note','external_doc','architecture','customer_evidence','regulatory','design_packet','standard','other')),
    uri TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_references_scope ON "references"(scope_type, scope_id);

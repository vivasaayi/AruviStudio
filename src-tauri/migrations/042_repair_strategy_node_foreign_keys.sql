PRAGMA foreign_keys=off;
PRAGMA legacy_alter_table=ON;

ALTER TABLE product_strategy_links RENAME TO product_strategy_links_strategy_fk_legacy;

CREATE TABLE product_strategy_links (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    strategy_node_id TEXT NOT NULL REFERENCES strategy_nodes(id) ON DELETE CASCADE,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(product_id, strategy_node_id)
);

INSERT INTO product_strategy_links (
    id,
    product_id,
    strategy_node_id,
    is_primary,
    created_at
)
SELECT
    id,
    product_id,
    strategy_node_id,
    is_primary,
    created_at
FROM product_strategy_links_strategy_fk_legacy;

DROP TABLE product_strategy_links_strategy_fk_legacy;

CREATE INDEX IF NOT EXISTS idx_product_strategy_links_product ON product_strategy_links(product_id);
CREATE INDEX IF NOT EXISTS idx_product_strategy_links_strategy ON product_strategy_links(strategy_node_id);

PRAGMA legacy_alter_table=OFF;
PRAGMA foreign_keys=on;

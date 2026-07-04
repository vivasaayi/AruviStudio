PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

ALTER TABLE work_items RENAME TO work_items_legacy_story_type;

CREATE TABLE work_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    module_id TEXT REFERENCES modules(id) ON DELETE SET NULL,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    source_node_id TEXT,
    source_node_type TEXT,
    parent_work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    problem_statement TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    constraints TEXT NOT NULL DEFAULT '',
    work_item_type TEXT NOT NULL DEFAULT 'story' CHECK(work_item_type IN ('story','task','setup','bug','refactor','test','review','security_fix','performance_improvement')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    complexity TEXT NOT NULL DEFAULT 'medium' CHECK(complexity IN ('trivial','low','medium','high','very_high')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready_for_review','approved','in_planning','in_progress','in_validation','waiting_human_review','done','blocked','failed','cancelled')),
    repo_override_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    active_repo_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    branch_name TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO work_items (
    id,
    product_id,
    module_id,
    capability_id,
    source_node_id,
    source_node_type,
    parent_work_item_id,
    title,
    problem_statement,
    description,
    acceptance_criteria,
    constraints,
    work_item_type,
    priority,
    complexity,
    status,
    repo_override_id,
    active_repo_id,
    branch_name,
    sort_order,
    created_at,
    updated_at
)
SELECT
    id,
    product_id,
    module_id,
    capability_id,
    source_node_id,
    source_node_type,
    parent_work_item_id,
    title,
    problem_statement,
    description,
    acceptance_criteria,
    constraints,
    CASE WHEN work_item_type = 'feature' THEN 'story' ELSE work_item_type END,
    priority,
    complexity,
    status,
    repo_override_id,
    active_repo_id,
    branch_name,
    sort_order,
    created_at,
    updated_at
FROM work_items_legacy_story_type;

DROP TABLE work_items_legacy_story_type;

CREATE INDEX IF NOT EXISTS idx_work_items_product ON work_items(product_id);
CREATE INDEX IF NOT EXISTS idx_work_items_module ON work_items(module_id);
CREATE INDEX IF NOT EXISTS idx_work_items_capability ON work_items(capability_id);
CREATE INDEX IF NOT EXISTS idx_work_items_parent ON work_items(parent_work_item_id);
CREATE INDEX IF NOT EXISTS idx_work_items_status ON work_items(status);
CREATE INDEX IF NOT EXISTS idx_work_items_source_node ON work_items(source_node_type, source_node_id);
CREATE INDEX IF NOT EXISTS idx_work_items_list_all ON work_items(sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_product_list ON work_items(product_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_module_list ON work_items(module_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_capability_list ON work_items(capability_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_source_list ON work_items(source_node_type, source_node_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_status_list ON work_items(status, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_product_status_summary ON work_items(product_id, status);

PRAGMA foreign_keys = ON;
PRAGMA legacy_alter_table = OFF;

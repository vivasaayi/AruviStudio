CREATE TABLE ci_target_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    auto_preview INTEGER NOT NULL DEFAULT 1 CHECK(auto_preview IN (0,1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(product_id, repository_id)
);

CREATE TABLE ci_dispatches (
    id TEXT PRIMARY KEY NOT NULL,
    binding_id TEXT NOT NULL REFERENCES ci_target_bindings(id) ON DELETE CASCADE,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    commit_sha TEXT NOT NULL,
    ci_run_id INTEGER,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','queued','failed')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(binding_id, commit_sha)
);

CREATE TABLE ci_feedback (
    id TEXT PRIMARY KEY NOT NULL,
    ci_run_id INTEGER NOT NULL,
    commit_sha TEXT NOT NULL,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    verdict TEXT NOT NULL CHECK(verdict IN ('works','needs_changes','blocked')),
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_ci_bindings_product ON ci_target_bindings(product_id);
CREATE INDEX idx_ci_dispatches_status ON ci_dispatches(status, created_at);
CREATE INDEX idx_ci_feedback_run ON ci_feedback(ci_run_id, created_at);

-- A product/repository may approve more than one build target (for example a
-- Preview target and both release architectures). Preserve existing bindings
-- while changing the natural key from product+repository to product+repository+target.
-- ci_dispatches is copied out temporarily because it has a foreign key to the
-- bindings table and SQLite cannot drop a referenced table while FK checks are on.
CREATE TABLE ci_dispatches_backup AS SELECT * FROM ci_dispatches;
DROP TABLE ci_dispatches;

CREATE TABLE ci_target_bindings_next (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    auto_preview INTEGER NOT NULL DEFAULT 0 CHECK(auto_preview IN (0,1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(product_id, repository_id, target_id)
);

INSERT INTO ci_target_bindings_next
    (id, product_id, repository_id, target_id, auto_preview, created_at, updated_at)
SELECT id, product_id, repository_id, target_id,
       CASE WHEN target_id LIKE '%-preview' THEN auto_preview ELSE 0 END,
       created_at, updated_at
FROM ci_target_bindings;

DROP TABLE ci_target_bindings;
ALTER TABLE ci_target_bindings_next RENAME TO ci_target_bindings;

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
INSERT INTO ci_dispatches
    (id, binding_id, work_item_id, workflow_run_id, commit_sha, ci_run_id, status, error_message, created_at, updated_at)
SELECT id, binding_id, work_item_id, workflow_run_id, commit_sha, ci_run_id, status, error_message, created_at, updated_at
FROM ci_dispatches_backup;
DROP TABLE ci_dispatches_backup;

CREATE INDEX idx_ci_bindings_product ON ci_target_bindings(product_id);
CREATE INDEX idx_ci_bindings_product_repository ON ci_target_bindings(product_id, repository_id);
CREATE INDEX idx_ci_dispatches_status ON ci_dispatches(status, created_at);

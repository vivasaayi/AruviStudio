CREATE TABLE IF NOT EXISTS agent_work_runs (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    repository_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    roadmap_hash TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','paused','completed','blocked','cancelled')),
    last_commit_sha TEXT,
    current_batch_id TEXT,
    next_action TEXT NOT NULL DEFAULT '',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_agent_work_runs_status ON agent_work_runs(status);
CREATE INDEX IF NOT EXISTS idx_agent_work_runs_product ON agent_work_runs(product_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_runs_repository ON agent_work_runs(repository_id);

CREATE TABLE IF NOT EXISTS agent_work_items (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    feature_id TEXT NOT NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    module TEXT NOT NULL DEFAULT '',
    service_or_domain TEXT,
    priority TEXT,
    release_phase TEXT,
    title TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','claimed','in_progress','implemented','tests_passed','committed','blocked','skipped','cancelled')),
    batch_id TEXT,
    agent TEXT,
    commit_sha TEXT,
    claim_token TEXT,
    lease_expires_at TEXT,
    heartbeat_at TEXT,
    conflict_zones_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(run_id, feature_id)
);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_run_status ON agent_work_items(run_id, status);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_batch ON agent_work_items(batch_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_agent ON agent_work_items(agent);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_commit ON agent_work_items(commit_sha);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_work_item ON agent_work_items(work_item_id);

CREATE TABLE IF NOT EXISTS agent_work_batches (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'claimed' CHECK(status IN ('claimed','in_progress','implemented','tests_passed','committed','blocked','skipped','cancelled')),
    selection_rule TEXT,
    agent TEXT,
    commit_sha TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_agent_work_batches_run_status ON agent_work_batches(run_id, status);
CREATE INDEX IF NOT EXISTS idx_agent_work_batches_agent ON agent_work_batches(agent);

CREATE TABLE IF NOT EXISTS agent_work_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    ts TEXT NOT NULL DEFAULT (datetime('now')),
    event_type TEXT NOT NULL,
    batch_id TEXT,
    feature_id TEXT,
    work_item_id TEXT,
    agent TEXT,
    command TEXT,
    status TEXT,
    details TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_agent_work_events_run_id ON agent_work_events(run_id, id);
CREATE INDEX IF NOT EXISTS idx_agent_work_events_batch ON agent_work_events(batch_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_events_feature ON agent_work_events(run_id, feature_id);

CREATE TABLE IF NOT EXISTS agent_work_locks (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    zone_key TEXT NOT NULL,
    batch_id TEXT,
    feature_id TEXT,
    agent TEXT NOT NULL,
    claim_token TEXT NOT NULL,
    lease_expires_at TEXT NOT NULL,
    released_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_work_locks_active_zone
    ON agent_work_locks(run_id, zone_key)
    WHERE released_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_agent_work_locks_token ON agent_work_locks(claim_token);
CREATE INDEX IF NOT EXISTS idx_agent_work_locks_expiry ON agent_work_locks(run_id, lease_expires_at);

CREATE TABLE IF NOT EXISTS agent_work_dependencies (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    feature_id TEXT NOT NULL,
    depends_on_feature_id TEXT NOT NULL,
    dependency_kind TEXT NOT NULL DEFAULT 'blocks',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(run_id, feature_id, depends_on_feature_id)
);
CREATE INDEX IF NOT EXISTS idx_agent_work_dependencies_feature
    ON agent_work_dependencies(run_id, feature_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_dependencies_prereq
    ON agent_work_dependencies(run_id, depends_on_feature_id);

CREATE TABLE IF NOT EXISTS agent_work_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    batch_id TEXT,
    feature_id TEXT,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    agent TEXT,
    evidence_type TEXT NOT NULL,
    command TEXT,
    exit_code INTEGER,
    status TEXT,
    summary TEXT NOT NULL DEFAULT '',
    details TEXT NOT NULL DEFAULT '',
    changed_files_json TEXT NOT NULL DEFAULT '[]',
    artifact_refs_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_run
    ON agent_work_evidence(run_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_feature
    ON agent_work_evidence(run_id, feature_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_batch
    ON agent_work_evidence(batch_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_agent
    ON agent_work_evidence(agent);

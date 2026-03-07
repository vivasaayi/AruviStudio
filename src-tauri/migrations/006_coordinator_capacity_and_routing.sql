ALTER TABLE agent_teams ADD COLUMN max_concurrent_workflows INTEGER NOT NULL DEFAULT 2;

CREATE TABLE IF NOT EXISTS workflow_stage_policies (
    id TEXT PRIMARY KEY NOT NULL,
    stage_name TEXT NOT NULL UNIQUE,
    primary_roles TEXT NOT NULL DEFAULT '[]',
    fallback_roles TEXT NOT NULL DEFAULT '[]',
    coordinator_required INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

ALTER TABLE workflow_runs ADD COLUMN pending_stage_name TEXT;

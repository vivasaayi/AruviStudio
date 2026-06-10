CREATE TABLE IF NOT EXISTS external_cli_runs (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    label TEXT NOT NULL,
    command TEXT NOT NULL,
    args_json TEXT NOT NULL DEFAULT '[]',
    prompt TEXT NOT NULL DEFAULT '',
    cwd TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','completed','failed','cancelled')),
    exit_code INTEGER,
    duration_ms INTEGER,
    stdout_chars INTEGER NOT NULL DEFAULT 0,
    stderr_chars INTEGER NOT NULL DEFAULT 0,
    output_artifact_id TEXT REFERENCES artifacts(id) ON DELETE SET NULL,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_external_cli_runs_work_item ON external_cli_runs(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_provider ON external_cli_runs(provider);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_status ON external_cli_runs(status);

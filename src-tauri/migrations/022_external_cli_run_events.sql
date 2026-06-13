CREATE TABLE IF NOT EXISTS external_cli_run_events (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES external_cli_runs(id) ON DELETE CASCADE,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    stream TEXT NOT NULL CHECK(stream IN ('lifecycle','stdout','stderr','error')),
    message TEXT NOT NULL DEFAULT '',
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_run_id ON external_cli_run_events(run_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_work_item ON external_cli_run_events(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_sequence ON external_cli_run_events(run_id, sequence);

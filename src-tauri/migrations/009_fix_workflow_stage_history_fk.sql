PRAGMA foreign_keys = OFF;

ALTER TABLE workflow_stage_history RENAME TO workflow_stage_history_legacy;

CREATE TABLE workflow_stage_history (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    from_stage TEXT NOT NULL,
    to_stage TEXT NOT NULL,
    trigger TEXT NOT NULL DEFAULT 'automatic',
    notes TEXT NOT NULL DEFAULT '',
    transitioned_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO workflow_stage_history (
    id,
    workflow_run_id,
    from_stage,
    to_stage,
    trigger,
    notes,
    transitioned_at
)
SELECT
    id,
    workflow_run_id,
    from_stage,
    to_stage,
    trigger,
    notes,
    transitioned_at
FROM workflow_stage_history_legacy;

DROP TABLE workflow_stage_history_legacy;

CREATE INDEX idx_stage_history_workflow ON workflow_stage_history(workflow_run_id);

PRAGMA foreign_keys = ON;

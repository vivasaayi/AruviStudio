PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

CREATE TABLE IF NOT EXISTS work_items_legacy_task_type (
    id TEXT PRIMARY KEY
);

INSERT OR IGNORE INTO work_items_legacy_task_type (id)
SELECT id FROM work_items;

ALTER TABLE workflow_runs RENAME TO workflow_runs_repair_legacy;
ALTER TABLE agent_runs RENAME TO agent_runs_repair_legacy;
ALTER TABLE approvals RENAME TO approvals_repair_legacy;
ALTER TABLE artifacts RENAME TO artifacts_repair_legacy;
ALTER TABLE findings RENAME TO findings_repair_legacy;
ALTER TABLE agent_model_calls RENAME TO agent_model_calls_repair_legacy;
ALTER TABLE model_calls RENAME TO model_calls_repair_legacy;
ALTER TABLE external_cli_runs RENAME TO external_cli_runs_repair_legacy;
ALTER TABLE external_cli_run_events RENAME TO external_cli_run_events_repair_legacy;
ALTER TABLE delivery_items RENAME TO delivery_items_repair_legacy;

CREATE TABLE workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    workflow_version TEXT NOT NULL DEFAULT '1.0',
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','paused','completed','failed','cancelled')),
    current_stage TEXT NOT NULL DEFAULT 'draft',
    assigned_team_id TEXT REFERENCES agent_teams(id) ON DELETE SET NULL,
    coordinator_agent_id TEXT REFERENCES agent_definitions(id) ON DELETE SET NULL,
    pending_stage_name TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE agent_runs (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    agent_id TEXT NOT NULL REFERENCES agent_definitions(id),
    model_id TEXT REFERENCES model_definitions(id) ON DELETE SET NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed','cancelled')),
    prompt_snapshot_path TEXT,
    output_snapshot_path TEXT,
    token_count_input INTEGER,
    token_count_output INTEGER,
    duration_ms INTEGER,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE approvals (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    approval_type TEXT NOT NULL CHECK(approval_type IN ('task_approval','plan_approval','test_review')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    notes TEXT NOT NULL DEFAULT '',
    acted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    artifact_type TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    size_bytes INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE findings (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    source_agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK(category IN ('security','performance')),
    severity TEXT NOT NULL DEFAULT 'medium' CHECK(severity IN ('critical','high','medium','low','info')),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','resolved','wont_fix','deferred')),
    is_blocking INTEGER NOT NULL DEFAULT 0,
    linked_followup_work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE agent_model_calls (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    agent_run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    agent_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_name TEXT NOT NULL DEFAULT '',
    provider_type TEXT NOT NULL DEFAULT '',
    provider_base_url TEXT NOT NULL DEFAULT '',
    model_id TEXT NOT NULL,
    model_name TEXT NOT NULL,
    call_index INTEGER NOT NULL,
    prompt_chars INTEGER NOT NULL DEFAULT 0,
    response_chars INTEGER NOT NULL DEFAULT 0,
    max_tokens INTEGER,
    token_count_input INTEGER,
    token_count_output INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL DEFAULT 'completed' CHECK(status IN ('completed','failed')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE model_calls (
    id TEXT PRIMARY KEY NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_label TEXT NOT NULL DEFAULT '',
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    session_id TEXT,
    agent_id TEXT,
    stage TEXT,
    provider_id TEXT NOT NULL,
    provider_name TEXT NOT NULL DEFAULT '',
    provider_type TEXT NOT NULL DEFAULT '',
    provider_base_url TEXT NOT NULL DEFAULT '',
    model_id TEXT,
    model_name TEXT NOT NULL,
    call_index INTEGER NOT NULL,
    request_message_count INTEGER NOT NULL DEFAULT 0,
    prompt_chars INTEGER NOT NULL DEFAULT 0,
    response_chars INTEGER NOT NULL DEFAULT 0,
    max_tokens INTEGER,
    temperature REAL,
    token_count_input INTEGER,
    token_count_output INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL DEFAULT 'completed' CHECK(status IN ('completed','failed')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    request_messages_json TEXT,
    response_text TEXT,
    request_snapshot_path TEXT,
    response_snapshot_path TEXT
);

CREATE TABLE external_cli_runs (
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
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    session_log_path TEXT NOT NULL DEFAULT ''
);

CREATE TABLE external_cli_run_events (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES external_cli_runs(id) ON DELETE CASCADE,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    stream TEXT NOT NULL CHECK(stream IN ('lifecycle','stdout','stderr','error')),
    message TEXT NOT NULL DEFAULT '',
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE delivery_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    capability_slice_id TEXT REFERENCES capability_slices(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    delivery_kind TEXT NOT NULL DEFAULT 'implementation' CHECK(delivery_kind IN ('implementation','test','review','documentation','release','other')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready','in_progress','review','done','blocked','cancelled')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO workflow_runs (
    id, work_item_id, workflow_version, status, current_stage, assigned_team_id,
    coordinator_agent_id, pending_stage_name, retry_count, max_retries, error_message,
    started_at, ended_at, updated_at
)
SELECT
    id, work_item_id, workflow_version, status, current_stage, assigned_team_id,
    coordinator_agent_id, pending_stage_name, retry_count, max_retries, error_message,
    started_at, ended_at, updated_at
FROM workflow_runs_repair_legacy;

INSERT INTO agent_runs (
    id, workflow_run_id, work_item_id, agent_id, model_id, stage, status,
    prompt_snapshot_path, output_snapshot_path, token_count_input, token_count_output,
    duration_ms, error_message, started_at, ended_at, created_at
)
SELECT
    id, workflow_run_id, work_item_id, agent_id, model_id, stage, status,
    prompt_snapshot_path, output_snapshot_path, token_count_input, token_count_output,
    duration_ms, error_message, started_at, ended_at, created_at
FROM agent_runs_repair_legacy;

INSERT INTO approvals (
    id, work_item_id, workflow_run_id, approval_type, status, notes, acted_at, created_at
)
SELECT
    id, work_item_id, workflow_run_id, approval_type, status, notes, acted_at, created_at
FROM approvals_repair_legacy;

INSERT INTO artifacts (
    id, work_item_id, workflow_run_id, agent_run_id, artifact_type, storage_path,
    summary, content_type, size_bytes, created_at
)
SELECT
    id, work_item_id, workflow_run_id, agent_run_id, artifact_type, storage_path,
    summary, content_type, size_bytes, created_at
FROM artifacts_repair_legacy;

INSERT INTO findings (
    id, work_item_id, source_agent_run_id, category, severity, title, description,
    status, is_blocking, linked_followup_work_item_id, created_at, updated_at
)
SELECT
    id, work_item_id, source_agent_run_id, category, severity, title, description,
    status, is_blocking, linked_followup_work_item_id, created_at, updated_at
FROM findings_repair_legacy;

INSERT INTO agent_model_calls (
    id, workflow_run_id, agent_run_id, work_item_id, agent_id, stage, provider_id,
    provider_name, provider_type, provider_base_url, model_id, model_name, call_index,
    prompt_chars, response_chars, max_tokens, token_count_input, token_count_output,
    duration_ms, status, error_message, created_at
)
SELECT
    id, workflow_run_id, agent_run_id, work_item_id, agent_id, stage, provider_id,
    provider_name, provider_type, provider_base_url, model_id, model_name, call_index,
    prompt_chars, response_chars, max_tokens, token_count_input, token_count_output,
    duration_ms, status, error_message, created_at
FROM agent_model_calls_repair_legacy;

INSERT INTO model_calls (
    id, source_kind, source_id, source_label, workflow_run_id, agent_run_id,
    work_item_id, product_id, session_id, agent_id, stage, provider_id, provider_name,
    provider_type, provider_base_url, model_id, model_name, call_index,
    request_message_count, prompt_chars, response_chars, max_tokens, temperature,
    token_count_input, token_count_output, duration_ms, status, error_message,
    created_at, request_messages_json, response_text, request_snapshot_path,
    response_snapshot_path
)
SELECT
    id, source_kind, source_id, source_label, workflow_run_id, agent_run_id,
    work_item_id, product_id, session_id, agent_id, stage, provider_id, provider_name,
    provider_type, provider_base_url, model_id, model_name, call_index,
    request_message_count, prompt_chars, response_chars, max_tokens, temperature,
    token_count_input, token_count_output, duration_ms, status, error_message,
    created_at, request_messages_json, response_text, request_snapshot_path,
    response_snapshot_path
FROM model_calls_repair_legacy;

INSERT INTO external_cli_runs (
    id, work_item_id, provider, label, command, args_json, prompt, cwd, status,
    exit_code, duration_ms, stdout_chars, stderr_chars, output_artifact_id,
    error_message, started_at, ended_at, created_at, session_log_path
)
SELECT
    id, work_item_id, provider, label, command, args_json, prompt, cwd, status,
    exit_code, duration_ms, stdout_chars, stderr_chars, output_artifact_id,
    error_message, started_at, ended_at, created_at, session_log_path
FROM external_cli_runs_repair_legacy;

INSERT INTO external_cli_run_events (
    id, run_id, work_item_id, stream, message, sequence, created_at
)
SELECT
    id, run_id, work_item_id, stream, message, sequence, created_at
FROM external_cli_run_events_repair_legacy;

INSERT INTO delivery_items (
    id, product_id, capability_id, capability_slice_id, work_item_id, title,
    description, delivery_kind, status, sort_order, created_at, updated_at
)
SELECT
    id, product_id, capability_id, capability_slice_id, work_item_id, title,
    description, delivery_kind, status, sort_order, created_at, updated_at
FROM delivery_items_repair_legacy;

DROP TABLE approvals_repair_legacy;
DROP TABLE findings_repair_legacy;
DROP TABLE agent_model_calls_repair_legacy;
DROP TABLE model_calls_repair_legacy;
DROP TABLE external_cli_run_events_repair_legacy;
DROP TABLE external_cli_runs_repair_legacy;
DROP TABLE artifacts_repair_legacy;
DROP TABLE delivery_items_repair_legacy;
DROP TABLE agent_runs_repair_legacy;
DROP TABLE workflow_runs_repair_legacy;
DROP TABLE work_items_legacy_task_type;

CREATE INDEX IF NOT EXISTS idx_workflow_runs_work_item ON workflow_runs(work_item_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_workflow ON agent_runs(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_approvals_work_item ON approvals(work_item_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_work_item ON artifacts(work_item_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_workflow ON artifacts(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_findings_work_item ON findings(work_item_id);
CREATE INDEX IF NOT EXISTS idx_agent_model_calls_workflow ON agent_model_calls(workflow_run_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run ON agent_model_calls(agent_run_id, call_index);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run_call_index ON agent_model_calls(agent_run_id, call_index);
CREATE INDEX IF NOT EXISTS idx_model_calls_created ON model_calls(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_model_calls_source ON model_calls(source_kind, source_id, call_index);
CREATE INDEX IF NOT EXISTS idx_model_calls_workflow ON model_calls(workflow_run_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_model_calls_agent_run ON model_calls(agent_run_id, call_index);
CREATE UNIQUE INDEX IF NOT EXISTS idx_model_calls_source_call_index ON model_calls(source_kind, COALESCE(source_id, ''), call_index);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_work_item ON external_cli_runs(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_provider ON external_cli_runs(provider);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_status ON external_cli_runs(status);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_run_id ON external_cli_run_events(run_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_work_item ON external_cli_run_events(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_sequence ON external_cli_run_events(run_id, sequence);
CREATE INDEX IF NOT EXISTS idx_delivery_items_product ON delivery_items(product_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_capability ON delivery_items(capability_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_slice ON delivery_items(capability_slice_id);

PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;

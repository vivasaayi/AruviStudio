CREATE TABLE IF NOT EXISTS model_calls (
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
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_model_calls_created
    ON model_calls(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_model_calls_source
    ON model_calls(source_kind, source_id, call_index);

CREATE INDEX IF NOT EXISTS idx_model_calls_workflow
    ON model_calls(workflow_run_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_model_calls_agent_run
    ON model_calls(agent_run_id, call_index);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_calls_source_call_index
    ON model_calls(source_kind, COALESCE(source_id, ''), call_index);

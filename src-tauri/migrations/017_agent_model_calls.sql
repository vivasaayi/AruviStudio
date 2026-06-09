CREATE TABLE IF NOT EXISTS agent_model_calls (
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

CREATE INDEX IF NOT EXISTS idx_agent_model_calls_workflow
    ON agent_model_calls(workflow_run_id, created_at);

CREATE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run
    ON agent_model_calls(agent_run_id, call_index);

CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run_call_index
    ON agent_model_calls(agent_run_id, call_index);

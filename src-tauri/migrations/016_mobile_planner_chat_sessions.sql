CREATE TABLE IF NOT EXISTS mobile_planner_chat_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    provider_id TEXT NULL,
    model_name TEXT NULL,
    active_product_id TEXT NULL REFERENCES products(id) ON DELETE SET NULL,
    active_product_name TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS mobile_planner_chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES mobile_planner_chat_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_mobile_planner_chat_messages_session
ON mobile_planner_chat_messages(session_id, created_at);

CREATE TABLE IF NOT EXISTS mobile_planner_chat_tool_traces (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES mobile_planner_chat_sessions(id) ON DELETE CASCADE,
    step INTEGER NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    result_json TEXT NULL,
    error TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_mobile_planner_chat_tool_traces_session
ON mobile_planner_chat_tool_traces(session_id, created_at);

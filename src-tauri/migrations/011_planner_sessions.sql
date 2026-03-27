CREATE TABLE IF NOT EXISTS planner_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    provider_id TEXT NULL,
    model_name TEXT NULL,
    pending_plan_json TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS planner_conversation_entries (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES planner_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_planner_conversation_session
ON planner_conversation_entries(session_id, created_at);

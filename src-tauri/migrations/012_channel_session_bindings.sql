CREATE TABLE IF NOT EXISTS planner_channel_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    channel TEXT NOT NULL,
    remote_user_id TEXT NOT NULL,
    remote_conversation_id TEXT NOT NULL,
    planner_session_id TEXT NOT NULL REFERENCES planner_sessions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_planner_channel_binding_unique
ON planner_channel_bindings(channel, remote_conversation_id);

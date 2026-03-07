ALTER TABLE agent_definitions ADD COLUMN skill_tags TEXT NOT NULL DEFAULT '[]';
ALTER TABLE agent_definitions ADD COLUMN employment_status TEXT NOT NULL DEFAULT 'active' CHECK(employment_status IN ('active','inactive','terminated'));

CREATE TABLE IF NOT EXISTS agent_teams (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    department TEXT NOT NULL DEFAULT 'engineering',
    description TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_team_memberships (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agent_definitions(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT '',
    is_lead INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, agent_id)
);
CREATE INDEX IF NOT EXISTS idx_agent_team_memberships_team ON agent_team_memberships(team_id);
CREATE INDEX IF NOT EXISTS idx_agent_team_memberships_agent ON agent_team_memberships(agent_id);

CREATE TABLE IF NOT EXISTS team_assignments (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('module','feature')),
    scope_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, scope_type, scope_id)
);
CREATE INDEX IF NOT EXISTS idx_team_assignments_team ON team_assignments(team_id);
CREATE INDEX IF NOT EXISTS idx_team_assignments_scope ON team_assignments(scope_type, scope_id);

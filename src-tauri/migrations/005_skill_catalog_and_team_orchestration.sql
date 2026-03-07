CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
    description TEXT NOT NULL DEFAULT '',
    instructions TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_skills_category ON skills(category);

CREATE TABLE IF NOT EXISTS agent_skill_links (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agent_definitions(id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    proficiency TEXT NOT NULL DEFAULT 'working' CHECK(proficiency IN ('learning', 'working', 'expert')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(agent_id, skill_id)
);
CREATE INDEX idx_agent_skill_links_agent ON agent_skill_links(agent_id);
CREATE INDEX idx_agent_skill_links_skill ON agent_skill_links(skill_id);

CREATE TABLE IF NOT EXISTS team_skill_links (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, skill_id)
);
CREATE INDEX idx_team_skill_links_team ON team_skill_links(team_id);
CREATE INDEX idx_team_skill_links_skill ON team_skill_links(skill_id);

ALTER TABLE workflow_runs ADD COLUMN assigned_team_id TEXT;
ALTER TABLE workflow_runs ADD COLUMN coordinator_agent_id TEXT;

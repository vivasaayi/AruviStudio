PRAGMA foreign_keys = OFF;

ALTER TABLE team_assignments RENAME TO team_assignments_legacy;

CREATE TABLE team_assignments (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('product','module','capability')),
    scope_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, scope_type, scope_id)
);

INSERT INTO team_assignments (id, team_id, scope_type, scope_id, created_at)
SELECT
    id,
    team_id,
    CASE
        WHEN scope_type = 'feature' THEN 'capability'
        ELSE scope_type
    END,
    scope_id,
    created_at
FROM team_assignments_legacy;

DROP TABLE team_assignments_legacy;

CREATE INDEX idx_team_assignments_team ON team_assignments(team_id);
CREATE INDEX idx_team_assignments_scope ON team_assignments(scope_type, scope_id);

PRAGMA foreign_keys = ON;

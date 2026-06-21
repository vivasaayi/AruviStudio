-- Canonical externally visible scope type for product areas.

PRAGMA foreign_keys=off;

ALTER TABLE repository_attachments RENAME TO repository_attachments_legacy;

CREATE TABLE repository_attachments (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('product','product_area')),
    scope_id TEXT NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO repository_attachments (
    id,
    scope_type,
    scope_id,
    repository_id,
    is_default,
    created_at
)
SELECT
    id,
    CASE lower(replace(scope_type, '-', '_'))
        WHEN 'module' THEN 'product_area'
        WHEN 'area' THEN 'product_area'
        WHEN 'productarea' THEN 'product_area'
        WHEN 'product_area' THEN 'product_area'
        ELSE 'product'
    END,
    scope_id,
    repository_id,
    is_default,
    created_at
FROM repository_attachments_legacy;

DROP TABLE repository_attachments_legacy;
CREATE INDEX IF NOT EXISTS idx_repo_attachments_scope ON repository_attachments(scope_type, scope_id);

ALTER TABLE team_assignments RENAME TO team_assignments_legacy;

CREATE TABLE team_assignments (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('product','product_area','capability')),
    scope_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, scope_type, scope_id)
);

INSERT INTO team_assignments (
    id,
    team_id,
    scope_type,
    scope_id,
    created_at
)
SELECT
    id,
    team_id,
    CASE lower(replace(scope_type, '-', '_'))
        WHEN 'module' THEN 'product_area'
        WHEN 'area' THEN 'product_area'
        WHEN 'productarea' THEN 'product_area'
        WHEN 'product_area' THEN 'product_area'
        WHEN 'feature' THEN 'capability'
        WHEN 'capability' THEN 'capability'
        ELSE 'product'
    END,
    scope_id,
    created_at
FROM team_assignments_legacy;

DROP TABLE team_assignments_legacy;
CREATE INDEX IF NOT EXISTS idx_team_assignments_team ON team_assignments(team_id);
CREATE INDEX IF NOT EXISTS idx_team_assignments_scope ON team_assignments(scope_type, scope_id);

PRAGMA foreign_keys=on;

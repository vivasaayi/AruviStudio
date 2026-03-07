PRAGMA foreign_keys = OFF;

ALTER TABLE features RENAME TO features_legacy;

CREATE TABLE capabilities (
    id TEXT PRIMARY KEY NOT NULL,
    module_id TEXT NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    parent_capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    level INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    risk TEXT NOT NULL DEFAULT 'low' CHECK(risk IN ('high','medium','low')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','in_progress','done','archived')),
    technical_notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO capabilities (
    id,
    module_id,
    parent_capability_id,
    level,
    sort_order,
    name,
    description,
    acceptance_criteria,
    priority,
    risk,
    status,
    technical_notes,
    created_at,
    updated_at
)
SELECT
    id,
    module_id,
    parent_feature_id,
    level,
    sort_order,
    name,
    description,
    acceptance_criteria,
    priority,
    risk,
    status,
    technical_notes,
    created_at,
    updated_at
FROM features_legacy;

DROP TABLE features_legacy;

CREATE INDEX idx_capabilities_module ON capabilities(module_id);
CREATE INDEX idx_capabilities_parent ON capabilities(parent_capability_id);

ALTER TABLE tasks RENAME TO tasks_legacy;

CREATE TABLE work_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    module_id TEXT REFERENCES modules(id) ON DELETE SET NULL,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    parent_work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    problem_statement TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    constraints TEXT NOT NULL DEFAULT '',
    work_item_type TEXT NOT NULL DEFAULT 'feature' CHECK(work_item_type IN ('feature','bug','refactor','test','review','security_fix','performance_improvement')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    complexity TEXT NOT NULL DEFAULT 'medium' CHECK(complexity IN ('trivial','low','medium','high','very_high')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready_for_review','approved','in_planning','in_progress','in_validation','waiting_human_review','done','blocked','failed','cancelled')),
    repo_override_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    active_repo_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    branch_name TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO work_items (
    id,
    product_id,
    module_id,
    capability_id,
    parent_work_item_id,
    title,
    problem_statement,
    description,
    acceptance_criteria,
    constraints,
    work_item_type,
    priority,
    complexity,
    status,
    repo_override_id,
    active_repo_id,
    branch_name,
    sort_order,
    created_at,
    updated_at
)
SELECT
    id,
    product_id,
    module_id,
    feature_id,
    parent_task_id,
    title,
    problem_statement,
    description,
    acceptance_criteria,
    constraints,
    task_type,
    priority,
    complexity,
    status,
    repo_override_id,
    active_repo_id,
    branch_name,
    sort_order,
    created_at,
    updated_at
FROM tasks_legacy;

DROP TABLE tasks_legacy;

CREATE INDEX idx_work_items_product ON work_items(product_id);
CREATE INDEX idx_work_items_module ON work_items(module_id);
CREATE INDEX idx_work_items_capability ON work_items(capability_id);
CREATE INDEX idx_work_items_parent ON work_items(parent_work_item_id);
CREATE INDEX idx_work_items_status ON work_items(status);

ALTER TABLE workflow_runs RENAME TO workflow_runs_legacy;

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

INSERT INTO workflow_runs (
    id,
    work_item_id,
    workflow_version,
    status,
    current_stage,
    assigned_team_id,
    coordinator_agent_id,
    pending_stage_name,
    retry_count,
    max_retries,
    error_message,
    started_at,
    ended_at,
    updated_at
)
SELECT
    id,
    task_id,
    workflow_version,
    status,
    current_stage,
    assigned_team_id,
    coordinator_agent_id,
    pending_stage_name,
    retry_count,
    max_retries,
    error_message,
    started_at,
    ended_at,
    updated_at
FROM workflow_runs_legacy;

DROP TABLE workflow_runs_legacy;

CREATE INDEX idx_workflow_runs_work_item ON workflow_runs(work_item_id);

ALTER TABLE agent_runs RENAME TO agent_runs_legacy;

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

INSERT INTO agent_runs (
    id,
    workflow_run_id,
    work_item_id,
    agent_id,
    model_id,
    stage,
    status,
    prompt_snapshot_path,
    output_snapshot_path,
    token_count_input,
    token_count_output,
    duration_ms,
    error_message,
    started_at,
    ended_at,
    created_at
)
SELECT
    id,
    workflow_run_id,
    NULL,
    agent_id,
    NULL,
    stage,
    status,
    prompt_snapshot_path,
    output_snapshot_path,
    token_count_input,
    token_count_output,
    duration_ms,
    error_message,
    started_at,
    ended_at,
    created_at
FROM agent_runs_legacy;

DROP TABLE agent_runs_legacy;

CREATE INDEX idx_agent_runs_workflow ON agent_runs(workflow_run_id);
CREATE INDEX idx_agent_runs_work_item ON agent_runs(work_item_id);

ALTER TABLE approvals RENAME TO approvals_legacy;

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

INSERT INTO approvals (
    id,
    work_item_id,
    workflow_run_id,
    approval_type,
    status,
    notes,
    acted_at,
    created_at
)
SELECT
    id,
    task_id,
    workflow_run_id,
    approval_type,
    status,
    notes,
    acted_at,
    created_at
FROM approvals_legacy;

DROP TABLE approvals_legacy;

CREATE INDEX idx_approvals_work_item ON approvals(work_item_id);

ALTER TABLE artifacts RENAME TO artifacts_legacy;

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

INSERT INTO artifacts (
    id,
    work_item_id,
    workflow_run_id,
    agent_run_id,
    artifact_type,
    storage_path,
    summary,
    content_type,
    size_bytes,
    created_at
)
SELECT
    id,
    task_id,
    workflow_run_id,
    agent_run_id,
    artifact_type,
    storage_path,
    summary,
    content_type,
    size_bytes,
    created_at
FROM artifacts_legacy;

DROP TABLE artifacts_legacy;

CREATE INDEX idx_artifacts_work_item ON artifacts(work_item_id);
CREATE INDEX idx_artifacts_workflow ON artifacts(workflow_run_id);

ALTER TABLE findings RENAME TO findings_legacy;

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

INSERT INTO findings (
    id,
    work_item_id,
    source_agent_run_id,
    category,
    severity,
    title,
    description,
    status,
    is_blocking,
    linked_followup_work_item_id,
    created_at,
    updated_at
)
SELECT
    id,
    task_id,
    source_agent_run_id,
    category,
    severity,
    title,
    description,
    status,
    is_blocking,
    linked_followup_task_id,
    created_at,
    updated_at
FROM findings_legacy;

DROP TABLE findings_legacy;

CREATE INDEX idx_findings_work_item ON findings(work_item_id);

PRAGMA foreign_keys = ON;

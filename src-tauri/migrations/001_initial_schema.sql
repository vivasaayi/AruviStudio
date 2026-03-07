-- AruviStudio initial schema

CREATE TABLE IF NOT EXISTS products (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    vision TEXT NOT NULL DEFAULT '',
    goals TEXT NOT NULL DEFAULT '[]',
    tags TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','archived')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS modules (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    purpose TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_modules_product ON modules(product_id);

CREATE TABLE IF NOT EXISTS features (
    id TEXT PRIMARY KEY NOT NULL,
    module_id TEXT NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    parent_feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,
    level INTEGER NOT NULL DEFAULT 0,
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
CREATE INDEX idx_features_module ON features(module_id);
CREATE INDEX idx_features_parent ON features(parent_feature_id);

CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    local_path TEXT NOT NULL,
    remote_url TEXT NOT NULL DEFAULT '',
    default_branch TEXT NOT NULL DEFAULT 'main',
    auth_profile TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS repository_attachments (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('product','module')),
    scope_id TEXT NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_repo_attachments_scope ON repository_attachments(scope_type, scope_id);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    module_id TEXT REFERENCES modules(id) ON DELETE SET NULL,
    feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,
    parent_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    problem_statement TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    constraints TEXT NOT NULL DEFAULT '',
    task_type TEXT NOT NULL DEFAULT 'feature' CHECK(task_type IN ('feature','bug','refactor','test','review','security_fix','performance_improvement')),
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
CREATE INDEX idx_tasks_product ON tasks(product_id);
CREATE INDEX idx_tasks_module ON tasks(module_id);
CREATE INDEX idx_tasks_feature ON tasks(feature_id);
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
CREATE INDEX idx_tasks_status ON tasks(status);

CREATE TABLE IF NOT EXISTS model_providers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL DEFAULT 'openai_compatible' CHECK(provider_type IN ('openai_compatible','local_runtime')),
    base_url TEXT NOT NULL,
    auth_secret_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS model_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    provider_id TEXT NOT NULL REFERENCES model_providers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    context_window INTEGER,
    capability_tags TEXT NOT NULL DEFAULT '[]',
    notes TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_model_defs_provider ON model_definitions(provider_id);

CREATE TABLE IF NOT EXISTS agent_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    prompt_template_ref TEXT NOT NULL DEFAULT '',
    allowed_tools TEXT NOT NULL DEFAULT '[]',
    boundaries TEXT NOT NULL DEFAULT '{}',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_model_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agent_definitions(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL REFERENCES model_definitions(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_agent_model_agent ON agent_model_bindings(agent_id);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workflow_version TEXT NOT NULL DEFAULT '1.0',
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','paused','completed','failed','cancelled')),
    current_stage TEXT NOT NULL DEFAULT 'draft',
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_workflow_runs_task ON workflow_runs(task_id);

CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agent_definitions(id),
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
CREATE INDEX idx_agent_runs_workflow ON agent_runs(workflow_run_id);

CREATE TABLE IF NOT EXISTS approvals (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    approval_type TEXT NOT NULL CHECK(approval_type IN ('task_approval','plan_approval','test_review')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    notes TEXT NOT NULL DEFAULT '',
    acted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_approvals_task ON approvals(task_id);

CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    artifact_type TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    size_bytes INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_artifacts_task ON artifacts(task_id);
CREATE INDEX idx_artifacts_workflow ON artifacts(workflow_run_id);

CREATE TABLE IF NOT EXISTS findings (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    source_agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK(category IN ('security','performance')),
    severity TEXT NOT NULL DEFAULT 'medium' CHECK(severity IN ('critical','high','medium','low','info')),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','resolved','wont_fix','deferred')),
    is_blocking INTEGER NOT NULL DEFAULT 0,
    linked_followup_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_findings_task ON findings(task_id);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS secret_refs (
    key TEXT PRIMARY KEY NOT NULL,
    encrypted_value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS docker_test_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    image TEXT NOT NULL,
    default_cmd TEXT NOT NULL DEFAULT '',
    env_vars TEXT NOT NULL DEFAULT '{}',
    mount_config TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS prompt_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    template_text TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS structured_logs (
    id TEXT PRIMARY KEY NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    level TEXT NOT NULL,
    target TEXT NOT NULL DEFAULT '',
    message TEXT NOT NULL,
    fields TEXT NOT NULL DEFAULT '{}',
    workflow_run_id TEXT,
    agent_run_id TEXT
);
CREATE INDEX idx_logs_timestamp ON structured_logs(timestamp);
CREATE INDEX idx_logs_workflow ON structured_logs(workflow_run_id);

CREATE TABLE IF NOT EXISTS workflow_stage_history (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    from_stage TEXT NOT NULL,
    to_stage TEXT NOT NULL,
    trigger TEXT NOT NULL DEFAULT 'automatic',
    notes TEXT NOT NULL DEFAULT '',
    transitioned_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_stage_history_workflow ON workflow_stage_history(workflow_run_id);

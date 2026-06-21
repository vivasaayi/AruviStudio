PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

ALTER TABLE workflow_runs RENAME TO workflow_runs_product_area_fk_legacy;
ALTER TABLE agent_runs RENAME TO agent_runs_product_area_fk_legacy;
ALTER TABLE approvals RENAME TO approvals_product_area_fk_legacy;
ALTER TABLE artifacts RENAME TO artifacts_product_area_fk_legacy;
ALTER TABLE findings RENAME TO findings_product_area_fk_legacy;
ALTER TABLE agent_model_calls RENAME TO agent_model_calls_product_area_fk_legacy;
ALTER TABLE model_calls RENAME TO model_calls_product_area_fk_legacy;
ALTER TABLE external_cli_runs RENAME TO external_cli_runs_product_area_fk_legacy;
ALTER TABLE external_cli_run_events RENAME TO external_cli_run_events_product_area_fk_legacy;
ALTER TABLE delivery_items RENAME TO delivery_items_product_area_fk_legacy;
ALTER TABLE agent_work_items RENAME TO agent_work_items_product_area_fk_legacy;
ALTER TABLE agent_work_evidence RENAME TO agent_work_evidence_product_area_fk_legacy;
ALTER TABLE work_items RENAME TO work_items_product_area_fk_legacy;
ALTER TABLE product_dependencies RENAME TO product_dependencies_product_area_fk_legacy;
ALTER TABLE capability_slices RENAME TO capability_slices_product_area_fk_legacy;
ALTER TABLE capabilities RENAME TO capabilities_product_area_fk_legacy;
ALTER TABLE modules RENAME TO modules_product_area_fk_legacy;

CREATE TABLE product_areas (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    purpose TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    node_kind TEXT NOT NULL DEFAULT 'product_area' CHECK(node_kind IN ('product_area')),
    explanation TEXT NOT NULL DEFAULT '',
    examples TEXT NOT NULL DEFAULT '',
    implementation_notes TEXT NOT NULL DEFAULT '',
    test_guidance TEXT NOT NULL DEFAULT ''
);

CREATE TABLE capabilities (
    id TEXT PRIMARY KEY NOT NULL,
    product_area_id TEXT NOT NULL REFERENCES product_areas(id) ON DELETE CASCADE,
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
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    node_kind TEXT NOT NULL DEFAULT 'capability' CHECK(node_kind IN ('capability','feature')),
    explanation TEXT NOT NULL DEFAULT '',
    examples TEXT NOT NULL DEFAULT '',
    implementation_notes TEXT NOT NULL DEFAULT '',
    test_guidance TEXT NOT NULL DEFAULT ''
);

CREATE TABLE product_dependencies (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    depends_on_product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    depends_on_capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    dependency_kind TEXT NOT NULL DEFAULT 'platform' CHECK(dependency_kind IN ('platform','capability','data','integration','operational','other')),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','planned','blocked','retired')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(product_id <> depends_on_product_id),
    UNIQUE(product_id, capability_id, depends_on_product_id, depends_on_capability_id, dependency_kind)
);

CREATE TABLE capability_slices (
    id TEXT PRIMARY KEY NOT NULL,
    capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical','high','medium','low')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','in_progress','done','archived')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE work_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    product_area_id TEXT REFERENCES product_areas(id) ON DELETE SET NULL,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    source_node_id TEXT,
    source_node_type TEXT,
    parent_work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    problem_statement TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    constraints TEXT NOT NULL DEFAULT '',
    work_item_type TEXT NOT NULL DEFAULT 'story' CHECK(work_item_type IN ('story','task','setup','bug','refactor','test','review','security_fix','performance_improvement')),
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

CREATE TABLE agent_model_calls (
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

CREATE TABLE model_calls (
    id TEXT PRIMARY KEY NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_label TEXT NOT NULL DEFAULT '',
    workflow_run_id TEXT REFERENCES workflow_runs(id) ON DELETE SET NULL,
    agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    session_id TEXT,
    agent_id TEXT,
    stage TEXT,
    provider_id TEXT NOT NULL,
    provider_name TEXT NOT NULL DEFAULT '',
    provider_type TEXT NOT NULL DEFAULT '',
    provider_base_url TEXT NOT NULL DEFAULT '',
    model_id TEXT,
    model_name TEXT NOT NULL,
    call_index INTEGER NOT NULL DEFAULT 0,
    request_message_count INTEGER NOT NULL DEFAULT 0,
    prompt_chars INTEGER NOT NULL DEFAULT 0,
    response_chars INTEGER NOT NULL DEFAULT 0,
    max_tokens INTEGER,
    temperature REAL,
    token_count_input INTEGER,
    token_count_output INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL DEFAULT 'completed' CHECK(status IN ('completed','failed')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    request_messages_json TEXT,
    response_text TEXT,
    request_snapshot_path TEXT,
    response_snapshot_path TEXT
);

CREATE TABLE external_cli_runs (
    id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    label TEXT NOT NULL,
    command TEXT NOT NULL,
    args_json TEXT NOT NULL DEFAULT '[]',
    prompt TEXT NOT NULL DEFAULT '',
    cwd TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','completed','failed','cancelled')),
    exit_code INTEGER,
    duration_ms INTEGER,
    stdout_chars INTEGER NOT NULL DEFAULT 0,
    stderr_chars INTEGER NOT NULL DEFAULT 0,
    output_artifact_id TEXT REFERENCES artifacts(id) ON DELETE SET NULL,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    session_log_path TEXT NOT NULL DEFAULT ''
);

CREATE TABLE external_cli_run_events (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES external_cli_runs(id) ON DELETE CASCADE,
    work_item_id TEXT NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    stream TEXT NOT NULL CHECK(stream IN ('lifecycle','stdout','stderr','error')),
    message TEXT NOT NULL DEFAULT '',
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE delivery_items (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,
    capability_slice_id TEXT REFERENCES capability_slices(id) ON DELETE SET NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    delivery_kind TEXT NOT NULL DEFAULT 'implementation' CHECK(delivery_kind IN ('implementation','test','review','documentation','release','other')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready','in_progress','review','done','blocked','cancelled')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE agent_work_items (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    feature_id TEXT NOT NULL,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    product_area TEXT NOT NULL DEFAULT '',
    service_or_domain TEXT,
    priority TEXT,
    release_phase TEXT,
    title TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','claimed','in_progress','implemented','tests_passed','committed','blocked','skipped','cancelled')),
    batch_id TEXT,
    agent TEXT,
    commit_sha TEXT,
    claim_token TEXT,
    lease_expires_at TEXT,
    heartbeat_at TEXT,
    conflict_zones_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(run_id, feature_id)
);

CREATE TABLE agent_work_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES agent_work_runs(id) ON DELETE CASCADE,
    batch_id TEXT,
    feature_id TEXT,
    work_item_id TEXT REFERENCES work_items(id) ON DELETE SET NULL,
    agent TEXT,
    evidence_type TEXT NOT NULL,
    command TEXT,
    exit_code INTEGER,
    status TEXT,
    summary TEXT NOT NULL DEFAULT '',
    details TEXT NOT NULL DEFAULT '',
    changed_files_json TEXT NOT NULL DEFAULT '[]',
    artifact_refs_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO product_areas SELECT * FROM modules_product_area_fk_legacy;
INSERT INTO capabilities (
    id, product_area_id, parent_capability_id, level, sort_order, name, description,
    acceptance_criteria, priority, risk, status, technical_notes, created_at,
    updated_at, node_kind, explanation, examples, implementation_notes, test_guidance
)
SELECT
    id, module_id, parent_capability_id, level, sort_order, name, description,
    acceptance_criteria, priority, risk, status, technical_notes, created_at,
    updated_at, node_kind, explanation, examples, implementation_notes, test_guidance
FROM capabilities_product_area_fk_legacy;
INSERT INTO product_dependencies SELECT * FROM product_dependencies_product_area_fk_legacy;
INSERT INTO capability_slices SELECT * FROM capability_slices_product_area_fk_legacy;
INSERT INTO work_items (
    id, product_id, product_area_id, capability_id, source_node_id, source_node_type,
    parent_work_item_id, title, problem_statement, description, acceptance_criteria,
    constraints, work_item_type, priority, complexity, status, repo_override_id,
    active_repo_id, branch_name, sort_order, created_at, updated_at
)
SELECT
    id, product_id, module_id, capability_id, source_node_id, source_node_type,
    parent_work_item_id, title, problem_statement, description, acceptance_criteria,
    constraints, work_item_type, priority, complexity, status, repo_override_id,
    active_repo_id, branch_name, sort_order, created_at, updated_at
FROM work_items_product_area_fk_legacy;
INSERT INTO workflow_runs SELECT * FROM workflow_runs_product_area_fk_legacy;
INSERT INTO agent_runs SELECT * FROM agent_runs_product_area_fk_legacy;
INSERT INTO approvals SELECT * FROM approvals_product_area_fk_legacy;
INSERT INTO artifacts SELECT * FROM artifacts_product_area_fk_legacy;
INSERT INTO findings SELECT * FROM findings_product_area_fk_legacy;
INSERT INTO agent_model_calls SELECT * FROM agent_model_calls_product_area_fk_legacy;
INSERT INTO model_calls SELECT * FROM model_calls_product_area_fk_legacy;
INSERT INTO external_cli_runs SELECT * FROM external_cli_runs_product_area_fk_legacy;
INSERT INTO external_cli_run_events SELECT * FROM external_cli_run_events_product_area_fk_legacy;
INSERT INTO delivery_items SELECT * FROM delivery_items_product_area_fk_legacy;
INSERT INTO agent_work_items (
    id, run_id, feature_id, work_item_id, product_area, service_or_domain, priority,
    release_phase, title, description, status, batch_id, agent, commit_sha, claim_token,
    lease_expires_at, heartbeat_at, conflict_zones_json, metadata_json, created_at, updated_at
)
SELECT
    id, run_id, feature_id, work_item_id, module, service_or_domain, priority,
    release_phase, title, description, status, batch_id, agent, commit_sha, claim_token,
    lease_expires_at, heartbeat_at, conflict_zones_json, metadata_json, created_at, updated_at
FROM agent_work_items_product_area_fk_legacy;
INSERT INTO agent_work_evidence SELECT * FROM agent_work_evidence_product_area_fk_legacy;

DROP TABLE agent_work_evidence_product_area_fk_legacy;
DROP TABLE agent_work_items_product_area_fk_legacy;
DROP TABLE delivery_items_product_area_fk_legacy;
DROP TABLE external_cli_run_events_product_area_fk_legacy;
DROP TABLE external_cli_runs_product_area_fk_legacy;
DROP TABLE model_calls_product_area_fk_legacy;
DROP TABLE agent_model_calls_product_area_fk_legacy;
DROP TABLE findings_product_area_fk_legacy;
DROP TABLE artifacts_product_area_fk_legacy;
DROP TABLE approvals_product_area_fk_legacy;
DROP TABLE agent_runs_product_area_fk_legacy;
DROP TABLE workflow_runs_product_area_fk_legacy;
DROP TABLE work_items_product_area_fk_legacy;
DROP TABLE capability_slices_product_area_fk_legacy;
DROP TABLE product_dependencies_product_area_fk_legacy;
DROP TABLE capabilities_product_area_fk_legacy;
DROP TABLE modules_product_area_fk_legacy;

CREATE INDEX IF NOT EXISTS idx_product_areas_product ON product_areas(product_id);
CREATE INDEX IF NOT EXISTS idx_product_areas_kind ON product_areas(node_kind);
CREATE INDEX IF NOT EXISTS idx_capabilities_product_area ON capabilities(product_area_id);
CREATE INDEX IF NOT EXISTS idx_capabilities_parent ON capabilities(parent_capability_id);
CREATE INDEX IF NOT EXISTS idx_capabilities_kind ON capabilities(node_kind);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_product ON product_dependencies(product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_capability ON product_dependencies(capability_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on ON product_dependencies(depends_on_product_id);
CREATE INDEX IF NOT EXISTS idx_product_dependencies_depends_on_capability ON product_dependencies(depends_on_capability_id);
CREATE INDEX IF NOT EXISTS idx_capability_slices_capability ON capability_slices(capability_id);
CREATE INDEX IF NOT EXISTS idx_work_items_product ON work_items(product_id);
CREATE INDEX IF NOT EXISTS idx_work_items_product_area ON work_items(product_area_id);
CREATE INDEX IF NOT EXISTS idx_work_items_capability ON work_items(capability_id);
CREATE INDEX IF NOT EXISTS idx_work_items_parent ON work_items(parent_work_item_id);
CREATE INDEX IF NOT EXISTS idx_work_items_status ON work_items(status);
CREATE INDEX IF NOT EXISTS idx_work_items_source_node ON work_items(source_node_type, source_node_id);
CREATE INDEX IF NOT EXISTS idx_work_items_list_all ON work_items(sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_product_list ON work_items(product_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_product_area_list ON work_items(product_area_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_capability_list ON work_items(capability_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_items_source_list ON work_items(source_node_type, source_node_id, sort_order, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_work_item ON workflow_runs(work_item_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_workflow ON agent_runs(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_approvals_work_item ON approvals(work_item_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_work_item ON artifacts(work_item_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_workflow ON artifacts(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_findings_work_item ON findings(work_item_id);
CREATE INDEX IF NOT EXISTS idx_agent_model_calls_workflow ON agent_model_calls(workflow_run_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run ON agent_model_calls(agent_run_id, call_index);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_model_calls_agent_run_call_index ON agent_model_calls(agent_run_id, call_index);
CREATE INDEX IF NOT EXISTS idx_model_calls_created ON model_calls(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_model_calls_source ON model_calls(source_kind, source_id, call_index);
CREATE INDEX IF NOT EXISTS idx_model_calls_workflow ON model_calls(workflow_run_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_model_calls_agent_run ON model_calls(agent_run_id, call_index);
CREATE UNIQUE INDEX IF NOT EXISTS idx_model_calls_source_call_index ON model_calls(source_kind, COALESCE(source_id, ''), call_index);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_work_item ON external_cli_runs(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_provider ON external_cli_runs(provider);
CREATE INDEX IF NOT EXISTS idx_external_cli_runs_status ON external_cli_runs(status);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_run_id ON external_cli_run_events(run_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_work_item ON external_cli_run_events(work_item_id);
CREATE INDEX IF NOT EXISTS idx_external_cli_run_events_sequence ON external_cli_run_events(run_id, sequence);
CREATE INDEX IF NOT EXISTS idx_delivery_items_product ON delivery_items(product_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_capability ON delivery_items(capability_id);
CREATE INDEX IF NOT EXISTS idx_delivery_items_slice ON delivery_items(capability_slice_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_run_status ON agent_work_items(run_id, status);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_batch ON agent_work_items(batch_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_agent ON agent_work_items(agent);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_commit ON agent_work_items(commit_sha);
CREATE INDEX IF NOT EXISTS idx_agent_work_items_work_item ON agent_work_items(work_item_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_run ON agent_work_evidence(run_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_feature ON agent_work_evidence(run_id, feature_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_batch ON agent_work_evidence(batch_id);
CREATE INDEX IF NOT EXISTS idx_agent_work_evidence_agent ON agent_work_evidence(agent);

PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;

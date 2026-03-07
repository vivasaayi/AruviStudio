// Domain types matching Rust backend

export interface Product {
  id: string;
  name: string;
  description: string;
  vision: string;
  goals: string[];
  tags: string[];
  status: "active" | "archived";
  created_at: string;
  updated_at: string;
}

export interface Module {
  id: string;
  product_id: string;
  name: string;
  description: string;
  purpose: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface Capability {
  id: string;
  module_id: string;
  parent_capability_id: string | null;
  level: number;
  sort_order: number;
  name: string;
  description: string;
  acceptance_criteria: string;
  priority: "critical" | "high" | "medium" | "low";
  risk: "high" | "medium" | "low";
  status: "draft" | "in_progress" | "done" | "archived";
  technical_notes: string;
  created_at: string;
  updated_at: string;
}

export interface WorkItem {
  id: string;
  product_id: string;
  module_id: string | null;
  capability_id: string | null;
  parent_work_item_id: string | null;
  title: string;
  problem_statement: string;
  description: string;
  acceptance_criteria: string;
  constraints: string;
  work_item_type: "feature" | "bug" | "refactor" | "test" | "review" | "security_fix" | "performance_improvement";
  priority: "critical" | "high" | "medium" | "low";
  complexity: "trivial" | "low" | "medium" | "high" | "very_high";
  status: "draft" | "ready_for_review" | "approved" | "in_planning" | "in_progress" | "in_validation" | "waiting_human_review" | "done" | "blocked" | "failed" | "cancelled";
  repo_override_id: string | null;
  active_repo_id: string | null;
  branch_name: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface Repository {
  id: string;
  name: string;
  local_path: string;
  remote_url: string;
  default_branch: string;
  auth_profile: string | null;
  created_at: string;
  updated_at: string;
}

export interface RepositoryTreeNode {
  name: string;
  relative_path: string;
  node_type: "file" | "directory";
  size_bytes: number | null;
  children: RepositoryTreeNode[];
}

export interface RepositoryAttachment {
  id: string;
  scope_type: "product" | "module";
  scope_id: string;
  repository_id: string;
  is_default: boolean;
  created_at: string;
}

export interface Approval {
  id: string;
  work_item_id: string;
  workflow_run_id: string | null;
  approval_type: "task_approval" | "plan_approval" | "test_review";
  status: "pending" | "approved" | "rejected";
  notes: string;
  acted_at: string | null;
  created_at: string;
}

export interface ModelProvider {
  id: string;
  name: string;
  provider_type: "openai_compatible" | "local_runtime";
  base_url: string;
  auth_secret_ref: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ModelDefinition {
  id: string;
  provider_id: string;
  name: string;
  context_window: number | null;
  capability_tags: string[];
  notes: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AgentModelBinding {
  id: string;
  agent_id: string;
  model_id: string;
  priority: number;
  created_at: string;
}

export interface AgentDefinition {
  id: string;
  name: string;
  role: string;
  description: string;
  prompt_template_ref: string;
  allowed_tools: string[];
  skill_tags: string[];
  boundaries: Record<string, unknown>;
  enabled: boolean;
  employment_status: "active" | "inactive" | "terminated";
  created_at: string;
  updated_at: string;
}

export interface AgentTeam {
  id: string;
  name: string;
  department: string;
  description: string;
  enabled: boolean;
  max_concurrent_workflows: number;
  created_at: string;
  updated_at: string;
}

export interface AgentTeamMembership {
  id: string;
  team_id: string;
  agent_id: string;
  title: string;
  is_lead: boolean;
  created_at: string;
}

export interface TeamAssignment {
  id: string;
  team_id: string;
  scope_type: "product" | "module" | "capability";
  scope_id: string;
  created_at: string;
}

export interface Skill {
  id: string;
  name: string;
  category: string;
  description: string;
  instructions: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AgentSkillLink {
  id: string;
  agent_id: string;
  skill_id: string;
  proficiency: "learning" | "working" | "expert";
  created_at: string;
}

export interface TeamSkillLink {
  id: string;
  team_id: string;
  skill_id: string;
  created_at: string;
}

export interface WorkflowStagePolicy {
  id: string;
  stage_name: string;
  primary_roles: string[];
  fallback_roles: string[];
  coordinator_required: boolean;
  created_at: string;
  updated_at: string;
}

export interface WorkflowRun {
  id: string;
  work_item_id: string;
  workflow_version: string;
  status: "running" | "paused" | "completed" | "failed" | "cancelled";
  current_stage: string;
  assigned_team_id: string | null;
  coordinator_agent_id: string | null;
  pending_stage_name: string | null;
  retry_count: number;
  max_retries: number;
  error_message: string | null;
  started_at: string;
  ended_at: string | null;
  updated_at: string;
}

export interface WorkflowStageHistory {
  id: string;
  workflow_run_id: string;
  from_stage: string;
  to_stage: string;
  trigger: string;
  notes: string;
  transitioned_at: string;
}

export interface AgentRun {
  id: string;
  workflow_run_id: string;
  agent_id: string;
  stage: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  prompt_snapshot_path: string | null;
  output_snapshot_path: string | null;
  token_count_input: number | null;
  token_count_output: number | null;
  duration_ms: number | null;
  error_message: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
}

export interface Artifact {
  id: string;
  work_item_id: string;
  workflow_run_id: string | null;
  agent_run_id: string | null;
  artifact_type: string;
  storage_path: string;
  summary: string;
  content_type: string;
  size_bytes: number | null;
  created_at: string;
}

export interface Finding {
  id: string;
  work_item_id: string;
  source_agent_run_id: string | null;
  category: "security" | "performance";
  severity: "critical" | "high" | "medium" | "low" | "info";
  title: string;
  description: string;
  status: "open" | "resolved" | "wont_fix" | "deferred";
  is_blocking: boolean;
  linked_followup_work_item_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProductTree {
  product: Product;
  modules: ModuleTree[];
}

export interface ModuleTree {
  module: Module;
  features: CapabilityTree[];
}

export interface CapabilityTree {
  capability: Capability;
  children: CapabilityTree[];
}

export interface MigrationStatus {
  version: number;
  description: string;
  success: boolean;
  installed_on: string;
}

export interface DatabaseHealth {
  applied_migrations: number;
  latest_version: number | null;
  migrations: MigrationStatus[];
}
export type Outcome = Capability;
export type CapabilityNode = Capability;

export interface ChatMessagePayload {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatCompletionResponse {
  content: string;
  token_count_input: number | null;
  token_count_output: number | null;
}

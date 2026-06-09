export type PlannerTreeNode = {
  id: string;
  label: string;
  meta?: string | null;
  node_type?: string | null;
  summary?: string | null;
  source?: string | null;
  confidence?: string | null;
  evidence?: string[];
  children: PlannerTreeNode[];
};

export type HierarchyNodeKind =
  | "area"
  | "domain"
  | "subdomain"
  | "system"
  | "subsystem"
  | "feature_set"
  | "capability"
  | "rollout"
  | "reference";

export type HierarchyNodeType = "module" | "capability";

export type Product = {
  id: string;
  name: string;
  description: string;
  vision: string;
  goals: string[];
  tags: string[];
  status: "active" | "archived";
  created_at: string;
  updated_at: string;
};

export type ProductTree = {
  product: Product;
  modules: ModuleTree[];
  roots: HierarchyTreeNode[];
};

export type ModuleTree = {
  module: Module;
  features: CapabilityTree[];
};

export type Module = {
  id: string;
  product_id: string;
  node_kind: HierarchyNodeKind;
  name: string;
  description: string;
  purpose: string;
  explanation: string;
  examples: string;
  implementation_notes: string;
  test_guidance: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
};

export type CapabilityTree = {
  capability: Capability;
  children: CapabilityTree[];
};

export type Capability = {
  id: string;
  module_id: string;
  parent_capability_id: string | null;
  level: number;
  node_kind: HierarchyNodeKind;
  sort_order: number;
  name: string;
  description: string;
  acceptance_criteria: string;
  explanation: string;
  examples: string;
  priority: "critical" | "high" | "medium" | "low";
  risk: "high" | "medium" | "low";
  status: "draft" | "in_progress" | "done" | "archived";
  technical_notes: string;
  implementation_notes: string;
  test_guidance: string;
  created_at: string;
  updated_at: string;
};

export type HierarchyTreeNode = {
  id: string;
  node_type: HierarchyNodeType;
  node_kind: HierarchyNodeKind;
  module_id: string;
  capability_id: string | null;
  parent_node_id: string | null;
  parent_node_type: HierarchyNodeType | null;
  depth: number;
  name: string;
  description: string;
  summary: string;
  path: string[];
  allowed_child_kinds: HierarchyNodeKind[];
  children: HierarchyTreeNode[];
};

export type PlannerPlan = {
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: Array<Record<string, unknown>>;
};

export type PlannerSessionInfo = {
  session_id: string;
  provider_id: string | null;
  model_name: string | null;
  has_pending_plan: boolean;
  has_draft_plan: boolean;
  selected_draft_node_id: string | null;
};

export type PlannerTurnResponse = {
  session_id: string;
  status: "proposal" | "clarification" | "report" | "execution" | "error" | "session_update";
  assistant_message: string;
  pending_plan: PlannerPlan | null;
  tree_nodes: PlannerTreeNode[] | null;
  draft_tree_nodes: PlannerTreeNode[] | null;
  selected_draft_node_id: string | null;
  execution_lines: string[];
  execution_errors: string[];
};

export type ChatMessagePayload = {
  role: "system" | "user" | "assistant";
  content: string;
};

export type ChatCompletionResponse = {
  content: string;
  token_count_input: number | null;
  token_count_output: number | null;
};

export type ModelCall = {
  id: string;
  source_kind: string;
  source_id: string | null;
  source_label: string;
  workflow_run_id: string | null;
  agent_run_id: string | null;
  work_item_id: string | null;
  product_id: string | null;
  session_id: string | null;
  agent_id: string | null;
  stage: string | null;
  provider_id: string;
  provider_name: string;
  provider_type: string;
  provider_base_url: string;
  model_id: string | null;
  model_name: string;
  call_index: number;
  request_message_count: number;
  prompt_chars: number;
  response_chars: number;
  request_snapshot_path: string | null;
  response_snapshot_path: string | null;
  max_tokens: number | null;
  temperature: number | null;
  token_count_input: number | null;
  token_count_output: number | null;
  duration_ms: number | null;
  status: "completed" | "failed";
  error_message: string | null;
  created_at: string;
};

export type MobilePlannerChatSession = {
  session_id: string;
  provider_id: string;
  model_name: string;
  product_id: string | null;
  product_name: string | null;
};

export type MobilePlannerToolTraceEntry = {
  step: number;
  tool_name: string;
  arguments: Record<string, unknown> | unknown;
  result: Record<string, unknown> | unknown | null;
  error: string | null;
};

export type MobilePlannerChatTurnResponse = {
  session_id: string;
  status: "final" | "tool_limit_final" | string;
  assistant_message: string;
  provider_id: string;
  model_name: string;
  product_id: string | null;
  product_name: string | null;
  tool_trace: MobilePlannerToolTraceEntry[];
  token_count_input: number | null;
  token_count_output: number | null;
};

import type { ChildReparentStrategy, HierarchyNodeKind, SemanticTemplateKind } from "./products";


export interface PlannerTarget {
  productName?: string;
  productAreaName?: string;
  capabilityName?: string;
  workItemTitle?: string;
}

export interface PlannerAction {
  type: string;
  target?: PlannerTarget;
  templateKind?: SemanticTemplateKind;
  nodeKind?: HierarchyNodeKind;
  childStrategy?: ChildReparentStrategy;
  name?: string;
  title?: string;
  description?: string;
  vision?: string;
  goals?: string[];
  tags?: string[];
  acceptanceCriteria?: string;
  explanation?: string;
  examples?: string;
  technicalNotes?: string;
  implementationNotes?: string;
  testGuidance?: string;
  problemStatement?: string;
  constraints?: string;
  workItemType?: string;
  priority?: string;
  complexity?: string;
  risk?: string;
  notes?: string;
  action?: string;
  fields?: Record<string, unknown>;
}

export interface PlannerPlan {
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: PlannerAction[];
}

export interface PlannerTreeNode {
  id: string;
  label: string;
  meta: string | null;
  node_type?: string | null;
  summary?: string | null;
  source?: string | null;
  confidence?: string | null;
  evidence?: string[];
  children: PlannerTreeNode[];
}

export interface PlannerTraceEvent {
  step: number;
  stage: string;
  title: string;
  detail: string;
}

export interface PlannerSessionInfo {
  session_id: string;
  provider_id: string | null;
  model_name: string | null;
  has_pending_plan: boolean;
  has_draft_plan: boolean;
  selected_draft_node_id: string | null;
}

export interface PlannerTurnResponse {
  session_id: string;
  status: "proposal" | "clarification" | "report" | "execution" | "error" | "session_update";
  assistant_message: string;
  pending_plan: PlannerPlan | null;
  tree_nodes: PlannerTreeNode[] | null;
  draft_tree_nodes: PlannerTreeNode[] | null;
  selected_draft_node_id: string | null;
  execution_lines: string[];
  execution_errors: string[];
  trace_events: PlannerTraceEvent[];
}

export type PlannerDraftChildType = "product_area" | "capability" | "work_item";

export interface SpeechToTextResponse {
  transcript: string;
}

export interface PlannerContactResult {
  channel: "whatsapp" | "voice";
  status: "sent" | "blocked";
  reason: string;
}

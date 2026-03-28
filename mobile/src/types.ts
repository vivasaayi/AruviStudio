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
  status: "proposal" | "clarification" | "report" | "execution" | "error";
  assistant_message: string;
  pending_plan: PlannerPlan | null;
  tree_nodes: PlannerTreeNode[] | null;
  draft_tree_nodes: PlannerTreeNode[] | null;
  selected_draft_node_id: string | null;
  execution_lines: string[];
  execution_errors: string[];
};

export type SpeechToTextResponse = {
  transcript: string;
};

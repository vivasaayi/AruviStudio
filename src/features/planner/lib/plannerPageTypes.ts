import type {
  PlannerDraftChildType,
  PlannerTraceEvent,
  Product,
  ProductTree,
  WorkItem,
} from "../../../lib/types";

export const PLANNER_WORK_ITEM_PAGE_SIZE = 500;

export type PlannerMessageKind = "text" | "proposal" | "tree" | "report" | "execution" | "error";

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

export type PlannerMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  meta?: string;
  kind?: PlannerMessageKind;
  plan?: PlannerPlan;
  treeNodes?: PlannerTreeNode[];
  traceEvents?: PlannerTraceEvent[];
};

export type PlannerAction =
  | {
      type: "update_product";
      target?: { productName?: string };
      fields: { name?: string; description?: string; vision?: string; goals?: string[]; tags?: string[] };
    }
  | {
      type: "create_product_area";
      target?: { productName?: string };
      name: string;
      description?: string;
      purpose?: string;
    }
  | {
      type: "update_product_area";
      target?: { productName?: string; productAreaName?: string };
      fields: { name?: string; description?: string; purpose?: string };
    }
  | { type: "delete_product_area"; target?: { productName?: string; productAreaName?: string } }
  | {
      type: "create_capability";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      name: string;
      description?: string;
      acceptanceCriteria?: string;
      technicalNotes?: string;
      priority?: "critical" | "high" | "medium" | "low";
      risk?: "high" | "medium" | "low";
    }
  | {
      type: "apply_capability_template";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      templateKind: "operator_chapter" | "technical_topic_book";
      name: string;
      description?: string;
      priority?: "critical" | "high" | "medium" | "low";
      risk?: "high" | "medium" | "low";
      explanation?: string;
      examples?: string;
      implementationNotes?: string;
      testGuidance?: string;
    }
  | {
      type: "convert_capability_kind";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      nodeKind: string;
      childStrategy?: "reject" | "reparent_to_parent";
    }
  | {
      type: "update_capability";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      fields: { name?: string; description?: string; acceptanceCriteria?: string; technicalNotes?: string; priority?: "critical" | "high" | "medium" | "low"; risk?: "high" | "medium" | "low" };
    }
  | { type: "delete_capability"; target?: { productName?: string; productAreaName?: string; capabilityName?: string } }
  | {
      type: "create_work_item";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      title: string;
      description?: string;
      problemStatement?: string;
      acceptanceCriteria?: string;
      constraints?: string;
      workItemType?: WorkItem["work_item_type"];
      priority?: WorkItem["priority"];
      complexity?: WorkItem["complexity"];
    }
  | {
      type: "update_work_item";
      target?: { productName?: string; workItemTitle?: string };
      fields: { title?: string; description?: string; problemStatement?: string; acceptanceCriteria?: string; constraints?: string; status?: WorkItem["status"] };
    }
  | { type: "delete_work_item"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "approve_work_item"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "reject_work_item"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "approve_work_item_plan"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "reject_work_item_plan"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "approve_work_item_test_review"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "start_workflow"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "workflow_action"; target?: { productName?: string; workItemTitle?: string }; action: "approve" | "reject" | "pause" | "resume" | "cancel"; notes?: string }
  | { type: "report_status"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "report_tree"; target?: { productName?: string } };

export type PlannerPlan = {
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: PlannerAction[];
};

export type PendingPlan = {
  sourceText: string;
  plan: PlannerPlan;
};

export type ResolverContext = {
  products: Product[];
  productTrees: ProductTree[];
  workItems: WorkItem[];
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
};

export type ExecutionResult = {
  lines: string[];
  errors: string[];
};

export type PlannerMutationResult =
  | {
      mode: "confirmed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "confirmation_required";
      userInput: string;
      plan: PlannerPlan;
      execution: null;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "draft_updated";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "clarification";
      userInput: string;
      plan: PlannerPlan;
      execution: null;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "executed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "session_updated";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "failed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    };

export type DraftEditOperation =
  | { kind: "rename"; nodeId: string; name: string }
  | { kind: "add_child"; parentNodeId: string; childType: PlannerDraftChildType; name: string; summary?: string }
  | { kind: "delete"; nodeId: string };

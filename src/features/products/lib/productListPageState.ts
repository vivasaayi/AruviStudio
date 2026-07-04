import type { HierarchyNodeKind, Product, ProductDependencyKind, ProductReference, WorkItem } from "../../../lib/types";

export const HIDE_EXAMPLE_PRODUCTS_KEY = "catalog.hide_example_products";
export const SUB_WORK_ITEM_PAGE_SIZE = 500;
export const PRODUCT_MANAGEMENT_STORY_PAGE_SIZE = 100;

export type ProductFormState = {
  name: string;
  description: string;
  vision: string;
  goals: string;
  tags: string;
  lifecycle: Product["lifecycle"];
  health: Product["health"];
  ownerLabel: string;
  investmentStatus: Product["investment_status"];
  roadmap: string;
  evidence: string;
};

export const emptyProductForm: ProductFormState = {
  name: "",
  description: "",
  vision: "",
  goals: "",
  tags: "",
  lifecycle: "incubating",
  health: "unknown",
  ownerLabel: "",
  investmentStatus: "evaluate",
  roadmap: "",
  evidence: "",
};

export type ProductAreaFormState = {
  name: string;
  description: string;
  purpose: string;
  nodeKind: HierarchyNodeKind;
};

export const emptyProductAreaForm: ProductAreaFormState = {
  name: "",
  description: "",
  purpose: "",
  nodeKind: "product_area",
};

export type CapabilityFormState = {
  name: string;
  description: string;
  acceptanceCriteria: string;
  technicalNotes: string;
  nodeKind: HierarchyNodeKind;
};

export function createEmptyCapabilityForm(nodeKind: HierarchyNodeKind): CapabilityFormState {
  return {
    name: "",
    description: "",
    acceptanceCriteria: "",
    technicalNotes: "",
    nodeKind,
  };
}

export function productToForm(product: Product): ProductFormState {
  return {
    name: product.name,
    description: product.description,
    vision: product.vision,
    goals: product.goals.join(", "),
    tags: product.tags.join(", "),
    lifecycle: product.lifecycle,
    health: product.health,
    ownerLabel: product.owner_label,
    investmentStatus: product.investment_status,
    roadmap: product.roadmap,
    evidence: product.evidence,
  };
}

export const productLifecycleOptions: Product["lifecycle"][] = ["idea", "incubating", "active", "maturing", "sunsetting", "retired"];
export const productHealthOptions: Product["health"][] = ["unknown", "healthy", "watch", "at_risk", "blocked"];
export const productInvestmentOptions: Product["investment_status"][] = ["evaluate", "invest", "maintain", "pause", "retire"];
export const referenceKindOptions: ProductReference["reference_kind"][] = ["note", "external_doc", "architecture", "customer_evidence", "regulatory", "design_packet", "standard", "other"];

export type ProductDependencyDraft = {
  capabilityId: string;
  dependsOnProductId: string;
  dependsOnCapabilityId: string;
  dependencyKind: ProductDependencyKind;
  description: string;
};

export const emptyProductDependencyDraft: ProductDependencyDraft = {
  capabilityId: "",
  dependsOnProductId: "",
  dependsOnCapabilityId: "",
  dependencyKind: "platform",
  description: "",
};

export type WorkItemDraftState = {
  title: string;
  problemStatement: string;
  description: string;
  acceptanceCriteria: string;
  constraints: string;
  status: WorkItem["status"];
  priority: WorkItem["priority"];
  complexity: WorkItem["complexity"];
};

export const emptyWorkItemDraft: WorkItemDraftState = {
  title: "",
  problemStatement: "",
  description: "",
  acceptanceCriteria: "",
  constraints: "",
  status: "draft",
  priority: "medium",
  complexity: "medium",
};

export const workItemStatusOptions: WorkItem["status"][] = ["draft", "ready_for_review", "approved", "in_planning", "in_progress", "in_validation", "waiting_human_review", "done", "blocked", "failed", "cancelled"];
export const workItemPriorityOptions: WorkItem["priority"][] = ["critical", "high", "medium", "low"];
export const workItemComplexityOptions: WorkItem["complexity"][] = ["trivial", "low", "medium", "high", "very_high"];

export function workItemToDraft(workItem: WorkItem): WorkItemDraftState {
  return {
    title: workItem.title,
    problemStatement: workItem.problem_statement,
    description: workItem.description,
    acceptanceCriteria: workItem.acceptance_criteria,
    constraints: workItem.constraints,
    status: workItem.status,
    priority: workItem.priority,
    complexity: workItem.complexity,
  };
}

export function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

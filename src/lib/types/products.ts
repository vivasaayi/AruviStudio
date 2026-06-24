import type { WorkItem } from "./workItems";


export type HierarchyNodeKind =
  | "product_area"
  | "capability"
  | "feature";

export type HierarchyNodeType = "product_area" | "capability";

export interface Product {
  id: string;
  name: string;
  description: string;
  vision: string;
  goals: string[];
  tags: string[];
  status: "active" | "archived";
  lifecycle: "idea" | "incubating" | "active" | "maturing" | "sunsetting" | "retired";
  health: "unknown" | "healthy" | "watch" | "at_risk" | "blocked";
  owner_label: string;
  investment_status: "evaluate" | "invest" | "maintain" | "pause" | "retire";
  roadmap: string;
  evidence: string;
  created_at: string;
  updated_at: string;
}

export interface CapabilitySlice {
  id: string;
  capability_id: string;
  name: string;
  description: string;
  acceptance_criteria: string;
  priority: "critical" | "high" | "medium" | "low";
  status: "draft" | "in_progress" | "done" | "archived";
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface ProductReference {
  id: string;
  scope_type: "strategy_node" | "product" | "product_area" | "capability" | "feature" | "delivery_item";
  scope_id: string;
  title: string;
  reference_kind: "note" | "external_doc" | "architecture" | "customer_evidence" | "regulatory" | "design_packet" | "standard" | "other";
  uri: string;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface ProductArea {
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
}

export interface Capability {
  id: string;
  product_area_id: string;
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
}

export interface ProductTree {
  product: Product;
  product_areas: ProductAreaTree[];
  roots: HierarchyTreeNode[];
}

export interface ProductTreeSummary {
  product_id: string;
  product_area_count: number;
  capability_count: number;
  total_node_count: number;
  leaf_node_count: number;
}

export interface ProductAreaTree {
  product_area: ProductArea;
  features: CapabilityTree[];
}

export interface CapabilityTree {
  capability: Capability;
  children: CapabilityTree[];
}

export interface HierarchyTreeNode {
  id: string;
  node_type: HierarchyNodeType;
  node_kind: HierarchyNodeKind;
  product_area_id: string;
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
}

export type ProductFeature = Capability;
export type Outcome = ProductFeature;
export type CapabilityNode = Capability;
export type SemanticTemplateKind = "operator_chapter" | "technical_topic_book";
export type ChildReparentStrategy = "reject" | "reparent_to_parent";

export interface SemanticTemplateApplicationResult {
  template_kind: SemanticTemplateKind;
  parent_node_id: string;
  parent_node_type: HierarchyNodeType;
  topic_node: Capability;
  created_nodes: Capability[];
  created_work_items: WorkItem[];
}

export interface NodeKindConversionResult {
  capability: Capability;
  previous_node_kind: HierarchyNodeKind;
  child_strategy: ChildReparentStrategy | null;
  reparented_children: Capability[];
}

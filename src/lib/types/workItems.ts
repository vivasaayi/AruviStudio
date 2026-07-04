import type { HierarchyNodeType } from "./products";


export interface WorkItem {
  id: string;
  product_id: string;
  product_area_id: string | null;
  capability_id: string | null;
  source_node_id: string | null;
  source_node_type: HierarchyNodeType | null;
  parent_work_item_id: string | null;
  title: string;
  problem_statement: string;
  description: string;
  acceptance_criteria: string;
  constraints: string;
  work_item_type: "story" | "task" | "setup" | "bug" | "refactor" | "test" | "review" | "security_fix" | "performance_improvement";
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

export interface WorkItemPage {
  items: WorkItem[];
  limit: number;
  offset: number;
  has_more: boolean;
}

export interface ProductWorkItemSummary {
  product_id: string;
  total_count: number;
  active_count: number;
  done_count: number;
  blocked_count: number;
}

export interface WorkItemScopeSummary {
  product_id: string;
  product_area_id: string | null;
  capability_id: string | null;
  source_node_id: string | null;
  source_node_type: HierarchyNodeType | null;
  status: WorkItem["status"];
  total_count: number;
  top_level_count: number;
  active_count: number;
  done_count: number;
  blocked_count: number;
}

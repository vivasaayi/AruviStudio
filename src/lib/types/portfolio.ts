export type StrategyNodeKind = "strategic_product_area" | "domain" | "sub_domain";

export interface StrategyNode {
  id: string;
  parent_node_id: string | null;
  node_kind: StrategyNodeKind;
  name: string;
  description: string;
  owner_label: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface ProductStrategyLink {
  id: string;
  product_id: string;
  strategy_node_id: string;
  is_primary: boolean;
  created_at: string;
}

export type ProductDependencyKind = "platform" | "capability" | "data" | "integration" | "operational" | "other";
export type ProductDependencyStatus = "active" | "planned" | "blocked" | "retired";

export interface ProductDependency {
  id: string;
  product_id: string;
  capability_id: string | null;
  depends_on_product_id: string;
  depends_on_capability_id: string | null;
  dependency_kind: ProductDependencyKind;
  description: string;
  status: ProductDependencyStatus;
  created_at: string;
  updated_at: string;
}

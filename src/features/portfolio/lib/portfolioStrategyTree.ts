import type { ProductStrategyLink, StrategyNode, StrategyNodeKind } from "../../../lib/types";

export type StrategyTreeNode = StrategyNode & { children: StrategyTreeNode[] };

export const strategyKindLabels: Record<StrategyNodeKind, string> = {
  strategic_product_area: "Strategic Product Area",
  domain: "Domain",
  sub_domain: "Sub Domain",
};

export function buildStrategyTree(nodes: StrategyNode[]): StrategyTreeNode[] {
  const byId = new Map<string, StrategyTreeNode>();
  nodes.forEach((node) => byId.set(node.id, { ...node, children: [] }));
  const roots: StrategyTreeNode[] = [];
  byId.forEach((node) => {
    if (node.parent_node_id && byId.has(node.parent_node_id)) {
      byId.get(node.parent_node_id)!.children.push(node);
      return;
    }
    roots.push(node);
  });
  const sortNodes = (items: StrategyTreeNode[]) => {
    items.sort((left, right) => left.sort_order - right.sort_order || left.name.localeCompare(right.name));
    items.forEach((item) => sortNodes(item.children));
  };
  sortNodes(roots);
  return roots;
}

export function findTreeNode(nodes: StrategyTreeNode[], id: string): StrategyTreeNode | null {
  for (const node of nodes) {
    if (node.id === id) {
      return node;
    }
    const found = findTreeNode(node.children, id);
    if (found) {
      return found;
    }
  }
  return null;
}

export function collectStrategySubtreeIds(nodes: StrategyTreeNode[], targetId: string): string[] {
  const target = findTreeNode(nodes, targetId);
  return target ? collectIds(target) : [];
}

function collectIds(node: StrategyTreeNode): string[] {
  return [node.id, ...node.children.flatMap(collectIds)];
}

export function collectDescendantIds(nodes: StrategyNode[], nodeId: string): string[] {
  const children = nodes.filter((node) => node.parent_node_id === nodeId);
  return children.flatMap((child) => [child.id, ...collectDescendantIds(nodes, child.id)]);
}

export function countProductsForStrategy(
  node: StrategyTreeNode | null,
  links: ProductStrategyLink[],
): number {
  if (!node) {
    return 0;
  }
  const ids = new Set(collectIds(node));
  return links.filter((link) => ids.has(link.strategy_node_id)).length;
}

export function getChildKind(kind: StrategyNodeKind): StrategyNodeKind | null {
  switch (kind) {
    case "strategic_product_area":
      return "domain";
    case "domain":
      return "sub_domain";
    case "sub_domain":
      return null;
  }
}

export function formatPortfolioError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

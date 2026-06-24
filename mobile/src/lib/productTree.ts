import type { HierarchyTreeNode } from "../types";

export type FlatProductNode = {
  node: HierarchyTreeNode;
  pathLabel: string;
};

export function productViewNeedsTree(mode: string) {
  return mode === "map" || mode === "search";
}

export function formatNodeKind(value?: string | null) {
  return String(value ?? "node").replace(/_/g, " ");
}

export function getNodeSummary(node: HierarchyTreeNode) {
  return node.summary || node.description || "No summary yet.";
}

export function countTreeNodes(nodes: HierarchyTreeNode[]): number {
  return nodes.reduce((total, node) => total + 1 + countTreeNodes(node.children ?? []), 0);
}

export function countLeafNodes(nodes: HierarchyTreeNode[]): number {
  return nodes.reduce((total, node) => {
    const children = node.children ?? [];
    if (!children.length) return total + 1;
    return total + countLeafNodes(children);
  }, 0);
}

export function flattenProductNodes(nodes: HierarchyTreeNode[], parentPath: string[] = []): FlatProductNode[] {
  return nodes.flatMap((node) => {
    const path = [...parentPath, node.name];
    return [
      {
        node,
        pathLabel: path.join(" / "),
      },
      ...flattenProductNodes(node.children ?? [], path),
    ];
  });
}

export function findProductNode(nodes: HierarchyTreeNode[], nodeId: string | null): HierarchyTreeNode | null {
  if (!nodeId) return null;
  for (const node of nodes) {
    if (node.id === nodeId) return node;
    const childMatch = findProductNode(node.children ?? [], nodeId);
    if (childMatch) return childMatch;
  }
  return null;
}

export function findProductNodePath(nodes: HierarchyTreeNode[], nodeId: string | null): HierarchyTreeNode[] {
  if (!nodeId) return [];
  for (const node of nodes) {
    if (node.id === nodeId) return [node];
    const childPath = findProductNodePath(node.children ?? [], nodeId);
    if (childPath.length) return [node, ...childPath];
  }
  return [];
}

import type { HierarchyNodeType, HierarchyTreeNode, ProductTree, WorkItem } from "./types";

export function getHierarchyNodeKey(node: Pick<HierarchyTreeNode, "id" | "node_type">) {
  return `${node.node_type}:${node.id}`;
}

export function getWorkItemOwnerKey(workItem: WorkItem) {
  if (workItem.source_node_id && workItem.source_node_type) {
    return `${workItem.source_node_type}:${workItem.source_node_id}`;
  }
  if (workItem.capability_id) {
    return `capability:${workItem.capability_id}`;
  }
  if (workItem.module_id) {
    return `module:${workItem.module_id}`;
  }
  return "product";
}

export function isDirectProductWorkItem(workItem: WorkItem) {
  return getWorkItemOwnerKey(workItem) === "product";
}

export function flattenHierarchyNodes(roots: HierarchyTreeNode[]) {
  const nodes: HierarchyTreeNode[] = [];

  const visit = (node: HierarchyTreeNode) => {
    nodes.push(node);
    node.children.forEach(visit);
  };

  roots.forEach(visit);
  return nodes;
}

export function countHierarchyNodes(roots: HierarchyTreeNode[]) {
  return flattenHierarchyNodes(roots).length;
}

export function countLeafNodes(roots: HierarchyTreeNode[]) {
  return flattenHierarchyNodes(roots).filter((node) => node.children.length === 0).length;
}

export function countDescendantNodes(node: HierarchyTreeNode) {
  return flattenHierarchyNodes([node]).length - 1;
}

export function findHierarchyNode(
  roots: HierarchyTreeNode[],
  nodeId: string | null | undefined,
  nodeType?: HierarchyNodeType | null,
): HierarchyTreeNode | null {
  if (!nodeId) {
    return null;
  }

  for (const node of roots) {
    if (node.id === nodeId && (!nodeType || node.node_type === nodeType)) {
      return node;
    }
    const child = findHierarchyNode(node.children, nodeId, nodeType);
    if (child) {
      return child;
    }
  }

  return null;
}

export function findHierarchyNodePath(
  roots: HierarchyTreeNode[],
  nodeId: string | null | undefined,
  nodeType?: HierarchyNodeType | null,
): HierarchyTreeNode[] {
  if (!nodeId) {
    return [];
  }

  const visit = (node: HierarchyTreeNode): HierarchyTreeNode[] | null => {
    if (node.id === nodeId && (!nodeType || node.node_type === nodeType)) {
      return [node];
    }
    for (const child of node.children) {
      const result = visit(child);
      if (result) {
        return [node, ...result];
      }
    }
    return null;
  };

  for (const root of roots) {
    const result = visit(root);
    if (result) {
      return result;
    }
  }

  return [];
}

export function getHierarchyNodeSectionId(
  node: Pick<HierarchyTreeNode, "id" | "node_type"> | null | undefined,
) {
  if (!node) {
    return "product-overview-top";
  }
  return `${node.node_type}-${node.id}`;
}

export function getDirectWorkItemsForNode(node: HierarchyTreeNode, workItems: WorkItem[]) {
  const nodeKey = getHierarchyNodeKey(node);
  return workItems.filter((workItem) => getWorkItemOwnerKey(workItem) === nodeKey);
}

export function getProductDirectWorkItems(workItems: WorkItem[]) {
  return workItems.filter(isDirectProductWorkItem);
}

export function getSubtreeWorkItemsForNode(node: HierarchyTreeNode, workItems: WorkItem[]) {
  const subtreeKeys = new Set(flattenHierarchyNodes([node]).map(getHierarchyNodeKey));
  return workItems.filter((workItem) => subtreeKeys.has(getWorkItemOwnerKey(workItem)));
}

export function getDirectChildNodes(
  tree: ProductTree | undefined,
  node: HierarchyTreeNode | null,
) {
  if (!tree) {
    return [];
  }
  return node ? node.children : tree.roots;
}

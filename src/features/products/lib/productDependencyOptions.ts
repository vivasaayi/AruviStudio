import { flattenHierarchyNodes } from "../../../lib/hierarchyTree";
import type { HierarchyTreeNode, ProductTree } from "../../../lib/types";

export type CapabilityOption = {
  id: string;
  label: string;
};

export function buildCapabilityOptionsFromNodes(nodes: HierarchyTreeNode[]): CapabilityOption[] {
  return nodes
    .filter((node) => node.node_type === "capability")
    .map((node) => ({ id: node.id, label: node.path.join(" / ") }));
}

export function buildDependencyTargetCapabilityOptions(
  productTreeById: Map<string, ProductTree>,
  targetProductId: string,
): CapabilityOption[] {
  const targetTree = productTreeById.get(targetProductId);
  return targetTree ? buildCapabilityOptionsFromNodes(flattenHierarchyNodes(targetTree.roots)) : [];
}

export function buildCapabilityLabelById(
  productTreeById: Map<string, ProductTree>,
  currentTreeNodes: HierarchyTreeNode[],
) {
  const map = new Map<string, string>();
  productTreeById.forEach((productTree) => {
    buildCapabilityOptionsFromNodes(flattenHierarchyNodes(productTree.roots))
      .forEach((option) => map.set(option.id, option.label));
  });
  buildCapabilityOptionsFromNodes(currentTreeNodes)
    .forEach((option) => map.set(option.id, option.label));
  return map;
}

import type { CapabilityTree, ProductAreaTree } from "../../../lib/types";

export type HierarchyDeleteKind = "product_area" | "capability" | "feature";

export function countCapabilities(productAreas: ProductAreaTree[]) {
  return productAreas.reduce(
    (total, productAreaTree) => total + productAreaTree.features.reduce(
      (sum, capabilityTree) => sum + countCapabilityTree(capabilityTree),
      0,
    ),
    0,
  );
}

export function countCapabilityTree(capabilityTree: CapabilityTree): number {
  return 1 + capabilityTree.children.reduce((sum, child) => sum + countCapabilityTree(child), 0);
}

export function getHierarchyDeleteLabel(kind: HierarchyDeleteKind) {
  switch (kind) {
    case "product_area":
      return "Product Area";
    case "capability":
      return "Capability";
    case "feature":
      return "Feature";
  }
}

export function findCapabilityTree(productAreas: ProductAreaTree[], capabilityId: string | null): CapabilityTree | null {
  if (!capabilityId) {
    return null;
  }

  for (const productAreaTree of productAreas) {
    for (const capabilityTree of productAreaTree.features) {
      const found = searchCapabilityTree(capabilityTree, capabilityId);
      if (found) {
        return found;
      }
    }
  }

  return null;
}

function searchCapabilityTree(capabilityTree: CapabilityTree, capabilityId: string | null): CapabilityTree | null {
  if (!capabilityId) {
    return null;
  }
  if (capabilityTree.capability.id === capabilityId) {
    return capabilityTree;
  }

  for (const child of capabilityTree.children) {
    const found = searchCapabilityTree(child, capabilityId);
    if (found) {
      return found;
    }
  }

  return null;
}

export function flattenCapabilityTreeList(nodes: CapabilityTree[]): CapabilityTree[] {
  return nodes.flatMap((node) => [node, ...flattenCapabilityTreeList(node.children)]);
}

export function getCapabilityOrderKey(productAreaId: string, parentCapabilityId: string | null) {
  return `${productAreaId}:${parentCapabilityId ?? "root"}`;
}

export function seedCapabilityOrderMap(target: Record<string, string[]>, nodes: CapabilityTree[]) {
  nodes.forEach((node) => {
    target[getCapabilityOrderKey(node.capability.product_area_id, node.capability.id)] = node.children.map((child) => child.capability.id);
    seedCapabilityOrderMap(target, node.children);
  });
}

export function getOrderedCapabilityTrees(nodes: CapabilityTree[], orderedIds?: string[]) {
  return orderItemsByIds(nodes, orderedIds ?? [], (node) => node.capability.id);
}

export function orderItemsByIds<T>(items: T[], orderedIds: string[], getId: (item: T) => string) {
  if (orderedIds.length === 0) {
    return items;
  }
  const rank = new Map(orderedIds.map((id, index) => [id, index]));
  return [...items].sort((a, b) => (rank.get(getId(a)) ?? Number.MAX_SAFE_INTEGER) - (rank.get(getId(b)) ?? Number.MAX_SAFE_INTEGER));
}

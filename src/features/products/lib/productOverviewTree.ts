import type {
  Capability,
  CapabilityTree,
  HierarchyTreeNode,
  Product,
  ProductArea,
  ProductTree,
} from "../../../lib/types";

export function buildProductAreaOnlyTree(product: Product, productAreas: ProductArea[]): ProductTree {
  const productAreaTrees = productAreas.map((product_area) => ({
    product_area,
    features: [],
  }));

  return {
    product,
    product_areas: productAreaTrees,
    roots: productAreas.map(productAreaToHierarchyRoot),
  };
}

function productAreaToHierarchyRoot(productArea: ProductArea): HierarchyTreeNode {
  return {
    id: productArea.id,
    node_type: "product_area",
    node_kind: productArea.node_kind,
    product_area_id: productArea.id,
    capability_id: null,
    parent_node_id: null,
    parent_node_type: null,
    depth: 0,
    name: productArea.name,
    description: productArea.description,
    summary: productArea.description || productArea.purpose,
    path: [productArea.name],
    allowed_child_kinds: ["capability"],
    children: [],
  };
}

export function buildCapabilityTrees(capabilities: Capability[]): CapabilityTree[] {
  const childrenByParent = new Map<string, Capability[]>();
  capabilities.forEach((capability) => {
    const parentKey = capability.parent_capability_id ?? "";
    const siblings = childrenByParent.get(parentKey) ?? [];
    siblings.push(capability);
    childrenByParent.set(parentKey, siblings);
  });

  const sortCapabilities = (items: Capability[]) =>
    [...items].sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name));
  const buildTree = (capability: Capability): CapabilityTree => ({
    capability,
    children: sortCapabilities(childrenByParent.get(capability.id) ?? []).map(buildTree),
  });

  return sortCapabilities(childrenByParent.get("") ?? []).map(buildTree);
}

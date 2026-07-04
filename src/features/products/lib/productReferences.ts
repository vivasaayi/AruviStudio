import type { Capability, HierarchyTreeNode, ProductReference, ProductTree } from "../../../lib/types";

export type ProductReferenceScope = {
  scopeType: ProductReference["scope_type"];
  scopeId: string;
};

export function getProductAreaReferenceScope(productAreaId: string): ProductReferenceScope {
  return { scopeType: "product_area", scopeId: productAreaId };
}

export function getCapabilityReferenceScope(capability: Capability): ProductReferenceScope {
  return {
    scopeType: capability.node_kind === "feature" ? "feature" : "capability",
    scopeId: capability.id,
  };
}

export function filterReferencesForScope(
  references: ProductReference[],
  scope: ProductReferenceScope,
): ProductReference[] {
  return references.filter(
    (reference) => reference.scope_type === scope.scopeType && reference.scope_id === scope.scopeId,
  );
}

export function filterReferencesForProductBook(
  productId: string,
  tree: ProductTree | undefined,
  references: ProductReference[],
): ProductReference[] {
  const scopeKeys = new Set<string>([`product:${productId}`]);
  const visit = (node: HierarchyTreeNode) => {
    if (node.node_type === "product_area") {
      scopeKeys.add(`product_area:${node.id}`);
    } else if (node.capability_id) {
      scopeKeys.add(`${node.node_kind === "feature" ? "feature" : "capability"}:${node.capability_id}`);
    }
    node.children.forEach(visit);
  };

  (tree?.roots ?? []).forEach(visit);

  return references.filter((reference) => scopeKeys.has(`${reference.scope_type}:${reference.scope_id}`));
}

export function getReferenceKindLabel(kind: ProductReference["reference_kind"]): string {
  return kind
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

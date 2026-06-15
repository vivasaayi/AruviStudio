import type { HierarchyNodeKind } from "./types";

type LabelForms = {
  singular: string;
  plural: string;
  singularLower: string;
  pluralLower: string;
};

const NODE_KIND_LABELS: Record<HierarchyNodeKind, LabelForms> = {
  area: {
    singular: "Product Area",
    plural: "Product Areas",
    singularLower: "product area",
    pluralLower: "product areas",
  },
  capability: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  feature: {
    singular: "Feature",
    plural: "Features",
    singularLower: "feature",
    pluralLower: "features",
  },
};

const NODE_KIND_GUIDANCE: Record<HierarchyNodeKind, string> = {
  area: "Use for a durable top-level product area.",
  capability: "Use for something the product must be able to do inside a product area.",
  feature: "Use for a product-visible feature under a capability. Delivery stories and tasks execute against features.",
};

export const ROOT_NODE_KINDS: HierarchyNodeKind[] = ["area"];
export const NODE_KIND_DISPLAY_ORDER: HierarchyNodeKind[] = ["area", "capability", "feature"];

export const NODE_KIND_GROUPS: Array<{ label: string; kinds: HierarchyNodeKind[] }> = [
  { label: "Product Management", kinds: ["area", "capability", "feature"] },
];

export function orderHierarchyNodeKinds(nodeKinds: HierarchyNodeKind[]) {
  const present = new Set(nodeKinds);
  return NODE_KIND_DISPLAY_ORDER.filter((nodeKind) => present.has(nodeKind));
}

export function groupHierarchyNodeKinds(nodeKinds: HierarchyNodeKind[]) {
  const present = new Set(nodeKinds);
  return NODE_KIND_GROUPS
    .map((group) => ({
      ...group,
      kinds: group.kinds.filter((nodeKind) => present.has(nodeKind)),
    }))
    .filter((group) => group.kinds.length > 0);
}

export function getHierarchyNodeKindLabel(
  nodeKind: HierarchyNodeKind | null | undefined,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const labels = nodeKind ? NODE_KIND_LABELS[nodeKind] : undefined;
  if (!labels) {
    if (options.lowercase) {
      return options.plural ? "nodes" : "node";
    }
    return options.plural ? "Nodes" : "Node";
  }
  if (options.lowercase) {
    return options.plural ? labels.pluralLower : labels.singularLower;
  }
  return options.plural ? labels.plural : labels.singular;
}

export function supportsHierarchyChildren(nodeKind: HierarchyNodeKind | null | undefined) {
  return Boolean(nodeKind && nodeKind !== "feature");
}

export function getHierarchyNodeKindGuidance(nodeKind: HierarchyNodeKind | null | undefined) {
  if (!nodeKind) {
    return "Choose the semantic role this node plays in Product Area > Capability > Feature.";
  }
  return NODE_KIND_GUIDANCE[nodeKind];
}

export function getAllowedChildNodeKinds(parentKind: HierarchyNodeKind | null | undefined): HierarchyNodeKind[] {
  switch (parentKind) {
    case "area":
      return ["capability"];
    case "capability":
      return ["feature"];
    case "feature":
      return [];
    default:
      return ["area"];
  }
}

export function getDefaultChildNodeKind(parentKind: HierarchyNodeKind | null | undefined): HierarchyNodeKind {
  switch (parentKind) {
    case "area":
      return "capability";
    case "capability":
      return "feature";
    case "feature":
      return "feature";
    default:
      return "area";
  }
}

export function getHierarchyChildLabel(
  parentKind: HierarchyNodeKind | null | undefined,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const allowedChildKinds = getAllowedChildNodeKinds(parentKind);
  if (allowedChildKinds.length === 1) {
    return getHierarchyNodeKindLabel(allowedChildKinds[0], options);
  }
  if (options.lowercase) {
    return options.plural ? "children" : "child";
  }
  return options.plural ? "Children" : "Child";
}

function legacyLevelToNodeKind(level: number): HierarchyNodeKind {
  return level <= 0 ? "capability" : "feature";
}

export function getCapabilityHierarchyLabel(
  levelOrKind: number | HierarchyNodeKind,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const nodeKind = typeof levelOrKind === "number" ? legacyLevelToNodeKind(levelOrKind) : levelOrKind;
  return getHierarchyNodeKindLabel(nodeKind, options);
}

export function getCapabilityChildLabel(
  levelOrKind: number | HierarchyNodeKind,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const nodeKind = typeof levelOrKind === "number" ? legacyLevelToNodeKind(levelOrKind) : levelOrKind;
  return getHierarchyChildLabel(nodeKind, options);
}

export function isCapabilityFeatureLevel(levelOrKind: number | HierarchyNodeKind) {
  const nodeKind = typeof levelOrKind === "number" ? legacyLevelToNodeKind(levelOrKind) : levelOrKind;
  return nodeKind === "feature";
}

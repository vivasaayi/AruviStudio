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
  domain: {
    singular: "Product Area",
    plural: "Product Areas",
    singularLower: "product area",
    pluralLower: "product areas",
  },
  subdomain: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  system: {
    singular: "Product Area",
    plural: "Product Areas",
    singularLower: "product area",
    pluralLower: "product areas",
  },
  subsystem: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  feature_set: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  capability: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  rollout: {
    singular: "Feature",
    plural: "Features",
    singularLower: "feature",
    pluralLower: "features",
  },
  reference: {
    singular: "Attached Reference",
    plural: "Attached References",
    singularLower: "attached reference",
    pluralLower: "attached references",
  },
};

const NODE_KIND_GUIDANCE: Record<HierarchyNodeKind, string> = {
  area: "Use for a durable top-level product area. Strategy belongs in Portfolio; this is the product management tree.",
  domain: "Use for a durable top-level product area when existing data stores this root as a domain.",
  subdomain: "Use for a nested capability grouping when existing data already uses this kind.",
  system: "Use for a durable top-level product area when existing data stores this root as a system.",
  subsystem: "Use for a nested capability grouping when existing data already uses this kind.",
  feature_set: "Use as a capability grouping only when it clarifies features under a product capability.",
  capability: "Use for something the product must be able to do. Capabilities can contain product-visible features and attached references.",
  rollout: "Use for a product-visible feature under a capability. Delivery stories and tasks execute against features in Builder.",
  reference: "Use for attached context such as notes, standards, evidence, constraints, or design packets.",
};

export const ROOT_NODE_KINDS: HierarchyNodeKind[] = ["area", "domain", "system"];
export const NODE_KIND_DISPLAY_ORDER: HierarchyNodeKind[] = [
  "area",
  "domain",
  "subdomain",
  "system",
  "subsystem",
  "feature_set",
  "capability",
  "rollout",
  "reference",
];

export const NODE_KIND_GROUPS: Array<{ label: string; kinds: HierarchyNodeKind[] }> = [
  { label: "Product management", kinds: ["area", "domain", "system", "subdomain", "subsystem", "feature_set", "capability", "rollout"] },
  { label: "Attached context", kinds: ["reference"] },
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
  return Boolean(nodeKind && nodeKind !== "rollout" && nodeKind !== "reference");
}

export function getHierarchyNodeKindGuidance(nodeKind: HierarchyNodeKind | null | undefined) {
  if (!nodeKind) {
    return "Choose the semantic role this node plays in the product model.";
  }
  return NODE_KIND_GUIDANCE[nodeKind];
}

export function getAllowedChildNodeKinds(parentKind: HierarchyNodeKind | null | undefined): HierarchyNodeKind[] {
  switch (parentKind) {
    case "area":
    case "domain":
    case "system":
      return ["capability"];
    case "subdomain":
    case "subsystem":
    case "feature_set":
    case "capability":
      return ["rollout"];
    case "rollout":
    case "reference":
      return [];
    default:
      return ["capability"];
  }
}

export function getDefaultChildNodeKind(parentKind: HierarchyNodeKind | null | undefined): HierarchyNodeKind {
  switch (parentKind) {
    case "capability":
      return "rollout";
    case "feature_set":
      return "capability";
    case "area":
    case "domain":
    case "subdomain":
    case "system":
    case "subsystem":
      return "capability";
    default:
      return "capability";
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
  return level <= 0 ? "capability" : "rollout";
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

export function isCapabilityRolloutLevel(levelOrKind: number | HierarchyNodeKind) {
  const nodeKind = typeof levelOrKind === "number" ? legacyLevelToNodeKind(levelOrKind) : levelOrKind;
  return nodeKind === "rollout";
}

import type { HierarchyNodeKind } from "./types";

type LabelForms = {
  singular: string;
  plural: string;
  singularLower: string;
  pluralLower: string;
};

const NODE_KIND_LABELS: Record<HierarchyNodeKind, LabelForms> = {
  area: {
    singular: "Area",
    plural: "Areas",
    singularLower: "area",
    pluralLower: "areas",
  },
  domain: {
    singular: "Domain",
    plural: "Domains",
    singularLower: "domain",
    pluralLower: "domains",
  },
  subdomain: {
    singular: "Subdomain",
    plural: "Subdomains",
    singularLower: "subdomain",
    pluralLower: "subdomains",
  },
  system: {
    singular: "System",
    plural: "Systems",
    singularLower: "system",
    pluralLower: "systems",
  },
  subsystem: {
    singular: "Subsystem",
    plural: "Subsystems",
    singularLower: "subsystem",
    pluralLower: "subsystems",
  },
  feature_set: {
    singular: "Feature Set",
    plural: "Feature Sets",
    singularLower: "feature set",
    pluralLower: "feature sets",
  },
  capability: {
    singular: "Capability",
    plural: "Capabilities",
    singularLower: "capability",
    pluralLower: "capabilities",
  },
  rollout: {
    singular: "Rollout",
    plural: "Rollouts",
    singularLower: "rollout",
    pluralLower: "rollouts",
  },
  reference: {
    singular: "Reference",
    plural: "References",
    singularLower: "reference",
    pluralLower: "references",
  },
};

const NODE_KIND_GUIDANCE: Record<HierarchyNodeKind, string> = {
  area: "Use for a broad product, business, user, or operational area. Areas can contain other areas, domains, systems, feature sets, capabilities, and references.",
  domain: "Use for a subject-matter boundary or major concept space. Domain is the anchor level; use subdomains for deeper domain breakdowns.",
  subdomain: "Use for a narrower domain slice when a domain is too broad. Subdomains can nest and can lead to systems, feature sets, capabilities, and references.",
  system: "Use for a concrete product, app, service, physical device, platform, or major component. System is the anchor level; use subsystems for deeper component breakdowns.",
  subsystem: "Use for a smaller component inside a system or another subsystem. Subsystems can nest before reaching feature sets or capabilities.",
  feature_set: "Use for a grouped set of related capabilities. Feature sets are structural containers, not final delivery items.",
  capability: "Use for something the product must be able to do. Capabilities can still contain finer capabilities, rollout slices, and references.",
  rollout: "Use for a delivery or release slice. Rollouts are leaves and should not contain deeper structural children.",
  reference: "Use for explanation, standards, constraints, notes, or source material. References are leaves and should not contain deeper structural children.",
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
  { label: "Scope, domains, and systems", kinds: ["area", "domain", "subdomain", "system", "subsystem"] },
  { label: "Product design", kinds: ["feature_set", "capability"] },
  { label: "Delivery and reference", kinds: ["rollout", "reference"] },
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
      return ["area", "domain", "system", "subsystem", "feature_set", "capability", "reference"];
    case "domain":
      return ["subdomain", "system", "subsystem", "feature_set", "capability", "reference"];
    case "subdomain":
      return ["subdomain", "system", "subsystem", "feature_set", "capability", "reference"];
    case "system":
      return ["subsystem", "feature_set", "capability", "reference"];
    case "subsystem":
      return ["subsystem", "feature_set", "capability", "reference"];
    case "feature_set":
      return ["feature_set", "capability", "rollout", "reference"];
    case "capability":
      return ["feature_set", "capability", "rollout", "reference"];
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

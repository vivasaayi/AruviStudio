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

export const ROOT_NODE_KINDS: HierarchyNodeKind[] = ["area", "domain", "system"];

export function getHierarchyNodeKindLabel(
  nodeKind: HierarchyNodeKind,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const labels = NODE_KIND_LABELS[nodeKind];
  if (options.lowercase) {
    return options.plural ? labels.pluralLower : labels.singularLower;
  }
  return options.plural ? labels.plural : labels.singular;
}

export function supportsHierarchyChildren(nodeKind: HierarchyNodeKind | null | undefined) {
  return Boolean(nodeKind && nodeKind !== "rollout" && nodeKind !== "reference");
}

export function getAllowedChildNodeKinds(parentKind: HierarchyNodeKind | null | undefined): HierarchyNodeKind[] {
  switch (parentKind) {
    case "area":
      return ["area", "domain", "system", "subsystem", "feature_set", "capability", "reference"];
    case "domain":
      return ["subdomain", "system", "subsystem", "feature_set", "capability", "reference"];
    case "subdomain":
      return ["subdomain", "feature_set", "capability", "reference"];
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

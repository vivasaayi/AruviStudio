type LabelForms = {
  singular: string;
  plural: string;
  singularLower: string;
  pluralLower: string;
};

const CAPABILITY_LABELS: LabelForms = {
  singular: "Capability",
  plural: "Capabilities",
  singularLower: "capability",
  pluralLower: "capabilities",
};

const ROLLOUT_LABELS: LabelForms = {
  singular: "Capability Rollout",
  plural: "Capability Rollouts",
  singularLower: "capability rollout",
  pluralLower: "capability rollouts",
};

function getLabelForms(level: number) {
  return level === 0 ? CAPABILITY_LABELS : ROLLOUT_LABELS;
}

export function getCapabilityHierarchyLabel(
  level: number,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  const labels = getLabelForms(level);
  if (options.lowercase) {
    return options.plural ? labels.pluralLower : labels.singularLower;
  }
  return options.plural ? labels.plural : labels.singular;
}

export function getCapabilityChildLabel(
  level: number,
  options: { plural?: boolean; lowercase?: boolean } = {},
) {
  if (level === 0) {
    return getCapabilityHierarchyLabel(1, options);
  }

  if (options.lowercase) {
    return options.plural ? "children" : "child";
  }

  return options.plural ? "Children" : "Child";
}

export function isCapabilityRolloutLevel(level: number) {
  return level > 0;
}

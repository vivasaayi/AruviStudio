import type {
  AgentDefinition,
  AgentTeam,
  Capability,
  Product,
  Skill,
  TeamAssignment,
  WorkflowStagePolicy,
} from "../../../lib/types";

export type AgentTab = "agents" | "teams" | "assignments" | "skills" | "routing";

export type AgentDraft = {
  name: string;
  role: string;
  description: string;
  promptTemplateRef: string;
  allowedTools: string;
  skillTags: string;
  boundaries: string;
  enabled: boolean;
  employmentStatus: "active" | "inactive" | "terminated";
};

export type TeamDraft = {
  name: string;
  department: string;
  description: string;
  enabled: boolean;
  maxConcurrentWorkflows: number;
};

export type SkillDraft = {
  name: string;
  category: string;
  description: string;
  instructions: string;
  enabled: boolean;
};

export type RoutingDraft = {
  stageName: string;
  primaryRoles: string;
  fallbackRoles: string;
  coordinatorRequired: boolean;
};

export const blankAgentDraft = (): AgentDraft => ({
  name: "",
  role: "developer",
  description: "",
  promptTemplateRef: "",
  allowedTools: "",
  skillTags: "",
  boundaries: "{}",
  enabled: true,
  employmentStatus: "active",
});

export const blankTeamDraft = (): TeamDraft => ({
  name: "",
  department: "engineering",
  description: "",
  enabled: true,
  maxConcurrentWorkflows: 2,
});

export const blankSkillDraft = (): SkillDraft => ({
  name: "",
  category: "general",
  description: "",
  instructions: "",
  enabled: true,
});

export const workflowStageOptions = [
  "requirement_analysis",
  "planning",
  "coding",
  "unit_test_generation",
  "integration_test_generation",
  "ui_test_planning",
  "qa_validation",
  "security_review",
  "performance_review",
  "push_preparation",
  "git_push",
];

export function parseAgentDraft(agent: AgentDefinition): AgentDraft {
  return {
    name: agent.name,
    role: agent.role,
    description: agent.description,
    promptTemplateRef: agent.prompt_template_ref,
    allowedTools: agent.allowed_tools.join(", "),
    skillTags: agent.skill_tags.join(", "),
    boundaries: JSON.stringify(agent.boundaries ?? {}, null, 2),
    enabled: agent.enabled,
    employmentStatus: agent.employment_status,
  };
}

export function parseTeamDraft(team: AgentTeam): TeamDraft {
  return {
    name: team.name,
    department: team.department,
    description: team.description,
    enabled: team.enabled,
    maxConcurrentWorkflows: team.max_concurrent_workflows,
  };
}

export function parseSkillDraft(skill: Skill): SkillDraft {
  return {
    name: skill.name,
    category: skill.category,
    description: skill.description,
    instructions: skill.instructions,
    enabled: skill.enabled,
  };
}

export function parsePolicyDraft(policy?: WorkflowStagePolicy | null, fallbackStage = workflowStageOptions[0]): RoutingDraft {
  return {
    stageName: policy?.stage_name ?? fallbackStage,
    primaryRoles: policy?.primary_roles.join(", ") ?? "",
    fallbackRoles: policy?.fallback_roles.join(", ") ?? "",
    coordinatorRequired: policy?.coordinator_required ?? true,
  };
}

export function countAssignmentsByType(assignments: TeamAssignment[]) {
  return assignments.reduce(
    (acc, assignment) => {
      acc[assignment.scope_type] += 1;
      return acc;
    },
    { product: 0, product_area: 0, capability: 0 } as Record<"product" | "product_area" | "capability", number>,
  );
}

export function formatCapabilityOptionName(capability: Capability) {
  return `${"  ".repeat(Math.max(0, capability.level))}${capability.name}`;
}

export function resolveScopeLabel(
  assignment: TeamAssignment,
  products: Product[],
  product_areas: Array<{ id: string; name: string }>,
  capabilities: Array<{ id: string; name: string }>,
) {
  if (assignment.scope_type === "product") {
    return products.find((product) => product.id === assignment.scope_id)?.name ?? assignment.scope_id;
  }
  if (assignment.scope_type === "product_area") {
    return product_areas.find((product_area) => product_area.id === assignment.scope_id)?.name ?? assignment.scope_id;
  }
  return capabilities.find((capability) => capability.id === assignment.scope_id)?.name.trim() ?? assignment.scope_id;
}

export function formatUiError(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return "Unknown error";
  }
}

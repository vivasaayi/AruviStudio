import type {
  AgentDefinition,
  AgentModelBinding,
  AgentTeam,
  AgentTeamMembership,
  ModelDefinition,
  ModelProvider,
  Repository,
  TeamAssignment,
  WorkItem,
  WorkflowStagePolicy,
} from "../../../lib/types";

export type WorkItemWorkflowReadiness = {
  blockers: string[];
  warnings: string[];
  checks: string[];
};

export type WorkItemWorkflowReadinessInput = {
  selectedWorkItem: WorkItem | null | undefined;
  teamAssignments: TeamAssignment[] | null | undefined;
  agentTeams: AgentTeam[] | null | undefined;
  teamMemberships: AgentTeamMembership[] | null | undefined;
  agentDefinitions: AgentDefinition[] | null | undefined;
  workflowPolicies: WorkflowStagePolicy[] | null | undefined;
  modelBindings: AgentModelBinding[] | null | undefined;
  modelDefinitions: ModelDefinition[] | null | undefined;
  providers: ModelProvider[] | null | undefined;
  resolvedRepository: Repository | null | undefined;
};

export function buildWorkItemWorkflowReadiness({
  selectedWorkItem,
  teamAssignments,
  agentTeams,
  teamMemberships,
  agentDefinitions,
  workflowPolicies,
  modelBindings,
  modelDefinitions,
  providers,
  resolvedRepository,
}: WorkItemWorkflowReadinessInput): WorkItemWorkflowReadiness {
  const blockers: string[] = [];
  const warnings: string[] = [];
  const checks: string[] = [];

  if (!selectedWorkItem) {
    return { blockers: ["Select a story to evaluate readiness."], warnings, checks };
  }

  if (selectedWorkItem.status !== "approved") {
    blockers.push("Work item status must be Approved before starting workflow.");
  } else {
    checks.push("Work item is approved.");
  }

  if (!resolvedRepository) {
    blockers.push("No workspace is attached to this story scope. Create a local workspace before starting delivery stages.");
  } else {
    checks.push(`Workspace resolved: ${resolvedRepository.name}.`);
    checks.push(`Branch resolved: ${selectedWorkItem.branch_name || resolvedRepository.default_branch}.`);
    if (!resolvedRepository.remote_url) {
      warnings.push("Workspace has no remote configured. Local-only delivery is fine, but push stages will remain local until a remote is added.");
    }
  }

  const assignmentMatch = (teamAssignments ?? []).find((assignment) => {
    if (assignment.scope_type === "capability" && selectedWorkItem.capability_id) {
      return assignment.scope_id === selectedWorkItem.capability_id;
    }
    if (assignment.scope_type === "product_area" && selectedWorkItem.product_area_id) {
      return assignment.scope_id === selectedWorkItem.product_area_id;
    }
    if (assignment.scope_type === "product") {
      return assignment.scope_id === selectedWorkItem.product_id;
    }
    return false;
  });

  const matchedTeam = assignmentMatch
    ? (agentTeams ?? []).find((team) => team.id === assignmentMatch.team_id)
    : null;

  if (!matchedTeam) {
    warnings.push("No team assignment found for capability/product_area/product scope. Fallback global agents will be used.");
  } else {
    checks.push(`Team assignment resolved: ${matchedTeam.name}.`);
    if (!matchedTeam.enabled) {
      blockers.push(`Assigned team "${matchedTeam.name}" is disabled.`);
    }
  }

  const activeAgents = (agentDefinitions ?? []).filter((agent) => agent.enabled && agent.employment_status === "active");
  if (activeAgents.length === 0) {
    blockers.push("No active agents are available.");
  } else {
    checks.push(`${activeAgents.length} active agents available.`);
  }

  const stagePolicy = (workflowPolicies ?? []).find((policy) => policy.stage_name === "requirement_analysis");
  const requiredRoles = stagePolicy
    ? [...stagePolicy.primary_roles, ...stagePolicy.fallback_roles]
    : ["manager", "architect", "analyst", "requirement_analysis"];
  const stageAgent = activeAgents.find((agent) =>
    requiredRoles.some((role) => role.toLowerCase() === agent.role.toLowerCase()),
  );

  if (!stageAgent) {
    blockers.push("No active agent matches requirement-analysis roles.");
  } else {
    checks.push(`Requirement-analysis agent ready: ${stageAgent.name} (${stageAgent.role}).`);
  }

  const stageAgentBinding = stageAgent
    ? (modelBindings ?? []).find((binding) => binding.agent_id === stageAgent.id)
    : null;
  const boundModel = stageAgentBinding
    ? (modelDefinitions ?? []).find((model) => model.id === stageAgentBinding.model_id)
    : null;
  const boundProvider = boundModel
    ? (providers ?? []).find((provider) => provider.id === boundModel.provider_id)
    : null;

  if (!stageAgentBinding || !boundModel) {
    blockers.push("Requirement-analysis agent has no model binding.");
  } else {
    checks.push(`Model binding resolved: ${boundModel.name}.`);
    if (!boundModel.enabled) {
      blockers.push(`Bound model "${boundModel.name}" is disabled.`);
    }
    if (!boundProvider) {
      blockers.push("Bound model provider is missing.");
    } else if (!boundProvider.enabled) {
      blockers.push(`Model provider "${boundProvider.name}" is disabled.`);
    } else {
      checks.push(`Provider ready: ${boundProvider.name}.`);
    }
  }

  const coordinatorRequired = stagePolicy ? stagePolicy.coordinator_required : true;
  if (coordinatorRequired) {
    if (!matchedTeam) {
      warnings.push("Coordinator review is enabled, but no team is assigned. Workflow will bypass coordinator stage.");
    } else {
      const teamMembers = (teamMemberships ?? []).filter((membership) => membership.team_id === matchedTeam.id);
      const hasCoordinator = teamMembers.some((membership) => {
        const memberAgent = (agentDefinitions ?? []).find((agent) => agent.id === membership.agent_id);
        if (!memberAgent || !memberAgent.enabled || memberAgent.employment_status !== "active") {
          return false;
        }
        const normalizedRole = memberAgent.role.toLowerCase();
        return membership.is_lead || normalizedRole === "manager" || normalizedRole === "team_lead";
      });
      if (!hasCoordinator) {
        warnings.push(`Coordinator review is enabled, but team "${matchedTeam.name}" has no active lead/manager.`);
      } else {
        checks.push("Coordinator available for review gates.");
      }
    }
  }

  return { blockers, warnings, checks };
}

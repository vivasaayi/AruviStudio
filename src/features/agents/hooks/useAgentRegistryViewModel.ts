import React from "react";
import { useQuery } from "@tanstack/react-query";
import {
  listAgentDefinitions,
  listAgentModelBindings,
  listAgentSkillLinks,
  listAgentTeams,
  listModelDefinitions,
  listProductAreas,
  listProductCapabilities,
  listProducts,
  listSkills,
  listTeamAssignments,
  listTeamMemberships,
  listTeamSkillLinks,
  listWorkflowStagePolicies,
} from "../../../lib/tauri";
import type { Capability, ProductArea } from "../../../lib/types";
import {
  buildCapabilityOptions,
  buildTeamMembershipsByTeam,
  countAssignmentsByType,
  filterUnassignedAgents,
  selectEntityById,
  selectPolicyByStage,
} from "../lib/agentRegistryPageModel";

type AgentRegistryViewModelInput = {
  selectedAgentId: string | null;
  selectedTeamId: string | null;
  selectedSkillId: string | null;
  selectedPolicyStage: string;
  assignmentProductId: string | null;
  assignmentScopeType: "product" | "product_area" | "capability";
};

export function useAgentRegistryViewModel({
  selectedAgentId,
  selectedTeamId,
  selectedSkillId,
  selectedPolicyStage,
  assignmentProductId,
  assignmentScopeType,
}: AgentRegistryViewModelInput) {
  const { data: agents = [], isLoading: agentsLoading } = useQuery({ queryKey: ["agents"], queryFn: listAgentDefinitions });
  const { data: agentModelBindings = [] } = useQuery({ queryKey: ["agent-model-bindings"], queryFn: listAgentModelBindings });
  const { data: modelDefinitions = [] } = useQuery({ queryKey: ["model-definitions"], queryFn: listModelDefinitions });
  const { data: teams = [], isLoading: teamsLoading } = useQuery({ queryKey: ["agent-teams"], queryFn: listAgentTeams });
  const { data: memberships = [] } = useQuery({ queryKey: ["agent-team-memberships"], queryFn: listTeamMemberships });
  const { data: assignments = [] } = useQuery({ queryKey: ["agent-team-assignments"], queryFn: listTeamAssignments });
  const { data: skills = [] } = useQuery({ queryKey: ["skills"], queryFn: listSkills });
  const { data: agentSkillLinks = [] } = useQuery({ queryKey: ["agent-skill-links"], queryFn: listAgentSkillLinks });
  const { data: teamSkillLinks = [] } = useQuery({ queryKey: ["team-skill-links"], queryFn: listTeamSkillLinks });
  const { data: routingPolicies = [] } = useQuery({ queryKey: ["workflow-stage-policies"], queryFn: listWorkflowStagePolicies });
  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: assignmentProductAreas = [] } = useQuery<ProductArea[]>({
    queryKey: ["agent-assignment-product-areas", assignmentProductId],
    queryFn: () => listProductAreas(assignmentProductId as string),
    enabled: Boolean(assignmentProductId),
  });
  const { data: assignmentCapabilities = [] } = useQuery<Capability[]>({
    queryKey: ["agent-assignment-capabilities", assignmentProductId],
    queryFn: () => listProductCapabilities(assignmentProductId as string),
    enabled: Boolean(assignmentProductId) && assignmentScopeType === "capability",
  });

  const selectedAgent = selectEntityById(agents, selectedAgentId);
  const selectedTeam = selectEntityById(teams, selectedTeamId);
  const selectedSkill = selectEntityById(skills, selectedSkillId);
  const selectedPolicy = selectPolicyByStage(routingPolicies, selectedPolicyStage);
  const currentCapabilityOptions = React.useMemo(
    () => buildCapabilityOptions(assignmentCapabilities),
    [assignmentCapabilities],
  );
  const selectedTeamMemberships = memberships.filter((membership) => membership.team_id === selectedTeamId);
  const selectedTeamAssignments = assignments.filter((assignment) => assignment.team_id === selectedTeamId);
  const assignmentCounts = countAssignmentsByType(assignments);
  const teamMembershipsByTeam = React.useMemo(
    () => buildTeamMembershipsByTeam(memberships),
    [memberships],
  );
  const unassignedAgents = React.useMemo(
    () => filterUnassignedAgents(agents, memberships),
    [agents, memberships],
  );

  return {
    agents,
    agentsLoading,
    agentModelBindings,
    modelDefinitions,
    teams,
    teamsLoading,
    memberships,
    assignments,
    skills,
    agentSkillLinks,
    teamSkillLinks,
    routingPolicies,
    products,
    currentProductAreaOptions: assignmentProductAreas,
    currentCapabilityOptions,
    selectedAgent,
    selectedTeam,
    selectedSkill,
    selectedPolicy,
    selectedTeamMemberships,
    selectedTeamAssignments,
    assignmentCounts,
    teamMembershipsByTeam,
    unassignedAgents,
  };
}

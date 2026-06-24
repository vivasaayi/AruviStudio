import type { Dispatch, SetStateAction } from "react";
import type { AgentDefinition, AgentTeam, Skill } from "../../../lib/types";
import {
  blankAgentDraft,
  blankSkillDraft,
  blankTeamDraft,
  formatUiError,
  parseAgentDraft,
  parsePolicyDraft,
  parseSkillDraft,
  parseTeamDraft,
  type AgentDraft,
  type AgentTab,
  type RoutingDraft,
  type SkillDraft,
  type TeamDraft,
} from "../lib/agentRegistryPageModel";
import type { useAgentRegistryMutations } from "./useAgentRegistryMutations";

type ScopeType = "product" | "product_area" | "capability";

type AgentRegistryPageActionsArgs = {
  agentDraft: AgentDraft;
  selectedAgent: AgentDefinition | null;
  selectedAgentId: string | null;
  selectedAgentSkillIds: string[];
  selectedAgentModelId: string;
  agentSkillLinks: Array<{ agent_id: string; skill_id: string }>;
  setSelectedAgentId: Dispatch<SetStateAction<string | null>>;
  setSelectedAgentSkillIds: Dispatch<SetStateAction<string[]>>;
  setSelectedAgentModelId: Dispatch<SetStateAction<string>>;
  setAgentDraft: Dispatch<SetStateAction<AgentDraft>>;
  setAgentError: Dispatch<SetStateAction<string | null>>;
  setAgentFeedback: Dispatch<SetStateAction<string | null>>;

  selectedTeam: AgentTeam | null;
  selectedTeamId: string | null;
  selectedTeamSkillIds: string[];
  teamDraft: TeamDraft;
  teamSkillLinks: Array<{ team_id: string; skill_id: string }>;
  setSelectedTeamId: Dispatch<SetStateAction<string | null>>;
  setSelectedTeamSkillIds: Dispatch<SetStateAction<string[]>>;
  setTeamDraft: Dispatch<SetStateAction<TeamDraft>>;
  setTeamError: Dispatch<SetStateAction<string | null>>;
  setTeamFeedback: Dispatch<SetStateAction<string | null>>;

  selectedSkill: Skill | null;
  skillDraft: SkillDraft;
  setSelectedSkillId: Dispatch<SetStateAction<string | null>>;
  setSkillDraft: Dispatch<SetStateAction<SkillDraft>>;
  setSkillError: Dispatch<SetStateAction<string | null>>;
  setSkillFeedback: Dispatch<SetStateAction<string | null>>;

  selectedPolicy: Parameters<typeof parsePolicyDraft>[0];
  selectedPolicyStage: string;
  routingDraft: RoutingDraft;
  setRoutingDraft: Dispatch<SetStateAction<RoutingDraft>>;
  setRoutingError: Dispatch<SetStateAction<string | null>>;
  setRoutingFeedback: Dispatch<SetStateAction<string | null>>;

  membershipDraft: { agentId: string; title: string; isLead: boolean };
  setMembershipError: Dispatch<SetStateAction<string | null>>;

  assignmentProductId: string | null;
  assignmentScopeType: ScopeType;
  assignmentProductAreaId: string;
  assignmentCapabilityId: string;
  setAssignmentProductId: Dispatch<SetStateAction<string | null>>;
  setAssignmentError: Dispatch<SetStateAction<string | null>>;
  setActiveProduct: (productId: string | null) => void;

  setActiveTab: Dispatch<SetStateAction<AgentTab>>;
  mutations: ReturnType<typeof useAgentRegistryMutations>;
};

export function useAgentRegistryPageActions({
  agentDraft,
  selectedAgent,
  selectedAgentId,
  selectedAgentSkillIds,
  selectedAgentModelId,
  agentSkillLinks,
  setSelectedAgentId,
  setSelectedAgentSkillIds,
  setSelectedAgentModelId,
  setAgentDraft,
  setAgentError,
  setAgentFeedback,
  selectedTeam,
  selectedTeamId,
  selectedTeamSkillIds,
  teamDraft,
  teamSkillLinks,
  setSelectedTeamId,
  setSelectedTeamSkillIds,
  setTeamDraft,
  setTeamError,
  setTeamFeedback,
  selectedSkill,
  skillDraft,
  setSelectedSkillId,
  setSkillDraft,
  setSkillError,
  setSkillFeedback,
  selectedPolicy,
  selectedPolicyStage,
  routingDraft,
  setRoutingDraft,
  setRoutingError,
  setRoutingFeedback,
  membershipDraft,
  setMembershipError,
  assignmentProductId,
  assignmentScopeType,
  assignmentProductAreaId,
  assignmentCapabilityId,
  setAssignmentProductId,
  setAssignmentError,
  setActiveProduct,
  setActiveTab,
  mutations,
}: AgentRegistryPageActionsArgs) {
  const handleSaveAgent = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    if (!agentDraft.name.trim() || !agentDraft.role.trim()) {
      setAgentError("Name and role are required.");
      return;
    }

    const payload = {
      name: agentDraft.name.trim(),
      role: agentDraft.role.trim(),
      description: agentDraft.description.trim(),
      promptTemplateRef: agentDraft.promptTemplateRef.trim(),
      allowedTools: agentDraft.allowedTools,
      skillTags: agentDraft.skillTags,
      boundaries: agentDraft.boundaries,
      enabled: agentDraft.enabled,
      employmentStatus: agentDraft.employmentStatus,
    } as const;

    try {
      let savedAgent: AgentDefinition;
      if (selectedAgent) {
        savedAgent = await mutations.updateAgentMutation.mutateAsync({
          id: selectedAgent.id,
          ...payload,
        });
      } else {
        savedAgent = await mutations.createAgentMutation.mutateAsync(payload);
      }
      setSelectedAgentId(savedAgent.id);
      await mutations.syncAgentSkills(savedAgent.id, selectedAgentSkillIds);
    } catch {
      // Mutation handlers already set error state.
    }
  };

  const handleBindSelectedAgentModel = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    if (!selectedAgent) {
      setAgentError("Select an agent first.");
      return;
    }
    if (!selectedAgentModelId) {
      setAgentError("Select a model definition first.");
      return;
    }
    try {
      await mutations.bindAgentModelMutation.mutateAsync({
        agentId: selectedAgent.id,
        modelId: selectedAgentModelId,
      });
    } catch {
      // Mutation handler sets feedback.
    }
  };

  const handleSaveTeam = async () => {
    setTeamError(null);
    setTeamFeedback(null);
    if (!teamDraft.name.trim()) {
      setTeamError("Team name is required.");
      return;
    }

    const payload = {
      name: teamDraft.name.trim(),
      department: teamDraft.department.trim(),
      description: teamDraft.description.trim(),
      enabled: teamDraft.enabled,
      maxConcurrentWorkflows: Math.max(1, teamDraft.maxConcurrentWorkflows),
    } as const;

    try {
      let savedTeam: AgentTeam;
      if (selectedTeam) {
        savedTeam = await mutations.updateTeamMutation.mutateAsync({
          id: selectedTeam.id,
          ...payload,
        });
      } else {
        savedTeam = await mutations.createTeamMutation.mutateAsync(payload);
      }
      setSelectedTeamId(savedTeam.id);
      await mutations.syncTeamSkills(savedTeam.id, selectedTeamSkillIds);
    } catch (error) {
      setTeamError(formatUiError(error));
      setTeamFeedback(null);
    }
  };

  const handleSaveSkill = async () => {
    setSkillError(null);
    setSkillFeedback(null);
    if (!skillDraft.name.trim()) {
      setSkillError("Skill name is required.");
      return;
    }

    const payload = {
      name: skillDraft.name.trim(),
      category: skillDraft.category.trim(),
      description: skillDraft.description.trim(),
      instructions: skillDraft.instructions,
      enabled: skillDraft.enabled,
    } as const;

    try {
      if (selectedSkill) {
        await mutations.updateSkillMutation.mutateAsync({
          id: selectedSkill.id,
          ...payload,
        });
      } else {
        await mutations.createSkillMutation.mutateAsync(payload);
      }
    } catch {
      // Mutation handlers already set error state.
    }
  };

  const handleSaveRoutingPolicy = async () => {
    setRoutingError(null);
    setRoutingFeedback(null);
    if (!routingDraft.stageName) {
      setRoutingError("Select a workflow stage.");
      return;
    }

    try {
      await mutations.upsertRoutingPolicyMutation.mutateAsync({
        stageName: routingDraft.stageName,
        primaryRoles: routingDraft.primaryRoles,
        fallbackRoles: routingDraft.fallbackRoles,
        coordinatorRequired: routingDraft.coordinatorRequired,
      });
    } catch {
      // Mutation handlers already set error state.
    }
  };

  const handleAddMembership = async () => {
    setMembershipError(null);
    if (!selectedTeamId) {
      setMembershipError("Select a team first.");
      return;
    }
    if (!membershipDraft.agentId) {
      setMembershipError("Choose an agent to add.");
      return;
    }
    try {
      await mutations.addMembershipMutation.mutateAsync({
        teamId: selectedTeamId,
        agentId: membershipDraft.agentId,
        title: membershipDraft.title.trim() || "Member",
        isLead: membershipDraft.isLead,
      });
    } catch {
      // Mutation handlers already set error state.
    }
  };

  const handleAssignScope = async () => {
    setAssignmentError(null);
    if (!selectedTeamId) {
      setAssignmentError("Select a team first.");
      return;
    }
    let scopeId = assignmentProductId ?? "";
    if (assignmentScopeType === "product_area") {
      scopeId = assignmentProductAreaId;
    }
    if (assignmentScopeType === "capability") {
      scopeId = assignmentCapabilityId;
    }
    if (!scopeId) {
      setAssignmentError("Choose a scope before assigning.");
      return;
    }
    try {
      await mutations.assignScopeMutation.mutateAsync({
        teamId: selectedTeamId,
        scopeType: assignmentScopeType,
        scopeId,
      });
    } catch {
      // Mutation handlers already set error state.
    }
  };

  const hireAgent = () => {
    setSelectedAgentId(null);
    setSelectedAgentSkillIds([]);
    setSelectedAgentModelId("");
    setAgentDraft(blankAgentDraft());
    setAgentError(null);
    setAgentFeedback(null);
  };

  const resetAgentForm = () => {
    if (selectedAgent) {
      setAgentDraft(parseAgentDraft(selectedAgent));
      setSelectedAgentSkillIds(
        agentSkillLinks
          .filter((link) => link.agent_id === selectedAgent.id)
          .map((link) => link.skill_id),
      );
    } else {
      setAgentDraft(blankAgentDraft());
      setSelectedAgentSkillIds([]);
    }
    setAgentError(null);
    setAgentFeedback(null);
  };

  const createNewTeam = () => {
    setSelectedTeamId(null);
    setSelectedTeamSkillIds([]);
    setTeamDraft(blankTeamDraft());
    setTeamError(null);
    setTeamFeedback(null);
  };

  const resetTeamForm = () => {
    if (selectedTeam) {
      setTeamDraft(parseTeamDraft(selectedTeam));
      setSelectedTeamSkillIds(
        teamSkillLinks
          .filter((link) => link.team_id === selectedTeam.id)
          .map((link) => link.skill_id),
      );
    } else {
      setTeamDraft(blankTeamDraft());
      setSelectedTeamSkillIds([]);
    }
    setTeamError(null);
    setTeamFeedback(null);
  };

  const createNewSkill = () => {
    setSelectedSkillId(null);
    setSkillDraft(blankSkillDraft());
    setSkillError(null);
    setSkillFeedback(null);
  };

  const resetSkillForm = () => {
    if (selectedSkill) {
      setSkillDraft(parseSkillDraft(selectedSkill));
    } else {
      setSkillDraft(blankSkillDraft());
    }
    setSkillError(null);
    setSkillFeedback(null);
  };

  const resetRoutingForm = () => {
    setRoutingDraft(parsePolicyDraft(selectedPolicy, selectedPolicyStage));
    setRoutingError(null);
    setRoutingFeedback(null);
  };

  const selectTeamFromAgentTab = (teamId: string) => {
    setSelectedTeamId(teamId);
    setSelectedAgentId(null);
  };

  const editTeamFromAgentTab = (teamId: string) => {
    setSelectedTeamId(teamId);
    setSelectedAgentId(null);
    setActiveTab("teams");
  };

  const selectAgentFromTeam = (teamId: string | null, agentId: string) => {
    setSelectedTeamId(teamId);
    setSelectedAgentId(agentId);
  };

  const changeAssignmentProduct = (nextId: string | null) => {
    setAssignmentProductId(nextId);
    setActiveProduct(nextId);
  };

  return {
    changeAssignmentProduct,
    createNewSkill,
    createNewTeam,
    editTeamFromAgentTab,
    handleAddMembership,
    handleAssignScope,
    handleBindSelectedAgentModel,
    handleSaveAgent,
    handleSaveRoutingPolicy,
    handleSaveSkill,
    handleSaveTeam,
    hireAgent,
    resetAgentForm,
    resetRoutingForm,
    resetSkillForm,
    resetTeamForm,
    selectAgentFromTeam,
    selectTeamFromAgentTab,
  };
}

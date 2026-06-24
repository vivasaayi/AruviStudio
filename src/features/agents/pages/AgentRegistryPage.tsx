import { useQueryClient } from "@tanstack/react-query";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { AgentRegistryHeader } from "../components/AgentRegistryHeader";
import { AgentRegistryTabPanels } from "../components/AgentRegistryTabPanels";
import { useAgentModelBindingActions } from "../hooks/useAgentModelBindingActions";
import { useAgentRegistryMutations } from "../hooks/useAgentRegistryMutations";
import { useAgentRegistryPageActions } from "../hooks/useAgentRegistryPageActions";
import { useAgentRegistryPageState } from "../hooks/useAgentRegistryPageState";
import { useAgentRegistryPageSync } from "../hooks/useAgentRegistryPageSync";
import { useAgentRegistryViewModel } from "../hooks/useAgentRegistryViewModel";
import { styles } from "../lib/agentRegistryPageStyles";

export function AgentRegistryPage() {
  const queryClient = useQueryClient();
  const { activeProductId, setActiveProduct } = useWorkspaceStore();
  const pageState = useAgentRegistryPageState(activeProductId);
  const {
    activeTab,
    setActiveTab,
    selectedAgentId,
    setSelectedAgentId,
    selectedAgentSkillIds,
    setSelectedAgentSkillIds,
    selectedAgentModelId,
    setSelectedAgentModelId,
    agentDraft,
    setAgentDraft,
    agentFeedback,
    setAgentFeedback,
    agentError,
    setAgentError,
    selectedTeamId,
    setSelectedTeamId,
    selectedTeamSkillIds,
    setSelectedTeamSkillIds,
    teamDraft,
    setTeamDraft,
    teamFeedback,
    setTeamFeedback,
    teamError,
    setTeamError,
    selectedSkillId,
    setSelectedSkillId,
    skillDraft,
    setSkillDraft,
    skillFeedback,
    setSkillFeedback,
    skillError,
    setSkillError,
    selectedPolicyStage,
    setSelectedPolicyStage,
    routingDraft,
    setRoutingDraft,
    routingFeedback,
    setRoutingFeedback,
    routingError,
    setRoutingError,
    membershipDraft,
    setMembershipDraft,
    membershipError,
    setMembershipError,
    assignmentProductId,
    setAssignmentProductId,
    assignmentScopeType,
    setAssignmentScopeType,
    assignmentProductAreaId,
    setAssignmentProductAreaId,
    assignmentCapabilityId,
    setAssignmentCapabilityId,
    assignmentError,
    setAssignmentError,
  } = pageState;

  const viewModel = useAgentRegistryViewModel({
    selectedAgentId,
    selectedTeamId,
    selectedSkillId,
    selectedPolicyStage,
    assignmentProductId,
    assignmentScopeType,
  });
  const {
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
    currentProductAreaOptions,
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
  } = viewModel;

  useAgentRegistryPageSync({
    activeProductId,
    assignmentProductId,
    setAssignmentProductId,
    agents,
    selectedAgentId,
    setSelectedAgentId,
    selectedAgent,
    agentSkillLinks,
    setSelectedAgentSkillIds,
    setAgentDraft,
    setAgentError,
    setAgentFeedback,
    agentModelBindings,
    setSelectedAgentModelId,
    teams,
    selectedTeamId,
    setSelectedTeamId,
    selectedTeam,
    teamSkillLinks,
    setSelectedTeamSkillIds,
    setTeamDraft,
    setTeamError,
    setTeamFeedback,
    skills,
    selectedSkillId,
    setSelectedSkillId,
    selectedSkill,
    setSkillDraft,
    setSkillError,
    setSkillFeedback,
    selectedPolicy,
    selectedPolicyStage,
    setRoutingDraft,
    setRoutingError,
    setRoutingFeedback,
    currentProductAreaOptions,
    assignmentProductAreaId,
    setAssignmentProductAreaId,
    currentCapabilityOptions,
    assignmentCapabilityId,
    setAssignmentCapabilityId,
  });

  const mutations = useAgentRegistryMutations({
    queryClient,
    agents,
    teams,
    skills,
    agentSkillLinks,
    teamSkillLinks,
    setSelectedAgentId,
    setSelectedAgentSkillIds,
    setAgentDraft,
    setAgentFeedback,
    setAgentError,
    setSelectedTeamId,
    setSelectedTeamSkillIds,
    setTeamDraft,
    setTeamFeedback,
    setTeamError,
    setSelectedSkillId,
    setSkillDraft,
    setSkillFeedback,
    setSkillError,
    setSelectedPolicyStage,
    setRoutingDraft,
    setRoutingFeedback,
    setRoutingError,
    setMembershipDraft,
    setMembershipError,
    setAssignmentError,
  });

  const {
    bindAllAgentsToDeepSeek,
    bindCodingAgentsToDeepSeek,
  } = useAgentModelBindingActions({
    agents,
    modelDefinitions,
    bindAgentModelMutation: mutations.bindAgentModelMutation,
    setAgentFeedback,
    setAgentError,
  });

  const actions = useAgentRegistryPageActions({
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
  });

  return (
    <div style={styles.page}>
      <AgentRegistryHeader
        activeTab={activeTab}
        agentCount={agents.length}
        teamCount={teams.length}
        assignmentCount={assignments.length}
        skillCount={skills.length}
        onActiveTabChange={setActiveTab}
      />

      <AgentRegistryTabPanels
        pageState={pageState}
        viewModel={viewModel}
        actions={actions}
        mutations={mutations}
        onBindAllAgentsToDeepSeek={bindAllAgentsToDeepSeek}
        onBindCodingAgentsToDeepSeek={bindCodingAgentsToDeepSeek}
      />
    </div>
  );
}

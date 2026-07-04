import { AgentAssignmentsTab } from "./AgentAssignmentsTab";
import { AgentProfileTab } from "./AgentProfileTab";
import { AgentRoutingTab } from "./AgentRoutingTab";
import { AgentSkillsTab } from "./AgentSkillsTab";
import { AgentTeamsTab } from "./AgentTeamsTab";
import type { useAgentRegistryMutations } from "../hooks/useAgentRegistryMutations";
import type { useAgentRegistryPageActions } from "../hooks/useAgentRegistryPageActions";
import type { useAgentRegistryPageState } from "../hooks/useAgentRegistryPageState";
import type { useAgentRegistryViewModel } from "../hooks/useAgentRegistryViewModel";

type AgentRegistryTabPanelsProps = {
  pageState: ReturnType<typeof useAgentRegistryPageState>;
  viewModel: ReturnType<typeof useAgentRegistryViewModel>;
  actions: ReturnType<typeof useAgentRegistryPageActions>;
  mutations: ReturnType<typeof useAgentRegistryMutations>;
  onBindAllAgentsToDeepSeek: () => void;
  onBindCodingAgentsToDeepSeek: () => void;
};

export function AgentRegistryTabPanels({
  pageState,
  viewModel,
  actions,
  mutations,
  onBindAllAgentsToDeepSeek,
  onBindCodingAgentsToDeepSeek,
}: AgentRegistryTabPanelsProps) {
  const {
    activeTab,
    expandedTeams,
    setExpandedTeams,
    selectedAgentId,
    setSelectedAgentId,
    selectedAgentSkillIds,
    setSelectedAgentSkillIds,
    selectedAgentModelId,
    setSelectedAgentModelId,
    agentDraft,
    setAgentDraft,
    agentFeedback,
    agentError,
    selectedTeamId,
    setSelectedTeamId,
    selectedTeamSkillIds,
    setSelectedTeamSkillIds,
    teamDraft,
    setTeamDraft,
    teamFeedback,
    teamError,
    selectedSkillId,
    setSelectedSkillId,
    skillDraft,
    setSkillDraft,
    skillFeedback,
    skillError,
    selectedPolicyStage,
    setSelectedPolicyStage,
    routingDraft,
    setRoutingDraft,
    routingFeedback,
    routingError,
    membershipDraft,
    setMembershipDraft,
    membershipError,
    assignmentProductId,
    assignmentScopeType,
    setAssignmentScopeType,
    assignmentProductAreaId,
    setAssignmentProductAreaId,
    assignmentCapabilityId,
    setAssignmentCapabilityId,
    assignmentError,
  } = pageState;

  const {
    agents,
    agentsLoading,
    agentModelBindings: _agentModelBindings,
    modelDefinitions,
    teams,
    teamsLoading,
    memberships,
    assignments: _assignments,
    skills,
    agentSkillLinks: _agentSkillLinks,
    teamSkillLinks: _teamSkillLinks,
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
    assignmentCounts: _assignmentCounts,
    teamMembershipsByTeam,
    unassignedAgents,
  } = viewModel;

  if (activeTab === "agents") {
    return (
      <AgentProfileTab
        agents={agents}
        teams={teams}
        skills={skills}
        modelDefinitions={modelDefinitions}
        agentsLoading={agentsLoading}
        teamsLoading={teamsLoading}
        selectedAgentId={selectedAgentId}
        selectedTeamId={selectedTeamId}
        selectedAgent={selectedAgent}
        expandedTeams={expandedTeams}
        teamMembershipsByTeam={teamMembershipsByTeam}
        unassignedAgents={unassignedAgents}
        agentDraft={agentDraft}
        selectedAgentSkillIds={selectedAgentSkillIds}
        selectedAgentModelId={selectedAgentModelId}
        agentError={agentError}
        agentFeedback={agentFeedback}
        onHireAgent={actions.hireAgent}
        onBindAllAgentsToDeepSeek={onBindAllAgentsToDeepSeek}
        onBindCodingAgentsToDeepSeek={onBindCodingAgentsToDeepSeek}
        onExpandedTeamsChange={setExpandedTeams}
        onSelectTeam={actions.selectTeamFromAgentTab}
        onEditTeam={actions.editTeamFromAgentTab}
        onSelectAgent={actions.selectAgentFromTeam}
        onUnassignMembership={(teamId, membershipId) => {
          setSelectedTeamId(teamId);
          mutations.removeMembershipMutation.mutate(membershipId);
        }}
        onDeleteAgent={(agentId) => mutations.deleteAgentMutation.mutate(agentId)}
        onAgentDraftChange={setAgentDraft}
        onSelectedAgentSkillIdsChange={setSelectedAgentSkillIds}
        onSelectedAgentModelChange={setSelectedAgentModelId}
        onBindSelectedAgentModel={actions.handleBindSelectedAgentModel}
        onSaveAgent={actions.handleSaveAgent}
        onResetAgentForm={actions.resetAgentForm}
      />
    );
  }

  if (activeTab === "teams") {
    return (
      <AgentTeamsTab
        agents={agents}
        teams={teams}
        skills={skills}
        teamsLoading={teamsLoading}
        memberships={memberships}
        selectedTeamId={selectedTeamId}
        selectedTeam={selectedTeam}
        teamDraft={teamDraft}
        selectedTeamSkillIds={selectedTeamSkillIds}
        selectedTeamMemberships={selectedTeamMemberships}
        selectedTeamAssignments={selectedTeamAssignments}
        membershipDraft={membershipDraft}
        membershipError={membershipError}
        teamError={teamError}
        teamFeedback={teamFeedback}
        isCreateTeamPending={mutations.createTeamMutation.isPending}
        isUpdateTeamPending={mutations.updateTeamMutation.isPending}
        onCreateNewTeam={actions.createNewTeam}
        onSelectTeam={setSelectedTeamId}
        onTeamDraftChange={setTeamDraft}
        onSelectedTeamSkillIdsChange={setSelectedTeamSkillIds}
        onMembershipDraftChange={setMembershipDraft}
        onSaveTeam={actions.handleSaveTeam}
        onResetTeamForm={actions.resetTeamForm}
        onDeleteTeam={(teamId) => mutations.deleteTeamMutation.mutate(teamId)}
        onRemoveMembership={(membershipId) => mutations.removeMembershipMutation.mutate(membershipId)}
        onAddMembership={actions.handleAddMembership}
      />
    );
  }

  if (activeTab === "assignments") {
    return (
      <AgentAssignmentsTab
        teams={teams}
        products={products}
        selectedTeamId={selectedTeamId}
        assignmentProductId={assignmentProductId}
        assignmentScopeType={assignmentScopeType}
        assignmentProductAreaId={assignmentProductAreaId}
        assignmentCapabilityId={assignmentCapabilityId}
        currentProductAreaOptions={currentProductAreaOptions}
        currentCapabilityOptions={currentCapabilityOptions}
        selectedTeamAssignments={selectedTeamAssignments}
        assignmentError={assignmentError}
        onSelectedTeamChange={setSelectedTeamId}
        onAssignmentProductChange={actions.changeAssignmentProduct}
        onAssignmentScopeTypeChange={setAssignmentScopeType}
        onAssignmentProductAreaChange={setAssignmentProductAreaId}
        onAssignmentCapabilityChange={setAssignmentCapabilityId}
        onAssignScope={actions.handleAssignScope}
        onRemoveAssignment={(assignmentId) => mutations.removeAssignmentMutation.mutate(assignmentId)}
      />
    );
  }

  if (activeTab === "skills") {
    return (
      <AgentSkillsTab
        skills={skills}
        selectedSkillId={selectedSkillId}
        selectedSkill={selectedSkill}
        skillDraft={skillDraft}
        onCreateNewSkill={actions.createNewSkill}
        onSelectSkill={setSelectedSkillId}
        onSkillDraftChange={setSkillDraft}
        onDeleteSkill={(skillId) => mutations.deleteSkillMutation.mutate(skillId)}
        onSaveSkill={actions.handleSaveSkill}
        onResetSkillForm={actions.resetSkillForm}
        skillError={skillError}
        skillFeedback={skillFeedback}
      />
    );
  }

  if (activeTab === "routing") {
    return (
      <AgentRoutingTab
        routingPolicies={routingPolicies}
        selectedPolicyStage={selectedPolicyStage}
        selectedPolicy={selectedPolicy}
        routingDraft={routingDraft}
        onSelectedPolicyStageChange={setSelectedPolicyStage}
        onRoutingDraftChange={setRoutingDraft}
        onDeleteRoutingPolicy={(stageName) => mutations.deleteRoutingPolicyMutation.mutate(stageName)}
        onSaveRoutingPolicy={actions.handleSaveRoutingPolicy}
        onResetRoutingForm={actions.resetRoutingForm}
        routingError={routingError}
        routingFeedback={routingFeedback}
      />
    );
  }

  return null;
}

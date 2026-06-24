import React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { AgentAssignmentsTab } from "../components/AgentAssignmentsTab";
import { AgentProfileTab } from "../components/AgentProfileTab";
import { AgentRegistryHeader } from "../components/AgentRegistryHeader";
import { AgentRoutingTab } from "../components/AgentRoutingTab";
import { AgentSkillsTab } from "../components/AgentSkillsTab";
import { AgentTeamsTab } from "../components/AgentTeamsTab";
import { useAgentModelBindingActions } from "../hooks/useAgentModelBindingActions";
import { useAgentRegistryMutations } from "../hooks/useAgentRegistryMutations";
import { useAgentRegistryPageActions } from "../hooks/useAgentRegistryPageActions";
import { useAgentRegistryPageSync } from "../hooks/useAgentRegistryPageSync";
import { useAgentRegistryViewModel } from "../hooks/useAgentRegistryViewModel";
import { styles } from "../lib/agentRegistryPageStyles";
import {
  blankAgentDraft,
  blankSkillDraft,
  blankTeamDraft,
  parsePolicyDraft,
  workflowStageOptions,
  type AgentDraft,
  type AgentTab,
  type RoutingDraft,
  type SkillDraft,
  type TeamDraft,
} from "../lib/agentRegistryPageModel";

export function AgentRegistryPage() {
  const queryClient = useQueryClient();
  const { activeProductId, setActiveProduct } = useWorkspaceStore();

  const [activeTab, setActiveTab] = React.useState<AgentTab>("agents");
  const [expandedTeams, setExpandedTeams] = React.useState<Record<string, boolean>>({});

  const [selectedAgentId, setSelectedAgentId] = React.useState<string | null>(null);
  const [selectedAgentSkillIds, setSelectedAgentSkillIds] = React.useState<string[]>([]);
  const [selectedAgentModelId, setSelectedAgentModelId] = React.useState<string>("");
  const [agentDraft, setAgentDraft] = React.useState<AgentDraft>(blankAgentDraft);
  const [agentFeedback, setAgentFeedback] = React.useState<string | null>(null);
  const [agentError, setAgentError] = React.useState<string | null>(null);

  const [selectedTeamId, setSelectedTeamId] = React.useState<string | null>(null);
  const [selectedTeamSkillIds, setSelectedTeamSkillIds] = React.useState<string[]>([]);
  const [teamDraft, setTeamDraft] = React.useState<TeamDraft>(blankTeamDraft);
  const [teamFeedback, setTeamFeedback] = React.useState<string | null>(null);
  const [teamError, setTeamError] = React.useState<string | null>(null);

  const [selectedSkillId, setSelectedSkillId] = React.useState<string | null>(null);
  const [skillDraft, setSkillDraft] = React.useState<SkillDraft>(blankSkillDraft);
  const [skillFeedback, setSkillFeedback] = React.useState<string | null>(null);
  const [skillError, setSkillError] = React.useState<string | null>(null);
  const [selectedPolicyStage, setSelectedPolicyStage] = React.useState<string>(workflowStageOptions[0]);
  const [routingDraft, setRoutingDraft] = React.useState<RoutingDraft>(parsePolicyDraft(null));
  const [routingFeedback, setRoutingFeedback] = React.useState<string | null>(null);
  const [routingError, setRoutingError] = React.useState<string | null>(null);

  const [membershipDraft, setMembershipDraft] = React.useState({ agentId: "", title: "", isLead: false });
  const [membershipError, setMembershipError] = React.useState<string | null>(null);

  const [assignmentProductId, setAssignmentProductId] = React.useState<string | null>(activeProductId);
  const [assignmentScopeType, setAssignmentScopeType] = React.useState<"product" | "product_area" | "capability">("product_area");
  const [assignmentProductAreaId, setAssignmentProductAreaId] = React.useState<string>("");
  const [assignmentCapabilityId, setAssignmentCapabilityId] = React.useState<string>("");
  const [assignmentError, setAssignmentError] = React.useState<string | null>(null);

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
  } = useAgentRegistryViewModel({
    selectedAgentId,
    selectedTeamId,
    selectedSkillId,
    selectedPolicyStage,
    assignmentProductId,
    assignmentScopeType,
  });

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

  const renderAgentTab = () => (
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
      onBindAllAgentsToDeepSeek={bindAllAgentsToDeepSeek}
      onBindCodingAgentsToDeepSeek={bindCodingAgentsToDeepSeek}
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

  const renderTeamTab = () => (
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

  const renderAssignmentsTab = () => (
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

  const renderSkillsTab = () => (
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

  const renderRoutingTab = () => (
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

      {activeTab === "agents" ? renderAgentTab() : null}
      {activeTab === "teams" ? renderTeamTab() : null}
      {activeTab === "assignments" ? renderAssignmentsTab() : null}
      {activeTab === "skills" ? renderSkillsTab() : null}
      {activeTab === "routing" ? renderRoutingTab() : null}
    </div>
  );
}

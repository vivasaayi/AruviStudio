import React from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  addTeamMember,
  assignTeamScope,
  createAgentDefinition,
  createAgentTeam,
  createSkill,
  deleteAgentDefinition,
  deleteAgentTeam,
  deleteSkill,
  deleteWorkflowStagePolicy,
  linkSkillToAgent,
  linkSkillToTeam,
  removeTeamAssignment,
  removeTeamMember,
  unlinkSkillFromAgent,
  unlinkSkillFromTeam,
  setPrimaryAgentModelBinding,
  updateAgentDefinition,
  updateAgentTeam,
  updateSkill,
  upsertWorkflowStagePolicy,
} from "../../../lib/tauri";
import type {
  AgentDefinition,
  AgentTeam,
  Product,
  Skill,
} from "../../../lib/types";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { AgentAssignmentsTab } from "../components/AgentAssignmentsTab";
import { AgentProfileTab } from "../components/AgentProfileTab";
import { AgentRegistryHeader } from "../components/AgentRegistryHeader";
import { AgentRoutingTab } from "../components/AgentRoutingTab";
import { AgentSkillsTab } from "../components/AgentSkillsTab";
import { AgentTeamsTab } from "../components/AgentTeamsTab";
import { useAgentRegistryPageSync } from "../hooks/useAgentRegistryPageSync";
import { useAgentRegistryViewModel } from "../hooks/useAgentRegistryViewModel";
import { styles } from "../lib/agentRegistryPageStyles";
import {
  blankAgentDraft,
  blankSkillDraft,
  blankTeamDraft,
  formatUiError,
  parseAgentDraft,
  parsePolicyDraft,
  parseSkillDraft,
  parseTeamDraft,
  workflowStageOptions,
  type AgentDraft,
  type AgentTab,
  type RoutingDraft,
  type SkillDraft,
  type TeamDraft,
} from "../lib/agentRegistryPageModel";
import { invalidateAgentRegistryData } from "../lib/agentRegistryQueries";

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

  const invalidateAgentData = () => invalidateAgentRegistryData(queryClient);

  const createAgentMutation = useMutation({
    mutationFn: createAgentDefinition,
    onSuccess: async (agent) => {
      await invalidateAgentData();
      setSelectedAgentId(agent.id);
      setAgentDraft(parseAgentDraft(agent));
      setAgentFeedback("Agent hired and added to the registry.");
      setAgentError(null);
    },
    onError: (error) => {
      setAgentError(String(error));
      setAgentFeedback(null);
    },
  });

  const updateAgentMutation = useMutation({
    mutationFn: updateAgentDefinition,
    onSuccess: async (agent) => {
      await invalidateAgentData();
      setSelectedAgentId(agent.id);
      setAgentDraft(parseAgentDraft(agent));
      setAgentFeedback("Agent profile updated.");
      setAgentError(null);
    },
    onError: (error) => {
      setAgentError(String(error));
      setAgentFeedback(null);
    },
  });

  const deleteAgentMutation = useMutation({
    mutationFn: deleteAgentDefinition,
    onSuccess: async (_, deletedId) => {
      await invalidateAgentData();
      const nextAgent = agents.find((agent) => agent.id !== deletedId) ?? null;
      setSelectedAgentId(nextAgent?.id ?? null);
      setAgentDraft(nextAgent ? parseAgentDraft(nextAgent) : blankAgentDraft());
      setSelectedAgentSkillIds([]);
      setAgentFeedback("Agent removed from the registry.");
      setAgentError(null);
    },
    onError: (error) => {
      setAgentError(String(error));
      setAgentFeedback(null);
    },
  });

  const createTeamMutation = useMutation({
    mutationFn: createAgentTeam,
    onSuccess: async (team) => {
      queryClient.setQueryData<AgentTeam[] | undefined>(["agent-teams"], (current) =>
        current ? [...current, team].sort((a, b) => a.name.localeCompare(b.name)) : [team],
      );
      await invalidateAgentData();
      setSelectedTeamId(team.id);
      setTeamDraft(parseTeamDraft(team));
      setSelectedTeamSkillIds([]);
      setTeamFeedback("Team created.");
      setTeamError(null);
    },
    onError: (error) => {
      setTeamError(formatUiError(error));
      setTeamFeedback(null);
    },
  });

  const updateTeamMutation = useMutation({
    mutationFn: updateAgentTeam,
    onSuccess: async (team) => {
      await invalidateAgentData();
      setSelectedTeamId(team.id);
      setTeamDraft(parseTeamDraft(team));
      setTeamFeedback("Team updated.");
      setTeamError(null);
    },
    onError: (error) => {
      setTeamError(formatUiError(error));
      setTeamFeedback(null);
    },
  });

  const deleteTeamMutation = useMutation({
    mutationFn: deleteAgentTeam,
    onSuccess: async (_, deletedId) => {
      await invalidateAgentData();
      const nextTeam = teams.find((team) => team.id !== deletedId) ?? null;
      setSelectedTeamId(nextTeam?.id ?? null);
      setTeamDraft(nextTeam ? parseTeamDraft(nextTeam) : blankTeamDraft());
      setSelectedTeamSkillIds([]);
      setTeamFeedback("Team removed.");
      setTeamError(null);
    },
    onError: (error) => {
      setTeamError(formatUiError(error));
      setTeamFeedback(null);
    },
  });

  const createSkillMutation = useMutation({
    mutationFn: createSkill,
    onSuccess: async (skill) => {
      await invalidateAgentData();
      setSelectedSkillId(skill.id);
      setSkillDraft(parseSkillDraft(skill));
      setSkillFeedback("Skill added to the catalog.");
      setSkillError(null);
    },
    onError: (error) => {
      setSkillError(String(error));
      setSkillFeedback(null);
    },
  });

  const updateSkillMutation = useMutation({
    mutationFn: updateSkill,
    onSuccess: async (skill) => {
      await invalidateAgentData();
      setSelectedSkillId(skill.id);
      setSkillDraft(parseSkillDraft(skill));
      setSkillFeedback("Skill updated.");
      setSkillError(null);
    },
    onError: (error) => {
      setSkillError(String(error));
      setSkillFeedback(null);
    },
  });

  const deleteSkillMutation = useMutation({
    mutationFn: deleteSkill,
    onSuccess: async (_, deletedId) => {
      await invalidateAgentData();
      const nextSkill = skills.find((skill) => skill.id !== deletedId) ?? null;
      setSelectedSkillId(nextSkill?.id ?? null);
      setSkillDraft(nextSkill ? parseSkillDraft(nextSkill) : blankSkillDraft());
      setSkillFeedback("Skill removed from the catalog.");
      setSkillError(null);
    },
    onError: (error) => {
      setSkillError(String(error));
      setSkillFeedback(null);
    },
  });

  const upsertRoutingPolicyMutation = useMutation({
    mutationFn: upsertWorkflowStagePolicy,
    onSuccess: async (policy) => {
      await invalidateAgentData();
      setSelectedPolicyStage(policy.stage_name);
      setRoutingDraft(parsePolicyDraft(policy, policy.stage_name));
      setRoutingFeedback("Routing policy saved.");
      setRoutingError(null);
    },
    onError: (error) => {
      setRoutingError(String(error));
      setRoutingFeedback(null);
    },
  });

  const deleteRoutingPolicyMutation = useMutation({
    mutationFn: deleteWorkflowStagePolicy,
    onSuccess: async (_, deletedStage) => {
      await invalidateAgentData();
      setSelectedPolicyStage(deletedStage);
      setRoutingDraft(parsePolicyDraft(null, deletedStage));
      setRoutingFeedback("Routing policy reset to defaults.");
      setRoutingError(null);
    },
    onError: (error) => {
      setRoutingError(String(error));
      setRoutingFeedback(null);
    },
  });

  const bindAgentModelMutation = useMutation({
    mutationFn: (payload: { agentId: string; modelId: string }) => setPrimaryAgentModelBinding(payload),
    onSuccess: async () => {
      setAgentFeedback("Agent model binding updated.");
      setAgentError(null);
      await invalidateAgentData();
    },
    onError: (error: unknown) => {
      setAgentError(String(error));
      setAgentFeedback(null);
    },
  });

  const addMembershipMutation = useMutation({
    mutationFn: addTeamMember,
    onSuccess: async () => {
      await invalidateAgentData();
      setMembershipDraft({ agentId: "", title: "", isLead: false });
      setMembershipError(null);
    },
    onError: (error) => setMembershipError(String(error)),
  });

  const removeMembershipMutation = useMutation({
    mutationFn: removeTeamMember,
    onSuccess: invalidateAgentData,
    onError: (error) => setMembershipError(String(error)),
  });

  const assignScopeMutation = useMutation({
    mutationFn: assignTeamScope,
    onSuccess: async () => {
      await invalidateAgentData();
      setAssignmentError(null);
    },
    onError: (error) => setAssignmentError(String(error)),
  });

  const removeAssignmentMutation = useMutation({
    mutationFn: removeTeamAssignment,
    onSuccess: invalidateAgentData,
    onError: (error) => setAssignmentError(String(error)),
  });

  const syncAgentSkills = async (agentId: string, nextSkillIds: string[]) => {
    if (!agentId) {
      return;
    }
    const currentLinks = agentSkillLinks.filter((link) => link.agent_id === agentId);
    const currentIds = new Set(currentLinks.map((link) => link.skill_id));
    const nextIds = new Set(nextSkillIds);

    const removals = currentLinks.filter((link) => !nextIds.has(link.skill_id));
    const additions = nextSkillIds.filter((skillId) => !currentIds.has(skillId));

    await Promise.all([
      ...removals.map((link) => unlinkSkillFromAgent(link.id)),
      ...additions.map((skillId) =>
        linkSkillToAgent({
          agentId,
          skillId,
          proficiency: "working",
        }),
      ),
    ]);
    setSelectedAgentSkillIds(nextSkillIds);
    await invalidateAgentData();
  };

  const syncTeamSkills = async (teamId: string, nextSkillIds: string[]) => {
    if (!teamId) {
      return;
    }
    const currentLinks = teamSkillLinks.filter((link) => link.team_id === teamId);
    const currentIds = new Set(currentLinks.map((link) => link.skill_id));
    const nextIds = new Set(nextSkillIds);

    const removals = currentLinks.filter((link) => !nextIds.has(link.skill_id));
    const additions = nextSkillIds.filter((skillId) => !currentIds.has(skillId));

    await Promise.all([
      ...removals.map((link) => unlinkSkillFromTeam(link.id)),
      ...additions.map((skillId) =>
        linkSkillToTeam({
          teamId,
          skillId,
        }),
      ),
    ]);
    setSelectedTeamSkillIds(nextSkillIds);
    await invalidateAgentData();
  };

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
        savedAgent = await updateAgentMutation.mutateAsync({ id: selectedAgent.id, ...payload });
      } else {
        savedAgent = await createAgentMutation.mutateAsync(payload);
      }
      setSelectedAgentId(savedAgent.id);
      await syncAgentSkills(savedAgent.id, selectedAgentSkillIds);
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
      await bindAgentModelMutation.mutateAsync({
        agentId: selectedAgent.id,
        modelId: selectedAgentModelId,
      });
    } catch {
      // Mutation handler sets feedback.
    }
  };

  const handleBindCodingAgentsToDeepSeek = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    const deepSeekModel =
      modelDefinitions.find((model) => model.name.toLowerCase().includes("deepseek-coder")) ??
      modelDefinitions.find((model) => model.name.toLowerCase().includes("deepseek"));
    if (!deepSeekModel) {
      setAgentError("No DeepSeek model definition found. Add one in the Models tab first.");
      return;
    }

    const codingAgents = agents.filter((agent) => {
      const role = agent.role.toLowerCase();
      return agent.enabled && (role.includes("coding") || role.includes("developer"));
    });

    if (codingAgents.length === 0) {
      setAgentError("No enabled coding/developer agents found.");
      return;
    }

    try {
      await Promise.all(
        codingAgents.map((agent) =>
          bindAgentModelMutation.mutateAsync({
            agentId: agent.id,
            modelId: deepSeekModel.id,
          }),
        ),
      );
      setAgentFeedback(`Bound ${codingAgents.length} coding agents to ${deepSeekModel.name}.`);
    } catch {
      // Mutation handler sets feedback.
    }
  };

  const handleBindAllAgentsToDeepSeek = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    const deepSeekModel =
      modelDefinitions.find((model) => model.enabled && model.name.toLowerCase().includes("deepseek-coder")) ??
      modelDefinitions.find((model) => model.enabled && model.name.toLowerCase().includes("deepseek"));
    if (!deepSeekModel) {
      setAgentError("No DeepSeek model definition found. Add one in the Models tab first.");
      return;
    }

    const enabledAgents = agents.filter(
      (agent) => agent.enabled && agent.employment_status === "active",
    );

    if (enabledAgents.length === 0) {
      setAgentError("No active enabled agents found.");
      return;
    }

    try {
      await Promise.all(
        enabledAgents.map((agent) =>
          bindAgentModelMutation.mutateAsync({
            agentId: agent.id,
            modelId: deepSeekModel.id,
          }),
        ),
      );
      setAgentFeedback(`Bound ${enabledAgents.length} enabled agents to ${deepSeekModel.name}.`);
      setAgentError(null);
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
        savedTeam = await updateTeamMutation.mutateAsync({ id: selectedTeam.id, ...payload });
      } else {
        savedTeam = await createTeamMutation.mutateAsync(payload);
      }
      setSelectedTeamId(savedTeam.id);
      await syncTeamSkills(savedTeam.id, selectedTeamSkillIds);
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
        await updateSkillMutation.mutateAsync({ id: selectedSkill.id, ...payload });
      } else {
        await createSkillMutation.mutateAsync(payload);
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
      await upsertRoutingPolicyMutation.mutateAsync({
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
      await addMembershipMutation.mutateAsync({
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
      await assignScopeMutation.mutateAsync({
        teamId: selectedTeamId,
        scopeType: assignmentScopeType,
        scopeId,
      });
    } catch {
      // Mutation handlers already set error state.
    }
  };

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
      onHireAgent={() => {
        setSelectedAgentId(null);
        setSelectedAgentSkillIds([]);
        setAgentDraft(blankAgentDraft());
        setAgentError(null);
        setAgentFeedback(null);
      }}
      onBindAllAgentsToDeepSeek={handleBindAllAgentsToDeepSeek}
      onBindCodingAgentsToDeepSeek={handleBindCodingAgentsToDeepSeek}
      onExpandedTeamsChange={setExpandedTeams}
      onSelectTeam={(teamId) => {
        setSelectedTeamId(teamId);
        setSelectedAgentId(null);
      }}
      onEditTeam={(teamId) => {
        setSelectedTeamId(teamId);
        setSelectedAgentId(null);
        setActiveTab("teams");
      }}
      onSelectAgent={(teamId, agentId) => {
        setSelectedTeamId(teamId);
        setSelectedAgentId(agentId);
      }}
      onUnassignMembership={(teamId, membershipId) => {
        setSelectedTeamId(teamId);
        removeMembershipMutation.mutate(membershipId);
      }}
      onDeleteAgent={(agentId) => deleteAgentMutation.mutate(agentId)}
      onAgentDraftChange={setAgentDraft}
      onSelectedAgentSkillIdsChange={setSelectedAgentSkillIds}
      onSelectedAgentModelChange={setSelectedAgentModelId}
      onBindSelectedAgentModel={handleBindSelectedAgentModel}
      onSaveAgent={handleSaveAgent}
      onResetAgentForm={() => {
        if (selectedAgent) {
          setAgentDraft(parseAgentDraft(selectedAgent));
          setSelectedAgentSkillIds(
            agentSkillLinks.filter((link) => link.agent_id === selectedAgent.id).map((link) => link.skill_id),
          );
        } else {
          setAgentDraft(blankAgentDraft());
          setSelectedAgentSkillIds([]);
        }
        setAgentError(null);
        setAgentFeedback(null);
      }}
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
      isCreateTeamPending={createTeamMutation.isPending}
      isUpdateTeamPending={updateTeamMutation.isPending}
      onCreateNewTeam={() => {
        setSelectedTeamId(null);
        setSelectedTeamSkillIds([]);
        setTeamDraft(blankTeamDraft());
        setTeamError(null);
        setTeamFeedback(null);
      }}
      onSelectTeam={setSelectedTeamId}
      onTeamDraftChange={setTeamDraft}
      onSelectedTeamSkillIdsChange={setSelectedTeamSkillIds}
      onMembershipDraftChange={setMembershipDraft}
      onSaveTeam={handleSaveTeam}
      onResetTeamForm={() => {
        if (selectedTeam) {
          setTeamDraft(parseTeamDraft(selectedTeam));
          setSelectedTeamSkillIds(
            teamSkillLinks.filter((link) => link.team_id === selectedTeam.id).map((link) => link.skill_id),
          );
        } else {
          setTeamDraft(blankTeamDraft());
          setSelectedTeamSkillIds([]);
        }
        setTeamError(null);
        setTeamFeedback(null);
      }}
      onDeleteTeam={(teamId) => deleteTeamMutation.mutate(teamId)}
      onRemoveMembership={(membershipId) => removeMembershipMutation.mutate(membershipId)}
      onAddMembership={handleAddMembership}
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
      onAssignmentProductChange={(nextId) => {
        setAssignmentProductId(nextId);
        setActiveProduct(nextId);
      }}
      onAssignmentScopeTypeChange={setAssignmentScopeType}
      onAssignmentProductAreaChange={setAssignmentProductAreaId}
      onAssignmentCapabilityChange={setAssignmentCapabilityId}
      onAssignScope={handleAssignScope}
      onRemoveAssignment={(assignmentId) => removeAssignmentMutation.mutate(assignmentId)}
    />
  );

  const renderSkillsTab = () => (
    <AgentSkillsTab
      skills={skills}
      selectedSkillId={selectedSkillId}
      selectedSkill={selectedSkill}
      skillDraft={skillDraft}
      onCreateNewSkill={() => {
        setSelectedSkillId(null);
        setSkillDraft(blankSkillDraft());
        setSkillError(null);
        setSkillFeedback(null);
      }}
      onSelectSkill={setSelectedSkillId}
      onSkillDraftChange={setSkillDraft}
      onDeleteSkill={(skillId) => deleteSkillMutation.mutate(skillId)}
      onSaveSkill={handleSaveSkill}
      onResetSkillForm={() => {
        if (selectedSkill) {
          setSkillDraft(parseSkillDraft(selectedSkill));
        } else {
          setSkillDraft(blankSkillDraft());
        }
        setSkillError(null);
        setSkillFeedback(null);
      }}
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
      onDeleteRoutingPolicy={(stageName) => deleteRoutingPolicyMutation.mutate(stageName)}
      onSaveRoutingPolicy={handleSaveRoutingPolicy}
      onResetRoutingForm={() => {
        setRoutingDraft(parsePolicyDraft(selectedPolicy, selectedPolicyStage));
        setRoutingError(null);
        setRoutingFeedback(null);
      }}
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

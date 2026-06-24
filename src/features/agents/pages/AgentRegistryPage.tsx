import React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
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
  listAgentModelBindings,
  listAgentDefinitions,
  listModelDefinitions,
  listAgentSkillLinks,
  listAgentTeams,
  listProductAreas,
  listProductCapabilities,
  listProducts,
  listSkills,
  listTeamAssignments,
  listTeamMemberships,
  listTeamSkillLinks,
  listWorkflowStagePolicies,
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
  Capability,
  Product,
  ProductArea,
  Skill,
} from "../../../lib/types";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { AgentAssignmentsTab } from "../components/AgentAssignmentsTab";
import { AgentProfileTab } from "../components/AgentProfileTab";
import { AgentRegistryHeader } from "../components/AgentRegistryHeader";
import { AgentRoutingTab } from "../components/AgentRoutingTab";
import { AgentSkillChooser } from "../components/AgentSkillChooser";
import { AgentSkillsTab } from "../components/AgentSkillsTab";
import { styles } from "../lib/agentRegistryPageStyles";
import {
  blankAgentDraft,
  blankSkillDraft,
  blankTeamDraft,
  buildCapabilityOptions,
  buildTeamMembershipsByTeam,
  countAssignmentsByType,
  filterUnassignedAgents,
  formatUiError,
  parseAgentDraft,
  parsePolicyDraft,
  parseSkillDraft,
  parseTeamDraft,
  selectEntityById,
  selectPolicyByStage,
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
  const hasInitializedAgentSelection = React.useRef(false);
  const hasInitializedTeamSelection = React.useRef(false);
  const hasInitializedSkillSelection = React.useRef(false);

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

  React.useEffect(() => {
    if (!hasInitializedAgentSelection.current && !selectedAgentId && agents.length > 0) {
      setSelectedAgentId(agents[0].id);
      hasInitializedAgentSelection.current = true;
    }
  }, [agents, selectedAgentId]);

  React.useEffect(() => {
    if (!hasInitializedTeamSelection.current && !selectedTeamId && teams.length > 0) {
      setSelectedTeamId(teams[0].id);
      hasInitializedTeamSelection.current = true;
    }
  }, [teams, selectedTeamId]);

  React.useEffect(() => {
    if (!hasInitializedSkillSelection.current && !selectedSkillId && skills.length > 0) {
      setSelectedSkillId(skills[0].id);
      hasInitializedSkillSelection.current = true;
    }
  }, [skills, selectedSkillId]);

  React.useEffect(() => {
    if (!assignmentProductId && activeProductId) {
      setAssignmentProductId(activeProductId);
    }
  }, [activeProductId, assignmentProductId]);

  const selectedAgent = selectEntityById(agents, selectedAgentId);
  const selectedTeam = selectEntityById(teams, selectedTeamId);
  const selectedSkill = selectEntityById(skills, selectedSkillId);
  const selectedPolicy = selectPolicyByStage(routingPolicies, selectedPolicyStage);

  React.useEffect(() => {
    if (selectedAgent) {
      setAgentDraft(parseAgentDraft(selectedAgent));
      setSelectedAgentSkillIds(
        agentSkillLinks.filter((link) => link.agent_id === selectedAgent.id).map((link) => link.skill_id),
      );
      setAgentError(null);
      setAgentFeedback(null);
    }
  }, [selectedAgent, agentSkillLinks]);

  React.useEffect(() => {
    if (selectedAgent) {
      const binding = agentModelBindings.find((entry) => entry.agent_id === selectedAgent.id);
      setSelectedAgentModelId(binding?.model_id ?? "");
    } else {
      setSelectedAgentModelId("");
    }
  }, [selectedAgent, agentModelBindings]);

  React.useEffect(() => {
    if (selectedTeam) {
      setTeamDraft(parseTeamDraft(selectedTeam));
      setSelectedTeamSkillIds(
        teamSkillLinks.filter((link) => link.team_id === selectedTeam.id).map((link) => link.skill_id),
      );
      setTeamError(null);
      setTeamFeedback(null);
    }
  }, [selectedTeam, teamSkillLinks]);

  React.useEffect(() => {
    if (selectedSkill) {
      setSkillDraft(parseSkillDraft(selectedSkill));
      setSkillError(null);
      setSkillFeedback(null);
    }
  }, [selectedSkill]);

  React.useEffect(() => {
    setRoutingDraft(parsePolicyDraft(selectedPolicy, selectedPolicyStage));
    setRoutingError(null);
    setRoutingFeedback(null);
  }, [selectedPolicy, selectedPolicyStage]);

  React.useEffect(() => {
    const firstProductAreaId = assignmentProductAreas[0]?.id ?? "";
    if (!assignmentProductAreaId || !assignmentProductAreas.some((entry) => entry.id === assignmentProductAreaId)) {
      setAssignmentProductAreaId(firstProductAreaId);
    }
  }, [assignmentProductAreaId, assignmentProductAreas]);

  const currentProductAreaOptions = assignmentProductAreas;
  const currentCapabilityOptions = React.useMemo(
    () => buildCapabilityOptions(assignmentCapabilities),
    [assignmentCapabilities],
  );

  React.useEffect(() => {
    const availableCapabilityIds = currentCapabilityOptions.map((capability) => capability.id);
    if (!assignmentCapabilityId || !availableCapabilityIds.includes(assignmentCapabilityId)) {
      setAssignmentCapabilityId(currentCapabilityOptions[0]?.id ?? "");
    }
  }, [assignmentCapabilityId, currentCapabilityOptions]);

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

  const invalidateAgentData = React.useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["agents"] }),
      queryClient.invalidateQueries({ queryKey: ["agent-teams"] }),
      queryClient.invalidateQueries({ queryKey: ["agent-team-memberships"] }),
      queryClient.invalidateQueries({ queryKey: ["agent-team-assignments"] }),
      queryClient.invalidateQueries({ queryKey: ["agent-model-bindings"] }),
      queryClient.invalidateQueries({ queryKey: ["skills"] }),
      queryClient.invalidateQueries({ queryKey: ["agent-skill-links"] }),
      queryClient.invalidateQueries({ queryKey: ["team-skill-links"] }),
      queryClient.invalidateQueries({ queryKey: ["workflow-stage-policies"] }),
      queryClient.invalidateQueries({ queryKey: ["model-definitions"] }),
      queryClient.refetchQueries({ queryKey: ["agents"], type: "active" }),
      queryClient.refetchQueries({ queryKey: ["agent-teams"], type: "active" }),
      queryClient.refetchQueries({ queryKey: ["agent-team-memberships"], type: "active" }),
      queryClient.refetchQueries({ queryKey: ["agent-team-assignments"], type: "active" }),
    ]);
  }, [queryClient]);

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
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.toolbar}>
          <button
            type="button"
            style={styles.buttonPrimary}
            onClick={() => {
              setSelectedTeamId(null);
              setSelectedTeamSkillIds([]);
              setTeamDraft(blankTeamDraft());
              setTeamError(null);
              setTeamFeedback(null);
            }}
          >
            + New Team
          </button>
        </div>
        <div style={styles.sectionTitle}>Teams</div>
        <div style={styles.list}>
          {teamsLoading ? (
            <div style={styles.empty}>Loading teams...</div>
          ) : teams.length === 0 ? (
            <div style={styles.empty}>No teams created yet.</div>
          ) : (
            teams.map((team) => {
              const memberCount = memberships.filter((membership) => membership.team_id === team.id).length;
              return (
                <button
                  key={team.id}
                  type="button"
                  style={{
                    ...styles.listItem,
                    ...(team.id === selectedTeamId ? styles.listItemActive : {}),
                    textAlign: "left",
                  }}
                  onClick={() => setSelectedTeamId(team.id)}
                >
                  <div style={styles.itemTitle}>{team.name}</div>
                  <div style={styles.itemMeta}>
                    {team.department} · {memberCount} member{memberCount === 1 ? "" : "s"}
                  </div>
                  <div style={styles.badgeRow}>
                    <span style={styles.badgeMuted}>{team.enabled ? "enabled" : "disabled"}</span>
                  </div>
                </button>
              );
            })
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedTeam ? "Edit Team" : "Create Team"}</h2>
            <div style={styles.subtitle}>Set team profile, save, then manage members. Keep this page focused on one team at a time.</div>
          </div>
          <div style={styles.headerActions}>
            <button
              type="button"
              style={{
                ...styles.buttonPrimary,
                opacity: !teamDraft.name.trim() ? 0.55 : 1,
                cursor: !teamDraft.name.trim() ? "not-allowed" : "pointer",
              }}
              disabled={!teamDraft.name.trim() || createTeamMutation.isPending || updateTeamMutation.isPending}
              onClick={handleSaveTeam}
            >
              {selectedTeam ? "Save Team" : "Create Team"}
            </button>
            <button
              type="button"
              style={styles.buttonSecondary}
              onClick={() => {
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
            >
              Reset
            </button>
            {selectedTeam ? (
              <button type="button" style={styles.buttonDanger} onClick={() => deleteTeamMutation.mutate(selectedTeam.id)}>
                Delete
              </button>
            ) : null}
          </div>
        </div>

        <div style={styles.teamStatsRow}>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Roster</div>
            <div style={styles.infoValue}>{selectedTeamMemberships.length} member{selectedTeamMemberships.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Assignments</div>
            <div style={styles.infoValue}>{selectedTeamAssignments.length} scope assignment{selectedTeamAssignments.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Skills</div>
            <div style={styles.infoValue}>{selectedTeamSkillIds.length} linked skill{selectedTeamSkillIds.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Capacity</div>
            <div style={styles.infoValue}>
              {selectedTeam ? selectedTeam.max_concurrent_workflows : teamDraft.maxConcurrentWorkflows} concurrent workflow
              {(selectedTeam ? selectedTeam.max_concurrent_workflows : teamDraft.maxConcurrentWorkflows) === 1 ? "" : "s"}
            </div>
          </div>
        </div>

        <div style={styles.teamPanel}>
          <div style={styles.teamPanelTitle}>Team Profile</div>
          <div style={styles.formGrid}>
            <div style={styles.field}>
              <label style={styles.label}>Team Name</label>
              <input style={styles.input} value={teamDraft.name} onChange={(e) => setTeamDraft((draft) => ({ ...draft, name: e.target.value }))} />
              {!teamDraft.name.trim() ? <div style={styles.error}>Team name is required.</div> : null}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Department</label>
              <input style={styles.input} value={teamDraft.department} onChange={(e) => setTeamDraft((draft) => ({ ...draft, department: e.target.value }))} />
            </div>
            <div style={{ ...styles.field, ...styles.fullWidth }}>
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={teamDraft.description} onChange={(e) => setTeamDraft((draft) => ({ ...draft, description: e.target.value }))} />
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Max Concurrent Workflows</label>
              <input
                type="number"
                min={1}
                style={styles.input}
                value={teamDraft.maxConcurrentWorkflows}
                onChange={(e) =>
                  setTeamDraft((draft) => ({
                    ...draft,
                    maxConcurrentWorkflows: Number.isFinite(Number(e.target.value)) ? Math.max(1, Number(e.target.value)) : 1,
                  }))
                }
              />
            </div>
            <div style={{ ...styles.field, ...styles.fullWidth }}>
              <label style={styles.label}>Team Skills</label>
              <AgentSkillChooser
                skills={skills}
                selectedIds={selectedTeamSkillIds}
                onToggle={(skillId, checked) => {
                  setSelectedTeamSkillIds((current) =>
                    checked ? [...new Set([...current, skillId])] : current.filter((id) => id !== skillId),
                  );
                }}
              />
            </div>
          </div>
          <label style={styles.checkboxRow}>
            <input
              type="checkbox"
              checked={teamDraft.enabled}
              onChange={(e) => setTeamDraft((draft) => ({ ...draft, enabled: e.target.checked }))}
            />
            Team is active
          </label>
          {teamError ? <div style={styles.error}>{teamError}</div> : null}
          {teamFeedback ? <div style={styles.success}>{teamFeedback}</div> : null}
        </div>

        <div style={styles.teamManagementGrid}>
          <div style={styles.teamPanel}>
            <div style={styles.teamPanelTitle}>Current Members</div>
            <div style={styles.subList}>
              {selectedTeamMemberships.length === 0 ? (
                <div style={styles.empty}>No one assigned to this team yet.</div>
              ) : (
                selectedTeamMemberships.map((membership) => {
                  const agent = agents.find((entry) => entry.id === membership.agent_id);
                  return (
                    <div key={membership.id} style={styles.listItem}>
                      <div style={styles.itemTitle}>{agent?.name ?? "Unknown agent"}</div>
                      <div style={styles.itemMeta}>
                        {membership.title}
                        {membership.is_lead ? " · team lead" : ""}
                      </div>
                      <div style={styles.toolbar}>
                        <button type="button" style={styles.buttonSecondary} onClick={() => removeMembershipMutation.mutate(membership.id)}>
                          Remove
                        </button>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>
          <div style={styles.teamPanel}>
            <div style={styles.teamPanelTitle}>Add Member</div>
            <div style={styles.field}>
              <label style={styles.label}>Agent</label>
              <select
                style={styles.select}
                value={membershipDraft.agentId}
                onChange={(e) => setMembershipDraft((draft) => ({ ...draft, agentId: e.target.value }))}
              >
                <option value="">Select an agent</option>
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.id}>
                    {agent.name} · {agent.role}
                  </option>
                ))}
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Title in Team</label>
              <input
                style={styles.input}
                value={membershipDraft.title}
                onChange={(e) => setMembershipDraft((draft) => ({ ...draft, title: e.target.value }))}
                placeholder="Staff Engineer, QA Lead, Architect"
              />
            </div>
            <label style={styles.checkboxRow}>
              <input
                type="checkbox"
                checked={membershipDraft.isLead}
                onChange={(e) => setMembershipDraft((draft) => ({ ...draft, isLead: e.target.checked }))}
              />
              Team lead
            </label>
            {membershipError ? <div style={styles.error}>{membershipError}</div> : null}
            <button type="button" style={styles.buttonPrimary} onClick={handleAddMembership}>
              Add To Team
            </button>
          </div>
        </div>
      </div>
    </div>
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

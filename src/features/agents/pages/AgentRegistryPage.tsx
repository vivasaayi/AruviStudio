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
  getProductTree,
  linkSkillToAgent,
  linkSkillToTeam,
  listAgentModelBindings,
  listAgentDefinitions,
  listModelDefinitions,
  listAgentSkillLinks,
  listAgentTeams,
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
  AgentModelBinding,
  AgentTeam,
  CapabilityTree,
  ModelDefinition,
  Product,
  Skill,
  TeamAssignment,
  WorkflowStagePolicy,
} from "../../../lib/types";
import { useWorkspaceStore } from "../../../state/workspaceStore";

type AgentTab = "agents" | "teams" | "assignments" | "skills" | "routing";

type AgentDraft = {
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

type TeamDraft = {
  name: string;
  department: string;
  description: string;
  enabled: boolean;
  maxConcurrentWorkflows: number;
};

type SkillDraft = {
  name: string;
  category: string;
  description: string;
  instructions: string;
  enabled: boolean;
};

type RoutingDraft = {
  stageName: string;
  primaryRoles: string;
  fallbackRoles: string;
  coordinatorRequired: boolean;
};

const blankAgentDraft = (): AgentDraft => ({
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

const blankTeamDraft = (): TeamDraft => ({
  name: "",
  department: "engineering",
  description: "",
  enabled: true,
  maxConcurrentWorkflows: 2,
});

const blankSkillDraft = (): SkillDraft => ({
  name: "",
  category: "general",
  description: "",
  instructions: "",
  enabled: true,
});

const workflowStageOptions = [
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

function parseAgentDraft(agent: AgentDefinition): AgentDraft {
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

function parseTeamDraft(team: AgentTeam): TeamDraft {
  return {
    name: team.name,
    department: team.department,
    description: team.description,
    enabled: team.enabled,
    maxConcurrentWorkflows: team.max_concurrent_workflows,
  };
}

function parseSkillDraft(skill: Skill): SkillDraft {
  return {
    name: skill.name,
    category: skill.category,
    description: skill.description,
    instructions: skill.instructions,
    enabled: skill.enabled,
  };
}

function parsePolicyDraft(policy?: WorkflowStagePolicy | null, fallbackStage = workflowStageOptions[0]): RoutingDraft {
  return {
    stageName: policy?.stage_name ?? fallbackStage,
    primaryRoles: policy?.primary_roles.join(", ") ?? "",
    fallbackRoles: policy?.fallback_roles.join(", ") ?? "",
    coordinatorRequired: policy?.coordinator_required ?? true,
  };
}

function countAssignmentsByType(assignments: TeamAssignment[]) {
  return assignments.reduce(
    (acc, assignment) => {
      acc[assignment.scope_type] += 1;
      return acc;
    },
    { product: 0, module: 0, capability: 0 } as Record<"product" | "module" | "capability", number>,
  );
}

function flattenCapabilityOptions(features: CapabilityTree[], depth = 0): Array<{ id: string; name: string }> {
  return features.flatMap((entry) => [
    { id: entry.capability.id, name: `${"  ".repeat(depth)}${entry.capability.name}` },
    ...flattenCapabilityOptions(entry.children, depth + 1),
  ]);
}

function resolveScopeLabel(
  assignment: TeamAssignment,
  products: Product[],
  modules: Array<{ id: string; name: string }>,
  capabilities: Array<{ id: string; name: string }>,
) {
  if (assignment.scope_type === "product") {
    return products.find((product) => product.id === assignment.scope_id)?.name ?? assignment.scope_id;
  }
  if (assignment.scope_type === "module") {
    return modules.find((module) => module.id === assignment.scope_id)?.name ?? assignment.scope_id;
  }
  return capabilities.find((capability) => capability.id === assignment.scope_id)?.name.trim() ?? assignment.scope_id;
}

function formatUiError(error: unknown): string {
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

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 16, height: "100%", minHeight: 0 },
  headerRow: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  titleWrap: { display: "flex", flexDirection: "column", gap: 4 },
  title: { fontSize: 18, fontWeight: 700, color: "#f0f0f0", margin: 0 },
  subtitle: { fontSize: 13, color: "#9aa0b0", lineHeight: 1.4 },
  summaryRow: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 12 },
  statCard: { backgroundColor: "#232834", borderRadius: 12, border: "1px solid #2f3643", padding: 14, display: "flex", flexDirection: "column", gap: 6 },
  statLabel: { fontSize: 11, letterSpacing: "0.08em", textTransform: "uppercase", color: "#8b93a7" },
  statValue: { fontSize: 24, fontWeight: 700, color: "#f5f6fb" },
  tabRow: { display: "flex", gap: 8, flexWrap: "wrap" },
  tab: { borderRadius: 10, border: "1px solid #2f3643", backgroundColor: "#252b38", color: "#cfd5e2", padding: "9px 14px", fontSize: 13, fontWeight: 700, cursor: "pointer" },
  tabActive: { backgroundColor: "#123d5a", borderColor: "#1f79b9", color: "#ffffff" },
  workspace: { display: "grid", gridTemplateColumns: "300px minmax(0, 1fr)", gap: 16, minHeight: 0, flex: 1 },
  rail: { backgroundColor: "#1b2130", borderRadius: 14, border: "1px solid #2c3340", padding: 14, display: "flex", flexDirection: "column", gap: 12, minHeight: 0 },
  detail: { backgroundColor: "#1b2130", borderRadius: 14, border: "1px solid #2c3340", padding: 16, display: "flex", flexDirection: "column", gap: 14, minHeight: 0, overflowY: "auto" as const },
  sectionTitle: { fontSize: 12, letterSpacing: "0.08em", textTransform: "uppercase", color: "#8b93a7", fontWeight: 700 },
  list: { display: "flex", flexDirection: "column", gap: 8, overflowY: "auto", minHeight: 0 },
  listItem: { borderRadius: 12, border: "1px solid #2d3442", backgroundColor: "#222938", padding: 12, cursor: "pointer", display: "flex", flexDirection: "column", gap: 6 },
  listItemActive: { borderColor: "#1f79b9", background: "linear-gradient(135deg, rgba(18,61,90,0.55), rgba(34,41,56,0.92))" },
  itemTitle: { fontSize: 14, fontWeight: 700, color: "#f3f5fa" },
  itemMeta: { fontSize: 12, color: "#9ea6bb", lineHeight: 1.35 },
  badgeRow: { display: "flex", gap: 8, flexWrap: "wrap" },
  badge: { fontSize: 11, borderRadius: 999, padding: "3px 8px", backgroundColor: "#153627", color: "#4ee2a1" },
  badgeMuted: { fontSize: 11, borderRadius: 999, padding: "3px 8px", backgroundColor: "#31384a", color: "#cfd5e2" },
  toolbar: { display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" },
  headerActions: { display: "flex", gap: 8, alignItems: "center", position: "relative" as const, zIndex: 5 },
  buttonPrimary: { borderRadius: 10, border: "1px solid #1f79b9", backgroundColor: "#0e639c", color: "#ffffff", padding: "9px 14px", fontSize: 13, fontWeight: 700, cursor: "pointer" },
  buttonSecondary: { borderRadius: 10, border: "1px solid #3a4354", backgroundColor: "#2b3241", color: "#e6ebf5", padding: "9px 14px", fontSize: 13, fontWeight: 700, cursor: "pointer" },
  buttonDanger: { borderRadius: 10, border: "1px solid #6b2930", backgroundColor: "#4b2026", color: "#ffb6bb", padding: "9px 14px", fontSize: 13, fontWeight: 700, cursor: "pointer" },
  formGrid: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12 },
  fullWidth: { gridColumn: "1 / -1" },
  field: { display: "flex", flexDirection: "column", gap: 6 },
  label: { fontSize: 12, color: "#aab2c5", fontWeight: 600 },
  input: { width: "100%", borderRadius: 10, border: "1px solid #364053", backgroundColor: "#111722", color: "#f3f6fc", padding: "10px 12px", fontSize: 13, boxSizing: "border-box" as const },
  textarea: { width: "100%", minHeight: 84, borderRadius: 10, border: "1px solid #364053", backgroundColor: "#111722", color: "#f3f6fc", padding: "10px 12px", fontSize: 13, resize: "vertical" as const, boxSizing: "border-box" as const },
  select: { width: "100%", borderRadius: 10, border: "1px solid #364053", backgroundColor: "#111722", color: "#f3f6fc", padding: "10px 12px", fontSize: 13, boxSizing: "border-box" as const },
  checkboxRow: { display: "flex", alignItems: "center", gap: 8, fontSize: 13, color: "#dbe1ec" },
  infoCard: { borderRadius: 12, border: "1px solid #2d3442", backgroundColor: "#222938", padding: 12, display: "flex", flexDirection: "column", gap: 6 },
  infoValue: { fontSize: 13, color: "#dbe1ec", lineHeight: 1.4 },
  split: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(280px, 360px)", gap: 16, minHeight: 0 },
  teamStatsRow: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 10 },
  teamStatChip: { borderRadius: 10, border: "1px solid #2d3442", backgroundColor: "#222938", padding: 10, display: "flex", flexDirection: "column", gap: 4 },
  teamPanel: { borderRadius: 12, border: "1px solid #2d3442", backgroundColor: "#1f2533", padding: 14, display: "flex", flexDirection: "column", gap: 12 },
  teamPanelTitle: { fontSize: 12, letterSpacing: "0.08em", textTransform: "uppercase" as const, color: "#8b93a7", fontWeight: 700, marginBottom: 2 },
  teamManagementGrid: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(320px, 420px)", gap: 16, minHeight: 0 },
  subList: { display: "flex", flexDirection: "column", gap: 8, maxHeight: 260, overflowY: "auto" as const },
  skillList: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 8 },
  skillPill: { borderRadius: 10, border: "1px solid #364053", backgroundColor: "#202736", padding: "8px 10px", display: "flex", alignItems: "center", gap: 8 },
  error: { color: "#ff8a92", fontSize: 12 },
  success: { color: "#64e3b3", fontSize: 12 },
  empty: { color: "#8f97aa", fontSize: 13, padding: "18px 0" },
  divider: { height: 1, backgroundColor: "#2c3340", margin: "4px 0" },
  treeTable: { display: "flex", flexDirection: "column", border: "1px solid #2d3442", borderRadius: 12, overflow: "hidden", backgroundColor: "#171c27" },
  treeHeader: { display: "grid", gridTemplateColumns: "minmax(0, 1.4fr) 120px 130px 110px", gap: 10, padding: "10px 12px", borderBottom: "1px solid #2d3442", fontSize: 11, fontWeight: 700, letterSpacing: "0.06em", textTransform: "uppercase" as const, color: "#8b93a7" },
  treeRow: { display: "grid", gridTemplateColumns: "minmax(0, 1.4fr) 120px 130px 110px", gap: 10, padding: "10px 12px", borderBottom: "1px solid #242a36", alignItems: "center", cursor: "pointer" },
  treeRowActive: { backgroundColor: "#173247" },
  treeNameCell: { display: "flex", alignItems: "center", gap: 8, minWidth: 0 },
  treeIndent: { width: 14, flexShrink: 0 },
  treeCaret: { width: 10, color: "#8b93a7", fontSize: 11, flexShrink: 0 },
  treeName: { fontSize: 13, fontWeight: 700, color: "#f3f5fa", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" as const },
  treeSubName: { fontSize: 12, color: "#9ea6bb", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" as const },
  treeCell: { fontSize: 12, color: "#cfd5e2" },
  treeMetaBadge: { fontSize: 11, borderRadius: 999, padding: "3px 8px", backgroundColor: "#31384a", color: "#cfd5e2", display: "inline-block" },
  treeActions: { display: "flex", gap: 6, justifyContent: "flex-end" },
  treeActionBtn: { borderRadius: 7, border: "1px solid #3a4354", backgroundColor: "#2b3241", color: "#e6ebf5", padding: "4px 7px", fontSize: 11, fontWeight: 700, cursor: "pointer" },
};

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
  const [assignmentScopeType, setAssignmentScopeType] = React.useState<"product" | "module" | "capability">("module");
  const [assignmentModuleId, setAssignmentModuleId] = React.useState<string>("");
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
  const { data: assignmentTree } = useQuery({
    queryKey: ["agent-assignment-tree", assignmentProductId],
    queryFn: () => getProductTree(assignmentProductId as string),
    enabled: Boolean(assignmentProductId),
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

  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId) ?? null;
  const selectedTeam = teams.find((team) => team.id === selectedTeamId) ?? null;
  const selectedSkill = skills.find((skill) => skill.id === selectedSkillId) ?? null;
  const selectedPolicy = routingPolicies.find((policy) => policy.stage_name === selectedPolicyStage) ?? null;

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
    const firstModuleId = assignmentTree?.modules[0]?.module.id ?? "";
    if (!assignmentModuleId || !assignmentTree?.modules.some((entry) => entry.module.id === assignmentModuleId)) {
      setAssignmentModuleId(firstModuleId);
    }
  }, [assignmentTree, assignmentModuleId]);

  const currentModules = assignmentTree?.modules ?? [];
  const currentModuleOptions = currentModules.map((entry) => entry.module);
  const currentCapabilityOptions = currentModules.flatMap((entry) => flattenCapabilityOptions(entry.features));

  React.useEffect(() => {
    const availableCapabilityIds = currentCapabilityOptions.map((capability) => capability.id);
    if (!assignmentCapabilityId || !availableCapabilityIds.includes(assignmentCapabilityId)) {
      setAssignmentCapabilityId(currentCapabilityOptions[0]?.id ?? "");
    }
  }, [assignmentCapabilityId, currentCapabilityOptions]);

  const selectedTeamMemberships = memberships.filter((membership) => membership.team_id === selectedTeamId);
  const selectedTeamAssignments = assignments.filter((assignment) => assignment.team_id === selectedTeamId);
  const assignmentCounts = countAssignmentsByType(assignments);
  const teamMembershipsByTeam = React.useMemo(() => {
    const next = new Map<string, typeof memberships>();
    memberships.forEach((membership) => {
      const current = next.get(membership.team_id) ?? [];
      next.set(membership.team_id, [...current, membership]);
    });
    return next;
  }, [memberships]);
  const unassignedAgents = React.useMemo(() => {
    const assignedAgentIds = new Set(memberships.map((membership) => membership.agent_id));
    return agents.filter((agent) => !assignedAgentIds.has(agent.id));
  }, [agents, memberships]);

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
    if (assignmentScopeType === "module") {
      scopeId = assignmentModuleId;
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

  const renderSkillChooser = (
    selectedIds: string[],
    onToggle: (skillId: string, checked: boolean) => void,
  ) => (
    <div style={styles.skillList}>
      {skills.length === 0 ? (
        <div style={styles.empty}>Add skills in the Skills tab first.</div>
      ) : (
        skills.map((skill) => (
          <label key={skill.id} style={styles.skillPill}>
            <input
              type="checkbox"
              checked={selectedIds.includes(skill.id)}
              onChange={(e) => onToggle(skill.id, e.target.checked)}
            />
            <span style={{ color: "#dbe1ec", fontSize: 12 }}>
              {skill.name} <span style={{ color: "#8f97aa" }}>({skill.category})</span>
            </span>
          </label>
        ))
      )}
    </div>
  );

  const renderAgentTab = () => (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.toolbar}>
          <button
            type="button"
            style={styles.buttonPrimary}
            onClick={() => {
              setSelectedAgentId(null);
              setSelectedAgentSkillIds([]);
              setAgentDraft(blankAgentDraft());
              setAgentError(null);
              setAgentFeedback(null);
            }}
          >
            + Hire Agent
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={handleBindAllAgentsToDeepSeek}
          >
            Use DeepSeek for All Enabled Agents
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={handleBindCodingAgentsToDeepSeek}
          >
            Use DeepSeek for Coding Agents
          </button>
        </div>
        <div style={styles.sectionTitle}>Org Tree</div>
        {agentsLoading || teamsLoading ? (
          <div style={styles.empty}>Loading organization...</div>
        ) : (
          <div style={styles.treeTable}>
            <div style={{ ...styles.treeHeader, gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px" }}>
              <div>Name</div>
              <div>Type</div>
              <div>Role / Lead</div>
              <div>Status</div>
              <div style={{ textAlign: "right" }}>Actions</div>
            </div>
            {teams.length === 0 && unassignedAgents.length === 0 ? (
              <div style={{ ...styles.empty, padding: 14 }}>No agents hired yet.</div>
            ) : (
              <>
                {teams.map((team) => {
                  const isExpanded = expandedTeams[team.id] ?? true;
                  const memberRows = teamMembershipsByTeam.get(team.id) ?? [];
                  return (
                    <React.Fragment key={team.id}>
                      <div
                        style={{
                          ...styles.treeRow,
                          gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                          ...(team.id === selectedTeamId && !selectedAgentId ? styles.treeRowActive : {}),
                        }}
                        onClick={() => {
                          setSelectedTeamId(team.id);
                          setSelectedAgentId(null);
                        }}
                      >
                        <div style={styles.treeNameCell}>
                          <span style={styles.treeCaret}>{isExpanded ? "▾" : "▸"}</span>
                          <span style={styles.treeName}>{team.name}</span>
                        </div>
                        <div style={styles.treeCell}>Team</div>
                        <div style={styles.treeCell}>
                          {memberRows.find((membership) => membership.is_lead)?.title ?? "No lead set"}
                        </div>
                        <div><span style={styles.treeMetaBadge}>{team.enabled ? "enabled" : "disabled"}</span></div>
                        <div style={styles.treeActions}>
                          <button
                            type="button"
                            style={styles.treeActionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setExpandedTeams((current) => ({ ...current, [team.id]: !isExpanded }));
                            }}
                          >
                            {isExpanded ? "Collapse" : "Expand"}
                          </button>
                          <button
                            type="button"
                            style={styles.treeActionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedTeamId(team.id);
                              setSelectedAgentId(null);
                              setActiveTab("teams");
                            }}
                          >
                            Edit
                          </button>
                        </div>
                      </div>
                      {isExpanded ? memberRows.map((membership) => {
                        const agent = agents.find((entry) => entry.id === membership.agent_id);
                        if (!agent) {
                          return null;
                        }
                        return (
                          <div
                            key={membership.id}
                            style={{
                              ...styles.treeRow,
                              gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                              ...(agent.id === selectedAgentId ? styles.treeRowActive : {}),
                            }}
                            onClick={() => {
                              setSelectedTeamId(team.id);
                              setSelectedAgentId(agent.id);
                            }}
                            >
                              <div style={styles.treeNameCell}>
                                <span style={styles.treeIndent} />
                                <span style={styles.treeCaret}>•</span>
                                <span style={styles.treeSubName}>{agent.name}</span>
                            </div>
                            <div style={styles.treeCell}>Agent</div>
                            <div style={styles.treeCell}>
                              {agent.role}{membership.is_lead ? " (lead)" : ""}
                            </div>
                            <div><span style={styles.treeMetaBadge}>{agent.employment_status}</span></div>
                            <div style={styles.treeActions}>
                              <button
                                type="button"
                                style={styles.treeActionBtn}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setSelectedTeamId(team.id);
                                  setSelectedAgentId(agent.id);
                                }}
                              >
                                Edit
                              </button>
                              <button
                                type="button"
                                style={styles.treeActionBtn}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setSelectedTeamId(team.id);
                                  removeMembershipMutation.mutate(membership.id);
                                }}
                              >
                                Unassign
                              </button>
                            </div>
                          </div>
                        );
                      }) : null}
                    </React.Fragment>
                  );
                })}
                {unassignedAgents.length > 0 ? (
                  <>
                    <div style={{ ...styles.treeRow, gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px" }}>
                      <div style={styles.treeNameCell}>
                        <span style={styles.treeCaret}>▾</span>
                        <span style={styles.treeName}>Unassigned</span>
                      </div>
                      <div style={styles.treeCell}>Group</div>
                      <div style={styles.treeCell}>No team</div>
                      <div />
                      <div />
                    </div>
                    {unassignedAgents.map((agent) => (
                      <div
                        key={agent.id}
                        style={{
                          ...styles.treeRow,
                          gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                          ...(agent.id === selectedAgentId ? styles.treeRowActive : {}),
                        }}
                        onClick={() => {
                          setSelectedTeamId(null);
                          setSelectedAgentId(agent.id);
                        }}
                      >
                        <div style={styles.treeNameCell}>
                          <span style={styles.treeIndent} />
                          <span style={styles.treeCaret}>•</span>
                          <span style={styles.treeSubName}>{agent.name}</span>
                        </div>
                        <div style={styles.treeCell}>Agent</div>
                        <div style={styles.treeCell}>{agent.role}</div>
                        <div><span style={styles.treeMetaBadge}>{agent.employment_status}</span></div>
                        <div style={styles.treeActions}>
                          <button
                            type="button"
                            style={styles.treeActionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedTeamId(null);
                              setSelectedAgentId(agent.id);
                            }}
                          >
                            Edit
                          </button>
                        </div>
                      </div>
                    ))}
                  </>
                ) : null}
              </>
            )}
          </div>
        )}
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedAgent ? "Edit Agent" : "Hire Agent"}</h2>
            <div style={styles.subtitle}>Model explicit roles and link each agent to reusable skills instead of relying only on freeform tags.</div>
          </div>
          {selectedAgent ? (
            <button type="button" style={styles.buttonDanger} onClick={() => deleteAgentMutation.mutate(selectedAgent.id)}>
              Remove
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Name</label>
            <input style={styles.input} value={agentDraft.name} onChange={(e) => setAgentDraft((draft) => ({ ...draft, name: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Role</label>
            <input style={styles.input} value={agentDraft.role} onChange={(e) => setAgentDraft((draft) => ({ ...draft, role: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Description</label>
            <textarea style={styles.textarea} value={agentDraft.description} onChange={(e) => setAgentDraft((draft) => ({ ...draft, description: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Prompt Template Ref</label>
            <input style={styles.input} value={agentDraft.promptTemplateRef} onChange={(e) => setAgentDraft((draft) => ({ ...draft, promptTemplateRef: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Employment Status</label>
            <select
              style={styles.select}
              value={agentDraft.employmentStatus}
              onChange={(e) =>
                setAgentDraft((draft) => ({ ...draft, employmentStatus: e.target.value as AgentDraft["employmentStatus"] }))
              }
            >
              <option value="active">active</option>
              <option value="inactive">inactive</option>
              <option value="terminated">terminated</option>
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Allowed Tools (comma-separated)</label>
            <input style={styles.input} value={agentDraft.allowedTools} onChange={(e) => setAgentDraft((draft) => ({ ...draft, allowedTools: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Skill Tags (comma-separated)</label>
            <input style={styles.input} value={agentDraft.skillTags} onChange={(e) => setAgentDraft((draft) => ({ ...draft, skillTags: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Boundaries (JSON object)</label>
            <textarea style={{ ...styles.textarea, minHeight: 120 }} value={agentDraft.boundaries} onChange={(e) => setAgentDraft((draft) => ({ ...draft, boundaries: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Linked Skills</label>
            {renderSkillChooser(selectedAgentSkillIds, (skillId, checked) => {
              setSelectedAgentSkillIds((current) =>
                checked ? [...new Set([...current, skillId])] : current.filter((id) => id !== skillId),
              );
            })}
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Primary Model</label>
            <select
              style={styles.select}
              value={selectedAgentModelId}
              onChange={(e) => setSelectedAgentModelId(e.target.value)}
            >
              <option value="">Select a model definition</option>
              {modelDefinitions.map((model: ModelDefinition) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))}
            </select>
            <div style={styles.toolbar}>
              <button type="button" style={styles.buttonSecondary} onClick={handleBindSelectedAgentModel} disabled={!selectedAgent || !selectedAgentModelId}>
                Bind Model
              </button>
              {selectedAgentModelId ? (
                <span style={styles.infoValue}>
                  {modelDefinitions.find((model) => model.id === selectedAgentModelId)?.name ?? "Selected model"}
                </span>
              ) : null}
            </div>
          </div>
        </div>
        <label style={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={agentDraft.enabled}
            onChange={(e) => setAgentDraft((draft) => ({ ...draft, enabled: e.target.checked }))}
          />
          Enabled for orchestration
        </label>
        {agentError ? <div style={styles.error}>{agentError}</div> : null}
        {agentFeedback ? <div style={styles.success}>{agentFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={handleSaveAgent}>
            {selectedAgent ? "Save Agent" : "Hire Agent"}
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={() => {
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
          >
            Reset
          </button>
        </div>
      </div>
    </div>
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
              {renderSkillChooser(selectedTeamSkillIds, (skillId, checked) => {
                setSelectedTeamSkillIds((current) =>
                  checked ? [...new Set([...current, skillId])] : current.filter((id) => id !== skillId),
                );
              })}
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
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.sectionTitle}>Teams</div>
        <div style={styles.list}>
          {teams.length === 0 ? (
            <div style={styles.empty}>Create a team before assigning delivery scope.</div>
          ) : (
            teams.map((team) => (
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
                <div style={styles.itemMeta}>{team.department}</div>
              </button>
            ))
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>Scope Assignments</h2>
            <div style={styles.subtitle}>Resolve work items to a team first: capability or outcome beats module, module beats product.</div>
          </div>
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Team</label>
            <select style={styles.select} value={selectedTeamId ?? ""} onChange={(e) => setSelectedTeamId(e.target.value || null)}>
              <option value="">Select a team</option>
              {teams.map((team) => (
                <option key={team.id} value={team.id}>
                  {team.name}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Product</label>
            <select
              style={styles.select}
              value={assignmentProductId ?? ""}
              onChange={(e) => {
                const nextId = e.target.value || null;
                setAssignmentProductId(nextId);
                setActiveProduct(nextId);
              }}
            >
              <option value="">Select a product</option>
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Scope Type</label>
            <select
              style={styles.select}
              value={assignmentScopeType}
              onChange={(e) => setAssignmentScopeType(e.target.value as "product" | "module" | "capability")}
            >
              <option value="product">Product</option>
              <option value="module">Module</option>
              <option value="capability">Capability / Outcome</option>
            </select>
          </div>
          {assignmentScopeType === "module" ? (
            <div style={styles.field}>
              <label style={styles.label}>Module</label>
              <select style={styles.select} value={assignmentModuleId} onChange={(e) => setAssignmentModuleId(e.target.value)}>
                <option value="">Select a module</option>
                {currentModuleOptions.map((module) => (
                  <option key={module.id} value={module.id}>
                    {module.name}
                  </option>
                ))}
              </select>
            </div>
          ) : null}
          {assignmentScopeType === "capability" ? (
            <div style={styles.field}>
              <label style={styles.label}>Capability / Outcome</label>
              <select style={styles.select} value={assignmentCapabilityId} onChange={(e) => setAssignmentCapabilityId(e.target.value)}>
                <option value="">Select a capability or outcome</option>
                {currentCapabilityOptions.map((capability) => (
                  <option key={capability.id} value={capability.id}>
                    {capability.name}
                  </option>
                ))}
              </select>
            </div>
          ) : null}
        </div>
        {assignmentError ? <div style={styles.error}>{assignmentError}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={handleAssignScope}>
            Assign Scope
          </button>
        </div>
        <div style={styles.divider} />
        <div style={styles.sectionTitle}>Current Assignments</div>
        {selectedTeamAssignments.length === 0 ? (
          <div style={styles.empty}>No scopes assigned to the selected team yet.</div>
        ) : (
          <div style={styles.treeTable}>
            <div style={{ ...styles.treeHeader, gridTemplateColumns: "minmax(0, 1.5fr) 100px 160px 140px" }}>
              <div>Scope</div>
              <div>Type</div>
              <div>Resolved Name</div>
              <div style={{ textAlign: "right" }}>Actions</div>
            </div>
            {selectedTeamAssignments.map((assignment) => (
              <div
                key={assignment.id}
                style={{ ...styles.treeRow, gridTemplateColumns: "minmax(0, 1.5fr) 100px 160px 140px" }}
              >
                <div style={styles.treeNameCell}>
                  <span style={styles.treeCaret}>{assignment.scope_type === "product" ? "▣" : assignment.scope_type === "module" ? "▸" : "•"}</span>
                  <span style={styles.treeSubName}>{assignment.scope_type === "capability" ? "capability" : assignment.scope_type}</span>
                </div>
                <div style={styles.treeCell}>{assignment.scope_type === "capability" ? "capability" : assignment.scope_type}</div>
                <div style={styles.treeCell}>
                  {resolveScopeLabel(
                    assignment,
                    products,
                    currentModuleOptions.map((module) => ({ id: module.id, name: module.name })),
                    currentCapabilityOptions.map((capability) => ({ id: capability.id, name: capability.name })),
                  )}
                </div>
                <div style={styles.treeActions}>
                  <button type="button" style={styles.treeActionBtn} onClick={() => removeAssignmentMutation.mutate(assignment.id)}>
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );

  const renderSkillsTab = () => (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.toolbar}>
          <button
            type="button"
            style={styles.buttonPrimary}
            onClick={() => {
              setSelectedSkillId(null);
              setSkillDraft(blankSkillDraft());
              setSkillError(null);
              setSkillFeedback(null);
            }}
          >
            + New Skill
          </button>
        </div>
        <div style={styles.sectionTitle}>Catalog</div>
        <div style={styles.list}>
          {skills.length === 0 ? (
            <div style={styles.empty}>No skills defined yet.</div>
          ) : (
            skills.map((skill) => (
              <button
                key={skill.id}
                type="button"
                style={{
                  ...styles.listItem,
                  ...(skill.id === selectedSkillId ? styles.listItemActive : {}),
                  textAlign: "left",
                }}
                onClick={() => setSelectedSkillId(skill.id)}
              >
                <div style={styles.itemTitle}>{skill.name}</div>
                <div style={styles.itemMeta}>{skill.category}</div>
                <div style={styles.badgeRow}>
                  <span style={styles.badgeMuted}>{skill.enabled ? "enabled" : "disabled"}</span>
                </div>
              </button>
            ))
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedSkill ? "Edit Skill" : "Create Skill"}</h2>
            <div style={styles.subtitle}>Define reusable capability packs and link them to both agents and teams.</div>
          </div>
          {selectedSkill ? (
            <button type="button" style={styles.buttonDanger} onClick={() => deleteSkillMutation.mutate(selectedSkill.id)}>
              Delete
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Name</label>
            <input style={styles.input} value={skillDraft.name} onChange={(e) => setSkillDraft((draft) => ({ ...draft, name: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Category</label>
            <input style={styles.input} value={skillDraft.category} onChange={(e) => setSkillDraft((draft) => ({ ...draft, category: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Description</label>
            <textarea style={styles.textarea} value={skillDraft.description} onChange={(e) => setSkillDraft((draft) => ({ ...draft, description: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Instructions</label>
            <textarea style={{ ...styles.textarea, minHeight: 140 }} value={skillDraft.instructions} onChange={(e) => setSkillDraft((draft) => ({ ...draft, instructions: e.target.value }))} />
          </div>
        </div>
        <label style={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={skillDraft.enabled}
            onChange={(e) => setSkillDraft((draft) => ({ ...draft, enabled: e.target.checked }))}
          />
          Skill is active
        </label>
        {skillError ? <div style={styles.error}>{skillError}</div> : null}
        {skillFeedback ? <div style={styles.success}>{skillFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={handleSaveSkill}>
            {selectedSkill ? "Save Skill" : "Create Skill"}
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={() => {
              if (selectedSkill) {
                setSkillDraft(parseSkillDraft(selectedSkill));
              } else {
                setSkillDraft(blankSkillDraft());
              }
              setSkillError(null);
              setSkillFeedback(null);
            }}
          >
            Reset
          </button>
        </div>
      </div>
    </div>
  );

  const renderRoutingTab = () => (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.sectionTitle}>Workflow Stages</div>
        <div style={styles.list}>
          {workflowStageOptions.map((stageName) => {
            const policy = routingPolicies.find((entry) => entry.stage_name === stageName);
            return (
              <button
                key={stageName}
                type="button"
                style={{
                  ...styles.listItem,
                  ...(selectedPolicyStage === stageName ? styles.listItemActive : {}),
                  textAlign: "left",
                }}
                onClick={() => setSelectedPolicyStage(stageName)}
              >
                <div style={styles.itemTitle}>{stageName}</div>
                <div style={styles.itemMeta}>
                  {policy ? `${policy.primary_roles.length} primary / ${policy.fallback_roles.length} fallback` : "using defaults"}
                </div>
                <div style={styles.badgeRow}>
                  <span style={styles.badgeMuted}>
                    {policy?.coordinator_required ?? true ? "coordinator on" : "coordinator off"}
                  </span>
                </div>
              </button>
            );
          })}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>Routing Policy Editor</h2>
            <div style={styles.subtitle}>Map each work item delivery stage to preferred and fallback roles, and control whether coordinator review is required before the specialist runs.</div>
          </div>
          {selectedPolicy ? (
            <button type="button" style={styles.buttonDanger} onClick={() => deleteRoutingPolicyMutation.mutate(selectedPolicy.stage_name)}>
              Reset To Default
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Stage</label>
            <select
              style={styles.select}
              value={routingDraft.stageName}
              onChange={(e) => {
                setSelectedPolicyStage(e.target.value);
                setRoutingDraft((draft) => ({ ...draft, stageName: e.target.value }));
              }}
            >
              {workflowStageOptions.map((stageName) => (
                <option key={stageName} value={stageName}>
                  {stageName}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Coordinator Review</label>
            <select
              style={styles.select}
              value={routingDraft.coordinatorRequired ? "required" : "skipped"}
              onChange={(e) => setRoutingDraft((draft) => ({ ...draft, coordinatorRequired: e.target.value === "required" }))}
            >
              <option value="required">required</option>
              <option value="skipped">skipped</option>
            </select>
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Primary Roles (comma-separated)</label>
            <input
              style={styles.input}
              value={routingDraft.primaryRoles}
              onChange={(e) => setRoutingDraft((draft) => ({ ...draft, primaryRoles: e.target.value }))}
              placeholder="developer, architect, manager"
            />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Fallback Roles (comma-separated)</label>
            <input
              style={styles.input}
              value={routingDraft.fallbackRoles}
              onChange={(e) => setRoutingDraft((draft) => ({ ...draft, fallbackRoles: e.target.value }))}
              placeholder="coding, planning"
            />
          </div>
        </div>
        {routingError ? <div style={styles.error}>{routingError}</div> : null}
        {routingFeedback ? <div style={styles.success}>{routingFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={handleSaveRoutingPolicy}>
            Save Policy
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={() => {
              setRoutingDraft(parsePolicyDraft(selectedPolicy, selectedPolicyStage));
              setRoutingError(null);
              setRoutingFeedback(null);
            }}
          >
            Reset Form
          </button>
        </div>
      </div>
    </div>
  );

  return (
    <div style={styles.page}>
      <div style={styles.headerRow}>
        <div style={styles.titleWrap}>
          <h1 style={styles.title}>Agent Management</h1>
          <div style={styles.subtitle}>
            Model the company explicitly: hire agents into specialist roles, organize them into teams, assign ownership by scope, and manage reusable skills as a real catalog.
          </div>
        </div>
      </div>

      <div style={styles.summaryRow}>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Agents</div>
          <div style={styles.statValue}>{agents.length}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Teams</div>
          <div style={styles.statValue}>{teams.length}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Assignments</div>
          <div style={styles.statValue}>{assignments.length}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Skills</div>
          <div style={styles.statValue}>{skills.length}</div>
        </div>
      </div>

      <div style={styles.tabRow}>
        {(["agents", "teams", "assignments", "skills", "routing"] as AgentTab[]).map((tab) => (
          <button
            key={tab}
            type="button"
            style={{ ...styles.tab, ...(activeTab === tab ? styles.tabActive : {}) }}
            onClick={() => setActiveTab(tab)}
          >
            {tab === "agents" ? "Agents" : tab === "teams" ? "Teams" : tab === "assignments" ? "Assignments" : tab === "skills" ? "Skills" : "Routing"}
          </button>
        ))}
      </div>

      {activeTab === "agents" ? renderAgentTab() : null}
      {activeTab === "teams" ? renderTeamTab() : null}
      {activeTab === "assignments" ? renderAssignmentsTab() : null}
      {activeTab === "skills" ? renderSkillsTab() : null}
      {activeTab === "routing" ? renderRoutingTab() : null}
    </div>
  );
}

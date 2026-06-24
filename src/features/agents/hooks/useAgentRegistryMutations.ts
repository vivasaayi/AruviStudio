import type { Dispatch, SetStateAction } from "react";
import { useMutation, type QueryClient } from "@tanstack/react-query";

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
  AgentSkillLink,
  AgentTeam,
  Skill,
  TeamSkillLink,
} from "../../../lib/types";
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
  type SkillDraft,
  type TeamDraft,
} from "../lib/agentRegistryPageModel";
import { invalidateAgentRegistryData } from "../lib/agentRegistryQueries";

type AgentRegistryMutationsInput = {
  queryClient: QueryClient;
  agents: AgentDefinition[];
  teams: AgentTeam[];
  skills: Skill[];
  agentSkillLinks: AgentSkillLink[];
  teamSkillLinks: TeamSkillLink[];
  setSelectedAgentId: Dispatch<SetStateAction<string | null>>;
  setSelectedAgentSkillIds: Dispatch<SetStateAction<string[]>>;
  setAgentDraft: Dispatch<SetStateAction<AgentDraft>>;
  setAgentFeedback: Dispatch<SetStateAction<string | null>>;
  setAgentError: Dispatch<SetStateAction<string | null>>;
  setSelectedTeamId: Dispatch<SetStateAction<string | null>>;
  setSelectedTeamSkillIds: Dispatch<SetStateAction<string[]>>;
  setTeamDraft: Dispatch<SetStateAction<TeamDraft>>;
  setTeamFeedback: Dispatch<SetStateAction<string | null>>;
  setTeamError: Dispatch<SetStateAction<string | null>>;
  setSelectedSkillId: Dispatch<SetStateAction<string | null>>;
  setSkillDraft: Dispatch<SetStateAction<SkillDraft>>;
  setSkillFeedback: Dispatch<SetStateAction<string | null>>;
  setSkillError: Dispatch<SetStateAction<string | null>>;
  setSelectedPolicyStage: Dispatch<SetStateAction<string>>;
  setRoutingDraft: Dispatch<SetStateAction<ReturnType<typeof parsePolicyDraft>>>;
  setRoutingFeedback: Dispatch<SetStateAction<string | null>>;
  setRoutingError: Dispatch<SetStateAction<string | null>>;
  setMembershipDraft: Dispatch<SetStateAction<{ agentId: string; title: string; isLead: boolean }>>;
  setMembershipError: Dispatch<SetStateAction<string | null>>;
  setAssignmentError: Dispatch<SetStateAction<string | null>>;
};

export function useAgentRegistryMutations({
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
}: AgentRegistryMutationsInput) {
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

  return {
    createAgentMutation,
    updateAgentMutation,
    deleteAgentMutation,
    createTeamMutation,
    updateTeamMutation,
    deleteTeamMutation,
    createSkillMutation,
    updateSkillMutation,
    deleteSkillMutation,
    upsertRoutingPolicyMutation,
    deleteRoutingPolicyMutation,
    bindAgentModelMutation,
    addMembershipMutation,
    removeMembershipMutation,
    assignScopeMutation,
    removeAssignmentMutation,
    syncAgentSkills,
    syncTeamSkills,
  };
}

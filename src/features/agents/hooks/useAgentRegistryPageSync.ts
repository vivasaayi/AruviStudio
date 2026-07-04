import React from "react";

import type { AgentDefinition, AgentTeam, ProductArea, Skill } from "../../../lib/types";
import {
  parseAgentDraft,
  parsePolicyDraft,
  parseSkillDraft,
  parseTeamDraft,
  type AgentDraft,
  type RoutingDraft,
  type SkillDraft,
  type TeamDraft,
} from "../lib/agentRegistryPageModel";

type AgentSkillLink = {
  agent_id: string;
  skill_id: string;
};
type TeamSkillLink = {
  team_id: string;
  skill_id: string;
};
type AgentModelBinding = {
  agent_id: string;
  model_id: string;
};
type CapabilityOption = {
  id: string;
};
type WorkflowStagePolicy = Parameters<typeof parsePolicyDraft>[0];

type AgentRegistryPageSyncInput = {
  activeProductId: string | null;
  assignmentProductId: string | null;
  setAssignmentProductId: React.Dispatch<React.SetStateAction<string | null>>;
  agents: AgentDefinition[];
  selectedAgentId: string | null;
  setSelectedAgentId: React.Dispatch<React.SetStateAction<string | null>>;
  selectedAgent: AgentDefinition | null;
  agentSkillLinks: AgentSkillLink[];
  setSelectedAgentSkillIds: React.Dispatch<React.SetStateAction<string[]>>;
  setAgentDraft: React.Dispatch<React.SetStateAction<AgentDraft>>;
  setAgentError: React.Dispatch<React.SetStateAction<string | null>>;
  setAgentFeedback: React.Dispatch<React.SetStateAction<string | null>>;
  agentModelBindings: AgentModelBinding[];
  setSelectedAgentModelId: React.Dispatch<React.SetStateAction<string>>;
  teams: AgentTeam[];
  selectedTeamId: string | null;
  setSelectedTeamId: React.Dispatch<React.SetStateAction<string | null>>;
  selectedTeam: AgentTeam | null;
  teamSkillLinks: TeamSkillLink[];
  setSelectedTeamSkillIds: React.Dispatch<React.SetStateAction<string[]>>;
  setTeamDraft: React.Dispatch<React.SetStateAction<TeamDraft>>;
  setTeamError: React.Dispatch<React.SetStateAction<string | null>>;
  setTeamFeedback: React.Dispatch<React.SetStateAction<string | null>>;
  skills: Skill[];
  selectedSkillId: string | null;
  setSelectedSkillId: React.Dispatch<React.SetStateAction<string | null>>;
  selectedSkill: Skill | null;
  setSkillDraft: React.Dispatch<React.SetStateAction<SkillDraft>>;
  setSkillError: React.Dispatch<React.SetStateAction<string | null>>;
  setSkillFeedback: React.Dispatch<React.SetStateAction<string | null>>;
  selectedPolicy: WorkflowStagePolicy;
  selectedPolicyStage: string;
  setRoutingDraft: React.Dispatch<React.SetStateAction<RoutingDraft>>;
  setRoutingError: React.Dispatch<React.SetStateAction<string | null>>;
  setRoutingFeedback: React.Dispatch<React.SetStateAction<string | null>>;
  currentProductAreaOptions: ProductArea[];
  assignmentProductAreaId: string;
  setAssignmentProductAreaId: React.Dispatch<React.SetStateAction<string>>;
  currentCapabilityOptions: CapabilityOption[];
  assignmentCapabilityId: string;
  setAssignmentCapabilityId: React.Dispatch<React.SetStateAction<string>>;
};

export function useAgentRegistryPageSync({
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
}: AgentRegistryPageSyncInput) {
  const hasInitializedAgentSelection = React.useRef(false);
  const hasInitializedTeamSelection = React.useRef(false);
  const hasInitializedSkillSelection = React.useRef(false);

  React.useEffect(() => {
    if (!hasInitializedAgentSelection.current && !selectedAgentId && agents.length > 0) {
      setSelectedAgentId(agents[0].id);
      hasInitializedAgentSelection.current = true;
    }
  }, [agents, selectedAgentId, setSelectedAgentId]);

  React.useEffect(() => {
    if (!hasInitializedTeamSelection.current && !selectedTeamId && teams.length > 0) {
      setSelectedTeamId(teams[0].id);
      hasInitializedTeamSelection.current = true;
    }
  }, [teams, selectedTeamId, setSelectedTeamId]);

  React.useEffect(() => {
    if (!hasInitializedSkillSelection.current && !selectedSkillId && skills.length > 0) {
      setSelectedSkillId(skills[0].id);
      hasInitializedSkillSelection.current = true;
    }
  }, [skills, selectedSkillId, setSelectedSkillId]);

  React.useEffect(() => {
    if (!assignmentProductId && activeProductId) {
      setAssignmentProductId(activeProductId);
    }
  }, [activeProductId, assignmentProductId, setAssignmentProductId]);

  React.useEffect(() => {
    if (selectedAgent) {
      setAgentDraft(parseAgentDraft(selectedAgent));
      setSelectedAgentSkillIds(
        agentSkillLinks.filter((link) => link.agent_id === selectedAgent.id).map((link) => link.skill_id),
      );
      setAgentError(null);
      setAgentFeedback(null);
    }
  }, [selectedAgent, agentSkillLinks, setAgentDraft, setSelectedAgentSkillIds, setAgentError, setAgentFeedback]);

  React.useEffect(() => {
    if (selectedAgent) {
      const binding = agentModelBindings.find((entry) => entry.agent_id === selectedAgent.id);
      setSelectedAgentModelId(binding?.model_id ?? "");
    } else {
      setSelectedAgentModelId("");
    }
  }, [selectedAgent, agentModelBindings, setSelectedAgentModelId]);

  React.useEffect(() => {
    if (selectedTeam) {
      setTeamDraft(parseTeamDraft(selectedTeam));
      setSelectedTeamSkillIds(
        teamSkillLinks.filter((link) => link.team_id === selectedTeam.id).map((link) => link.skill_id),
      );
      setTeamError(null);
      setTeamFeedback(null);
    }
  }, [selectedTeam, teamSkillLinks, setTeamDraft, setSelectedTeamSkillIds, setTeamError, setTeamFeedback]);

  React.useEffect(() => {
    if (selectedSkill) {
      setSkillDraft(parseSkillDraft(selectedSkill));
      setSkillError(null);
      setSkillFeedback(null);
    }
  }, [selectedSkill, setSkillDraft, setSkillError, setSkillFeedback]);

  React.useEffect(() => {
    setRoutingDraft(parsePolicyDraft(selectedPolicy, selectedPolicyStage));
    setRoutingError(null);
    setRoutingFeedback(null);
  }, [selectedPolicy, selectedPolicyStage, setRoutingDraft, setRoutingError, setRoutingFeedback]);

  React.useEffect(() => {
    const firstProductAreaId = currentProductAreaOptions[0]?.id ?? "";
    if (!assignmentProductAreaId || !currentProductAreaOptions.some((entry) => entry.id === assignmentProductAreaId)) {
      setAssignmentProductAreaId(firstProductAreaId);
    }
  }, [assignmentProductAreaId, currentProductAreaOptions, setAssignmentProductAreaId]);

  React.useEffect(() => {
    const availableCapabilityIds = currentCapabilityOptions.map((capability) => capability.id);
    if (!assignmentCapabilityId || !availableCapabilityIds.includes(assignmentCapabilityId)) {
      setAssignmentCapabilityId(currentCapabilityOptions[0]?.id ?? "");
    }
  }, [assignmentCapabilityId, currentCapabilityOptions, setAssignmentCapabilityId]);
}

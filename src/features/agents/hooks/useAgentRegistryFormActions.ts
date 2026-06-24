import type { Dispatch, SetStateAction } from "react";
import type { AgentDefinition, AgentTeam, Skill } from "../../../lib/types";
import {
  blankAgentDraft,
  blankSkillDraft,
  blankTeamDraft,
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

type AgentRegistryFormActionsArgs = {
  selectedAgent: AgentDefinition | null;
  agentSkillLinks: Array<{ agent_id: string; skill_id: string }>;
  setSelectedAgentId: Dispatch<SetStateAction<string | null>>;
  setSelectedAgentSkillIds: Dispatch<SetStateAction<string[]>>;
  setSelectedAgentModelId: Dispatch<SetStateAction<string>>;
  setAgentDraft: Dispatch<SetStateAction<AgentDraft>>;
  setAgentError: Dispatch<SetStateAction<string | null>>;
  setAgentFeedback: Dispatch<SetStateAction<string | null>>;

  selectedTeam: AgentTeam | null;
  teamSkillLinks: Array<{ team_id: string; skill_id: string }>;
  setSelectedTeamId: Dispatch<SetStateAction<string | null>>;
  setSelectedTeamSkillIds: Dispatch<SetStateAction<string[]>>;
  setTeamDraft: Dispatch<SetStateAction<TeamDraft>>;
  setTeamError: Dispatch<SetStateAction<string | null>>;
  setTeamFeedback: Dispatch<SetStateAction<string | null>>;

  selectedSkill: Skill | null;
  setSelectedSkillId: Dispatch<SetStateAction<string | null>>;
  setSkillDraft: Dispatch<SetStateAction<SkillDraft>>;
  setSkillError: Dispatch<SetStateAction<string | null>>;
  setSkillFeedback: Dispatch<SetStateAction<string | null>>;

  selectedPolicy: Parameters<typeof parsePolicyDraft>[0];
  selectedPolicyStage: string;
  setRoutingDraft: Dispatch<SetStateAction<RoutingDraft>>;
  setRoutingError: Dispatch<SetStateAction<string | null>>;
  setRoutingFeedback: Dispatch<SetStateAction<string | null>>;

  setAssignmentProductId: Dispatch<SetStateAction<string | null>>;
  setActiveProduct: (productId: string | null) => void;
  setActiveTab: Dispatch<SetStateAction<AgentTab>>;
};

export function useAgentRegistryFormActions({
  selectedAgent,
  agentSkillLinks,
  setSelectedAgentId,
  setSelectedAgentSkillIds,
  setSelectedAgentModelId,
  setAgentDraft,
  setAgentError,
  setAgentFeedback,
  selectedTeam,
  teamSkillLinks,
  setSelectedTeamId,
  setSelectedTeamSkillIds,
  setTeamDraft,
  setTeamError,
  setTeamFeedback,
  selectedSkill,
  setSelectedSkillId,
  setSkillDraft,
  setSkillError,
  setSkillFeedback,
  selectedPolicy,
  selectedPolicyStage,
  setRoutingDraft,
  setRoutingError,
  setRoutingFeedback,
  setAssignmentProductId,
  setActiveProduct,
  setActiveTab,
}: AgentRegistryFormActionsArgs) {
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
    hireAgent,
    resetAgentForm,
    resetRoutingForm,
    resetSkillForm,
    resetTeamForm,
    selectAgentFromTeam,
    selectTeamFromAgentTab,
  };
}

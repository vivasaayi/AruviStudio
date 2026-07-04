import React from "react";
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

export function useAgentRegistryPageState(activeProductId: string | null) {
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

  return {
    activeTab,
    setActiveTab,
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
  };
}

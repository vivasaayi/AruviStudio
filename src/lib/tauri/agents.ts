import { invoke, toJsonArrayString, toJsonObjectString } from "./core";
import type {
  AgentDefinition,
  AgentModelBinding,
  AgentSkillLink,
  AgentTeam,
  AgentTeamMembership,
  Skill,
  TeamAssignment,
  TeamSkillLink,
  WorkflowStagePolicy,
} from "../types";

// Agent commands
export const listAgentDefinitions = () => invoke<AgentDefinition[]>("list_agent_definitions");
export const listAgentModelBindings = () => invoke<AgentModelBinding[]>("list_agent_model_bindings");
export const setPrimaryAgentModelBinding = (data: { agentId: string; modelId: string }) =>
  invoke<AgentModelBinding>("set_primary_agent_model_binding", {
    agentId: data.agentId,
    agent_id: data.agentId,
    modelId: data.modelId,
    model_id: data.modelId,
  });
export const createAgentDefinition = (data: {
  name: string;
  role: string;
  description: string;
  promptTemplateRef: string;
  allowedTools: string;
  skillTags: string;
  boundaries: string;
  enabled: boolean;
  employmentStatus: "active" | "inactive" | "terminated";
}) =>
  invoke<AgentDefinition>("create_agent_definition", {
    request: {
      name: data.name,
      role: data.role,
      description: data.description,
      prompt_template_ref: data.promptTemplateRef,
      allowed_tools: toJsonArrayString(data.allowedTools) ?? "[]",
      skill_tags: toJsonArrayString(data.skillTags) ?? "[]",
      boundaries: toJsonObjectString(data.boundaries) ?? "{}",
      enabled: data.enabled,
      employment_status: data.employmentStatus,
    },
  });
export const updateAgentDefinition = (data: {
  id: string;
  name?: string;
  role?: string;
  description?: string;
  promptTemplateRef?: string;
  allowedTools?: string;
  skillTags?: string;
  boundaries?: string;
  enabled?: boolean;
  employmentStatus?: "active" | "inactive" | "terminated";
}) =>
  invoke<AgentDefinition>("update_agent_definition", {
    request: {
      id: data.id,
      name: data.name,
      role: data.role,
      description: data.description,
      prompt_template_ref: data.promptTemplateRef,
      allowed_tools: toJsonArrayString(data.allowedTools),
      skill_tags: toJsonArrayString(data.skillTags),
      boundaries: toJsonObjectString(data.boundaries),
      enabled: data.enabled,
      employment_status: data.employmentStatus,
    },
  });
export const deleteAgentDefinition = (id: string) => invoke("delete_agent_definition", { id });
export const listAgentTeams = () => invoke<AgentTeam[]>("list_agent_teams");
export const createAgentTeam = (data: { name: string; department: string; description: string; enabled: boolean; maxConcurrentWorkflows: number }) =>
  invoke<AgentTeam>("create_agent_team", {
    name: data.name,
    department: data.department,
    description: data.description,
    enabled: data.enabled,
    maxConcurrentWorkflows: data.maxConcurrentWorkflows,
    max_concurrent_workflows: data.maxConcurrentWorkflows,
  });
export const updateAgentTeam = (data: {
  id: string;
  name?: string;
  department?: string;
  description?: string;
  enabled?: boolean;
  maxConcurrentWorkflows?: number;
}) => invoke<AgentTeam>("update_agent_team", {
  id: data.id,
  name: data.name,
  department: data.department,
  description: data.description,
  enabled: data.enabled,
  maxConcurrentWorkflows: data.maxConcurrentWorkflows,
  max_concurrent_workflows: data.maxConcurrentWorkflows,
});
export const deleteAgentTeam = (id: string) => invoke("delete_agent_team", { id });
export const listTeamMemberships = () => invoke<AgentTeamMembership[]>("list_team_memberships");
export const addTeamMember = (data: { teamId: string; agentId: string; title: string; isLead: boolean }) =>
  invoke<AgentTeamMembership>("add_team_member", {
    teamId: data.teamId,
    team_id: data.teamId,
    agentId: data.agentId,
    agent_id: data.agentId,
    title: data.title,
    isLead: data.isLead,
    is_lead: data.isLead,
  });
export const removeTeamMember = (id: string) => invoke("remove_team_member", { id });
export const listTeamAssignments = () => invoke<TeamAssignment[]>("list_team_assignments");
export const assignTeamScope = (data: { teamId: string; scopeType: "product" | "product_area" | "capability"; scopeId: string }) =>
  invoke<TeamAssignment>("assign_team_scope", {
    teamId: data.teamId,
    team_id: data.teamId,
    scopeType: data.scopeType,
    scope_type: data.scopeType,
    scopeId: data.scopeId,
    scope_id: data.scopeId,
  });
export const removeTeamAssignment = (id: string) => invoke("remove_team_assignment", { id });
export const listSkills = () => invoke<Skill[]>("list_skills");
export const createSkill = (data: { name: string; category: string; description: string; instructions: string; enabled: boolean }) =>
  invoke<Skill>("create_skill", data);
export const updateSkill = (data: {
  id: string;
  name?: string;
  category?: string;
  description?: string;
  instructions?: string;
  enabled?: boolean;
}) => invoke<Skill>("update_skill", data);
export const deleteSkill = (id: string) => invoke("delete_skill", { id });
export const listAgentSkillLinks = () => invoke<AgentSkillLink[]>("list_agent_skill_links");
export const linkSkillToAgent = (data: { agentId: string; skillId: string; proficiency: "learning" | "working" | "expert" }) =>
  invoke<AgentSkillLink>("link_skill_to_agent", {
    agent_id: data.agentId,
    skill_id: data.skillId,
    proficiency: data.proficiency,
  });
export const unlinkSkillFromAgent = (id: string) => invoke("unlink_skill_from_agent", { id });
export const listTeamSkillLinks = () => invoke<TeamSkillLink[]>("list_team_skill_links");
export const linkSkillToTeam = (data: { teamId: string; skillId: string }) =>
  invoke<TeamSkillLink>("link_skill_to_team", {
    team_id: data.teamId,
    skill_id: data.skillId,
  });
export const unlinkSkillFromTeam = (id: string) => invoke("unlink_skill_from_team", { id });
export const listWorkflowStagePolicies = () => invoke<WorkflowStagePolicy[]>("list_workflow_stage_policies");
export const upsertWorkflowStagePolicy = (data: {
  stageName: string;
  primaryRoles: string;
  fallbackRoles: string;
  coordinatorRequired: boolean;
}) =>
  invoke<WorkflowStagePolicy>("upsert_workflow_stage_policy", {
    stage_name: data.stageName,
    primary_roles: toJsonArrayString(data.primaryRoles) ?? "[]",
    fallback_roles: toJsonArrayString(data.fallbackRoles) ?? "[]",
    coordinator_required: data.coordinatorRequired,
  });
export const deleteWorkflowStagePolicy = (stageName: string) =>
  invoke("delete_workflow_stage_policy", { stage_name: stageName });

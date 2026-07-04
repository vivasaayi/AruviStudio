import { describe, expect, it } from "vitest";

import { buildWorkItemWorkflowReadiness } from "./workItemWorkflowReadiness";
import type {
  AgentDefinition,
  AgentModelBinding,
  AgentTeam,
  AgentTeamMembership,
  ModelDefinition,
  ModelProvider,
  Repository,
  TeamAssignment,
  WorkItem,
  WorkflowStagePolicy,
} from "../../../lib/types";

const selectedWorkItem = {
  id: "story-1",
  product_id: "product-1",
  product_area_id: "area-1",
  capability_id: "capability-1",
  status: "approved",
  branch_name: "feature/story-1",
} as WorkItem;

const repository = {
  id: "repo-1",
  name: "Aruvi",
  default_branch: "main",
  remote_url: "https://example.com/repo.git",
} as Repository;

const teamAssignment = {
  team_id: "team-1",
  scope_type: "capability",
  scope_id: "capability-1",
} as TeamAssignment;

const team = { id: "team-1", name: "Delivery Team", enabled: true } as AgentTeam;

const agent = {
  id: "agent-1",
  name: "Requirement Analyst",
  role: "analyst",
  enabled: true,
  employment_status: "active",
} as AgentDefinition;

const teamMembership = {
  team_id: "team-1",
  agent_id: "agent-1",
  is_lead: true,
} as AgentTeamMembership;

const stagePolicy = {
  id: "policy-1",
  stage_name: "requirement_analysis",
  primary_roles: ["analyst"],
  fallback_roles: [],
  coordinator_required: true,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
} as WorkflowStagePolicy;

const modelBinding = { agent_id: "agent-1", model_id: "model-1" } as AgentModelBinding;
const model = { id: "model-1", provider_id: "provider-1", name: "Planning Model", enabled: true } as ModelDefinition;
const provider = { id: "provider-1", name: "Local Provider", enabled: true } as ModelProvider;

describe("workItemWorkflowReadiness", () => {
  it("blocks readiness when no work item is selected", () => {
    expect(buildWorkItemWorkflowReadiness({
      selectedWorkItem: null,
      teamAssignments: [],
      agentTeams: [],
      teamMemberships: [],
      agentDefinitions: [],
      workflowPolicies: [],
      modelBindings: [],
      modelDefinitions: [],
      providers: [],
      resolvedRepository: null,
    })).toEqual({
      blockers: ["Select a story to evaluate readiness."],
      warnings: [],
      checks: [],
    });
  });

  it("reports checks when the selected story has workspace, team, agent, and model coverage", () => {
    const readiness = buildWorkItemWorkflowReadiness({
      selectedWorkItem,
      teamAssignments: [teamAssignment],
      agentTeams: [team],
      teamMemberships: [teamMembership],
      agentDefinitions: [agent],
      workflowPolicies: [stagePolicy],
      modelBindings: [modelBinding],
      modelDefinitions: [model],
      providers: [provider],
      resolvedRepository: repository,
    });

    expect(readiness.blockers).toEqual([]);
    expect(readiness.warnings).toEqual([]);
    expect(readiness.checks).toEqual(expect.arrayContaining([
      "Work item is approved.",
      "Workspace resolved: Aruvi.",
      "Team assignment resolved: Delivery Team.",
      "Requirement-analysis agent ready: Requirement Analyst (analyst).",
      "Model binding resolved: Planning Model.",
      "Provider ready: Local Provider.",
      "Coordinator available for review gates.",
    ]));
  });

  it("surfaces missing model bindings for the selected requirement-analysis agent", () => {
    const readiness = buildWorkItemWorkflowReadiness({
      selectedWorkItem,
      teamAssignments: [teamAssignment],
      agentTeams: [team],
      teamMemberships: [teamMembership],
      agentDefinitions: [agent],
      workflowPolicies: [stagePolicy],
      modelBindings: [],
      modelDefinitions: [],
      providers: [],
      resolvedRepository: repository,
    });

    expect(readiness.blockers).toContain("Requirement-analysis agent has no model binding.");
  });
});

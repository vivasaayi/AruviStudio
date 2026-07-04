import { describe, expect, it } from "vitest";

import {
  buildCapabilityOptions,
  buildTeamMembershipsByTeam,
  countAssignmentsByType,
  filterUnassignedAgents,
  selectEntityById,
  selectPolicyByStage,
} from "./agentRegistryPageModel";
import type {
  AgentDefinition,
  AgentTeamMembership,
  Capability,
  TeamAssignment,
  WorkflowStagePolicy,
} from "../../../lib/types";

const agent = (id: string): AgentDefinition => ({
  id,
  name: id,
  role: "developer",
  description: "",
  prompt_template_ref: "",
  allowed_tools: [],
  skill_tags: [],
  boundaries: {},
  enabled: true,
  employment_status: "active",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
});

const membership = (teamId: string, agentId: string): AgentTeamMembership => ({
  id: `${teamId}-${agentId}`,
  team_id: teamId,
  agent_id: agentId,
  title: "Member",
  is_lead: false,
  created_at: "2026-01-01T00:00:00Z",
});

describe("agentRegistryPageModel", () => {
  it("selects entities and routing policies by active ids", () => {
    const agents = [agent("agent-1"), agent("agent-2")];
    const policy = {
      id: "policy-1",
      stage_name: "coding",
      primary_roles: ["developer"],
      fallback_roles: [],
      coordinator_required: true,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    } as WorkflowStagePolicy;

    expect(selectEntityById(agents, "agent-2")?.id).toBe("agent-2");
    expect(selectEntityById(agents, null)).toBeNull();
    expect(selectPolicyByStage([policy], "coding")).toBe(policy);
    expect(selectPolicyByStage([policy], "qa_validation")).toBeNull();
  });

  it("formats capability options with hierarchy indentation", () => {
    const capabilities = [
      { id: "capability-1", name: "Checkout", level: 0 },
      { id: "feature-1", name: "Coupons", level: 2 },
    ] as Capability[];

    expect(buildCapabilityOptions(capabilities)).toEqual([
      { id: "capability-1", name: "Checkout" },
      { id: "feature-1", name: "    Coupons" },
    ]);
  });

  it("groups memberships by team and filters unassigned agents", () => {
    const agents = [agent("agent-1"), agent("agent-2"), agent("agent-3")];
    const memberships = [
      membership("team-1", "agent-1"),
      membership("team-1", "agent-2"),
    ];

    expect((buildTeamMembershipsByTeam(memberships).get("team-1") ?? []).map((entry) => entry.agent_id))
      .toEqual(["agent-1", "agent-2"]);
    expect(filterUnassignedAgents(agents, memberships).map((entry) => entry.id)).toEqual(["agent-3"]);
  });

  it("counts team assignments by scope type", () => {
    const assignments = [
      { scope_type: "product" },
      { scope_type: "product_area" },
      { scope_type: "product_area" },
      { scope_type: "capability" },
    ] as TeamAssignment[];

    expect(countAssignmentsByType(assignments)).toEqual({
      product: 1,
      product_area: 2,
      capability: 1,
    });
  });
});

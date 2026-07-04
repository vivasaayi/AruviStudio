import { describe, expect, it } from "vitest";

import {
  buildPlannerComposerScopeChips,
  buildPlannerStatusSummary,
  findLatestAssistantMessage,
  findLatestDraftPlan,
  PLANNER_COMPOSER_SCOPE_HINT,
} from "./plannerViewModel";
import type { DraftValidationSummary } from "./plannerDraftTree";
import type { PendingPlan, PlannerMessage, PlannerPlan, PlannerTreeNode } from "./plannerPageTypes";

const validation: DraftValidationSummary = {
  score: 100,
  counts: {
    product: 1,
    "product area": 2,
    capability: 3,
    "work item": 4,
  },
  issues: [],
};

describe("plannerViewModel", () => {
  it("prioritizes active voice status over other planner state", () => {
    expect(buildPlannerStatusSummary({
      voiceActivity: "Listening...",
      pendingVoiceTranscript: "create checkout",
      reviewVoiceBeforeSend: true,
      draftTreeNodeCount: 1,
      draftValidation: validation,
      selectedDraftNode: { id: "node-1", label: "Checkout", children: [] },
      pendingPlan: null,
      latestAssistantMessage: null,
    })).toEqual({
      title: "Listening...",
      detail: "The transcript is ready for review before it becomes a planner turn.",
    });
  });

  it("summarizes active draft designs with validation counts", () => {
    const selectedDraftNode: PlannerTreeNode = { id: "node-1", label: "Checkout", children: [] };

    expect(buildPlannerStatusSummary({
      voiceActivity: null,
      pendingVoiceTranscript: null,
      reviewVoiceBeforeSend: false,
      draftTreeNodeCount: 1,
      draftValidation: validation,
      selectedDraftNode,
      pendingPlan: null,
      latestAssistantMessage: null,
    })).toEqual({
      title: "Design active: 1 product, 2 product area, 3 capability/feature, 4 story/task",
      detail: "Selected node: Checkout.",
    });
  });

  it("falls back to pending plan, assistant message, then ready copy", () => {
    const pendingPlan: PendingPlan = {
      sourceText: "draft",
      plan: {
        assistant_response: "Plan ready",
        needs_confirmation: true,
        clarification_question: null,
        actions: [{ type: "report_status" }, { type: "report_tree" }],
      },
    };
    const assistantMessage: PlannerMessage = {
      id: "message-1",
      role: "assistant",
      meta: "Latest update",
      content: "First line\nSecond line",
    };

    expect(buildPlannerStatusSummary({
      voiceActivity: null,
      pendingVoiceTranscript: null,
      reviewVoiceBeforeSend: false,
      draftTreeNodeCount: 0,
      draftValidation: validation,
      selectedDraftNode: null,
      pendingPlan,
      latestAssistantMessage: assistantMessage,
    })).toEqual({
      title: "Proposal waiting for confirmation",
      detail: "2 proposed changes are ready for review.",
    });

    expect(buildPlannerStatusSummary({
      voiceActivity: null,
      pendingVoiceTranscript: null,
      reviewVoiceBeforeSend: false,
      draftTreeNodeCount: 0,
      draftValidation: validation,
      selectedDraftNode: null,
      pendingPlan: null,
      latestAssistantMessage: assistantMessage,
    })).toEqual({
      title: "Latest update",
      detail: "First line",
    });
  });

  it("builds composer scope chips in stable order", () => {
    expect(buildPlannerComposerScopeChips({
      selectedDraftNodeId: "draft-1",
      selectedProductId: "product-1",
      activeProductAreaId: "area-1",
      activeCapabilityId: "capability-1",
      activeWorkItemId: "work-1",
    })).toEqual([
      "design node selected",
      "product selected",
      "product area selected",
      "capability selected",
      "story/task selected",
    ]);
    expect(PLANNER_COMPOSER_SCOPE_HINT).toContain("selected design node");
  });

  it("selects pending draft plans before assistant message plans", () => {
    const assistantPlan: PlannerPlan = {
      assistant_response: "Older plan",
      needs_confirmation: true,
      clarification_question: null,
      actions: [{ type: "report_status" }],
    };
    const pendingPlan: PendingPlan = {
      sourceText: "new plan",
      plan: {
        assistant_response: "Pending plan",
        needs_confirmation: true,
        clarification_question: null,
        actions: [{ type: "report_tree" }],
      },
    };

    expect(findLatestDraftPlan([
      { id: "assistant-1", role: "assistant", content: "older", plan: assistantPlan },
    ], pendingPlan)).toBe(pendingPlan.plan);
  });

  it("selects the latest assistant plan with actions and latest assistant message", () => {
    const emptyPlan: PlannerPlan = {
      assistant_response: "Empty",
      needs_confirmation: false,
      clarification_question: null,
      actions: [],
    };
    const latestPlan: PlannerPlan = {
      assistant_response: "Latest",
      needs_confirmation: true,
      clarification_question: null,
      actions: [{ type: "report_status" }],
    };
    const messages: PlannerMessage[] = [
      { id: "assistant-empty", role: "assistant", content: "empty", plan: emptyPlan },
      { id: "user-1", role: "user", content: "request" },
      { id: "assistant-latest", role: "assistant", content: "latest", plan: latestPlan },
    ];

    expect(findLatestDraftPlan(messages, null)).toBe(latestPlan);
    expect(findLatestAssistantMessage(messages)?.id).toBe("assistant-latest");
  });
});

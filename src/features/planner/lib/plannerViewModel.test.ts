import { describe, expect, it } from "vitest";

import {
  buildPlannerComposerScopeChips,
  buildPlannerStatusSummary,
  PLANNER_COMPOSER_SCOPE_HINT,
} from "./plannerViewModel";
import type { DraftValidationSummary } from "./plannerDraftTree";
import type { PendingPlan, PlannerMessage, PlannerTreeNode } from "./plannerPageTypes";

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
});

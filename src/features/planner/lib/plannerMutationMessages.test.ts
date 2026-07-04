import { describe, expect, it } from "vitest";

import {
  buildPlannerMutationMessages,
  getPlannerMutationSpeechText,
} from "./plannerMutationMessages";
import type { PlannerMutationResult, PlannerPlan } from "./plannerPageTypes";

const basePlan: PlannerPlan = {
  assistant_response: "Planner response",
  needs_confirmation: false,
  clarification_question: null,
  actions: [],
};

function makeIdFactory() {
  let nextId = 0;
  return () => `id-${nextId++}`;
}

describe("plannerMutationMessages", () => {
  it("builds confirmation-required proposal messages", () => {
    const result: PlannerMutationResult = {
      mode: "confirmation_required",
      userInput: "create the design",
      plan: { ...basePlan, needs_confirmation: true },
      execution: null,
      treeNodes: [{ id: "node-1", label: "Checkout", children: [] }],
    };

    const messages = buildPlannerMutationMessages([], result, makeIdFactory());

    expect(messages).toMatchObject([
      { role: "user", content: "create the design", kind: "text" },
      {
        role: "assistant",
        content: "Planner response",
        meta: "Suggestion awaiting confirmation",
        kind: "proposal",
        treeNodes: [{ id: "node-1", label: "Checkout", children: [] }],
      },
    ]);
    expect(getPlannerMutationSpeechText(result)).toBe("Planner response. Say confirm to apply the proposal.");
  });

  it("formats executed status reports as report messages", () => {
    const result: PlannerMutationResult = {
      mode: "executed",
      userInput: "show status",
      plan: {
        ...basePlan,
        assistant_response: "Here is the status",
        actions: [{ type: "report_status" }],
      },
      execution: { lines: ["All systems nominal."], errors: [] },
    };

    const messages = buildPlannerMutationMessages([], result, makeIdFactory());

    expect(messages[1]).toMatchObject({
      role: "assistant",
      content: "Here is the status\nAll systems nominal.",
      meta: "Status report",
      kind: "report",
    });
  });

  it("includes execution errors on failed messages", () => {
    const result: PlannerMutationResult = {
      mode: "failed",
      userInput: "apply update",
      plan: basePlan,
      execution: { lines: [], errors: ["Missing capability", "Retry later"] },
    };

    const messages = buildPlannerMutationMessages([], result, makeIdFactory());

    expect(messages[1]).toMatchObject({
      role: "assistant",
      content: "Planner response\nErrors: Missing capability | Retry later",
      meta: "Planner error",
      kind: "error",
    });
  });
});

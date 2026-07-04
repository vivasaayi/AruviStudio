import { describe, expect, it } from "vitest";

import {
  getPlannerVoiceViewCommand,
  isCollapseDraftVoiceCommand,
  isDraftWideVoiceTarget,
  isExpandDraftVoiceCommand,
  parseVoiceNodeReference,
} from "./plannerVoiceCommands";

describe("plannerVoiceCommands", () => {
  it("maps view commands to planner views", () => {
    expect(getPlannerVoiceViewCommand("show design tree")).toBe("draft");
    expect(getPlannerVoiceViewCommand("open trace")).toBe("trace");
    expect(getPlannerVoiceViewCommand("back to chat")).toBe("conversation");
    expect(getPlannerVoiceViewCommand("plan a new checkout flow")).toBeNull();
  });

  it("identifies draft-wide expand and collapse commands", () => {
    expect(isExpandDraftVoiceCommand("open all branches")).toBe(true);
    expect(isExpandDraftVoiceCommand("open checkout flow")).toBe(false);
    expect(isCollapseDraftVoiceCommand("collapse tree")).toBe(true);
    expect(isCollapseDraftVoiceCommand("collapse checkout flow")).toBe(false);
    expect(isDraftWideVoiceTarget("The Draft")).toBe(false);
    expect(isDraftWideVoiceTarget("draft")).toBe(true);
    expect(isDraftWideVoiceTarget(" all ")).toBe(true);
  });

  it("parses explicit node reference prefixes", () => {
    expect(parseVoiceNodeReference("work item finish onboarding")).toEqual({
      explicitType: "work item",
      reference: "finish onboarding",
    });
    expect(parseVoiceNodeReference("product area")).toEqual({
      explicitType: "product area",
      reference: "selected product area",
    });
    expect(parseVoiceNodeReference("checkout capability")).toEqual({
      reference: "checkout capability",
    });
  });
});

import type { DraftValidationSummary } from "./plannerDraftTree";
import type { PendingPlan, PlannerMessage, PlannerTreeNode } from "./plannerPageTypes";

export const PLANNER_COMPOSER_SCOPE_HINT =
  "If you omit names, the planner first tries the selected design node, then the selected workspace scope, then asks follow-up questions if it still cannot resolve the target cleanly.";

export type PlannerStatusSummary = {
  title: string;
  detail: string;
};

export function buildPlannerStatusSummary({
  voiceActivity,
  pendingVoiceTranscript,
  reviewVoiceBeforeSend,
  draftTreeNodeCount,
  draftValidation,
  selectedDraftNode,
  pendingPlan,
  latestAssistantMessage,
}: {
  voiceActivity: string | null;
  pendingVoiceTranscript: string | null;
  reviewVoiceBeforeSend: boolean;
  draftTreeNodeCount: number;
  draftValidation: DraftValidationSummary;
  selectedDraftNode: PlannerTreeNode | null;
  pendingPlan: PendingPlan | null;
  latestAssistantMessage: PlannerMessage | null;
}): PlannerStatusSummary {
  if (voiceActivity) {
    return {
      title: voiceActivity,
      detail: pendingVoiceTranscript
        ? reviewVoiceBeforeSend
          ? "The transcript is ready for review before it becomes a planner turn."
          : "The transcript is being sent to the planner."
        : "Voice capture is in progress.",
    };
  }
  if (pendingVoiceTranscript && reviewVoiceBeforeSend) {
    return {
      title: "Voice transcript ready",
      detail: "Review or edit the transcript, then send it to the planner.",
    };
  }
  if (draftTreeNodeCount > 0) {
    return {
      title: `Design active: ${draftValidation.counts.product} product, ${draftValidation.counts["product area"]} product area, ${draftValidation.counts.capability} capability/feature, ${draftValidation.counts["work item"]} story/task`,
      detail: selectedDraftNode
        ? `Selected node: ${selectedDraftNode.label}.`
        : "Select a node and keep refining before apply.",
    };
  }
  if (pendingPlan) {
    return {
      title: "Proposal waiting for confirmation",
      detail: `${pendingPlan.plan.actions.length} proposed changes are ready for review.`,
    };
  }
  if (latestAssistantMessage) {
    return {
      title: latestAssistantMessage.meta ?? "Planner ready",
      detail: latestAssistantMessage.content.split("\n")[0] || "Describe the product area, capability, feature, story, or task you want.",
    };
  }
  return {
    title: "Planner ready",
    detail: "Describe the product area, capability, feature, story, or task you want to stage.",
  };
}

export function buildPlannerComposerScopeChips({
  selectedDraftNodeId,
  selectedProductId,
  activeProductAreaId,
  activeCapabilityId,
  activeWorkItemId,
}: {
  selectedDraftNodeId: string | null;
  selectedProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
}) {
  const chips: string[] = [];
  if (selectedDraftNodeId) {
    chips.push("design node selected");
  }
  if (selectedProductId) {
    chips.push("product selected");
  }
  if (activeProductAreaId) {
    chips.push("product area selected");
  }
  if (activeCapabilityId) {
    chips.push("capability selected");
  }
  if (activeWorkItemId) {
    chips.push("story/task selected");
  }
  return chips;
}

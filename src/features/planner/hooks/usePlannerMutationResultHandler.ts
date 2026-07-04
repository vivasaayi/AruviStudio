import type { Dispatch, SetStateAction } from "react";
import type { QueryClient } from "@tanstack/react-query";

import type { PlannerTraceEvent } from "../../../lib/types";
import {
  buildPlannerMutationMessages,
  findTreeNodePath,
  getPlannerMutationSpeechText,
  makeId,
  type PendingPlan,
  type PlannerMessage,
  type PlannerMutationResult,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";

type PlannerMutationResultHandlerInput = {
  queryClient: QueryClient;
  selectedProductId: string | null;
  draftTreeNodes: PlannerTreeNode[];
  setPendingVoiceTranscript: Dispatch<SetStateAction<string | null>>;
  setEditableVoiceTranscript: Dispatch<SetStateAction<string>>;
  setVoiceActivity: Dispatch<SetStateAction<string | null>>;
  setIsVoiceSubmitting: Dispatch<SetStateAction<boolean>>;
  setLatestTraceEvents: Dispatch<SetStateAction<PlannerTraceEvent[]>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  setDraftTreeNodes: Dispatch<SetStateAction<PlannerTreeNode[]>>;
  setPlannerView: Dispatch<SetStateAction<"conversation" | "draft" | "trace">>;
  setSelectedDraftNodeId: Dispatch<SetStateAction<string | null>>;
  setExpandedDraftNodeIds: Dispatch<SetStateAction<string[]>>;
  setPendingPlan: Dispatch<SetStateAction<PendingPlan | null>>;
  autoSpeak: boolean;
  speakAssistantReply: (text: string) => Promise<void>;
};

export function usePlannerMutationResultHandler({
  queryClient,
  selectedProductId,
  draftTreeNodes,
  setPendingVoiceTranscript,
  setEditableVoiceTranscript,
  setVoiceActivity,
  setIsVoiceSubmitting,
  setLatestTraceEvents,
  setMessages,
  setDraftTreeNodes,
  setPlannerView,
  setSelectedDraftNodeId,
  setExpandedDraftNodeIds,
  setPendingPlan,
  autoSpeak,
  speakAssistantReply,
}: PlannerMutationResultHandlerInput) {
  return (result: PlannerMutationResult) => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setIsVoiceSubmitting(false);
    setLatestTraceEvents(result.traceEvents ?? []);
    setMessages((current) => buildPlannerMutationMessages(current, result, makeId));

    if (result.draftTreeNodes) {
      setDraftTreeNodes(result.draftTreeNodes);
      if (result.draftTreeNodes.length > 0) {
        setPlannerView("draft");
      }
    }
    if (result.selectedDraftNodeId !== undefined) {
      setSelectedDraftNodeId(result.selectedDraftNodeId ?? null);
      const treeForPath = result.draftTreeNodes ?? draftTreeNodes;
      if (result.selectedDraftNodeId && treeForPath.length > 0) {
        const pathIds = findTreeNodePath(treeForPath, result.selectedDraftNodeId).map((node) => node.id);
        setExpandedDraftNodeIds((current) => Array.from(new Set([...current, ...pathIds])));
      }
    }

    if (result.mode === "confirmation_required") {
      setPendingPlan({ sourceText: result.userInput, plan: result.plan });
    } else if (result.mode === "draft_updated") {
      setPendingPlan(null);
    } else if (result.mode === "session_updated") {
      // Preserve the currently staged plan while updating draft selection or voice-driven session state.
    } else if (result.mode === "failed") {
      setPendingPlan(null);
      setPlannerView("trace");
    } else {
      setPendingPlan(null);
      if (result.mode === "executed" && !result.draftTreeNodes?.length) {
        setDraftTreeNodes([]);
        setSelectedDraftNodeId(null);
      }
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductAreas", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree", selectedProductId] });
    }

    if (autoSpeak) {
      void speakAssistantReply(getPlannerMutationSpeechText(result));
    }
  };
}

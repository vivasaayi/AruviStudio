import type { Dispatch, SetStateAction } from "react";
import type { QueryClient } from "@tanstack/react-query";

import { clearPlannerPending, confirmPlannerPlan } from "../../../lib/tauri";
import {
  makeId,
  type ExecutionResult,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import type { PlannerTraceEvent } from "../../../lib/types";

type PlannerPendingPlanActionsInput = {
  queryClient: QueryClient;
  pendingPlan: PendingPlan | null;
  setPendingPlan: Dispatch<SetStateAction<PendingPlan | null>>;
  draftTreeNodes: PlannerTreeNode[];
  setDraftTreeNodes: Dispatch<SetStateAction<PlannerTreeNode[]>>;
  sessionId: string | null;
  isPlannerBusy: boolean;
  selectedProductId: string | null;
  setLatestTraceEvents: Dispatch<SetStateAction<PlannerTraceEvent[]>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  setSelectedDraftNodeId: Dispatch<SetStateAction<string | null>>;
  setPlannerView: Dispatch<SetStateAction<"conversation" | "draft" | "trace">>;
};

export function usePlannerPendingPlanActions({
  queryClient,
  pendingPlan,
  setPendingPlan,
  draftTreeNodes,
  setDraftTreeNodes,
  sessionId,
  isPlannerBusy,
  selectedProductId,
  setLatestTraceEvents,
  setMessages,
  setSelectedDraftNodeId,
  setPlannerView,
}: PlannerPendingPlanActionsInput) {
  const confirmPendingPlan = () => {
    if ((!pendingPlan && draftTreeNodes.length === 0) || isPlannerBusy || !sessionId) {
      return;
    }
    void (async () => {
      const response = await confirmPlannerPlan(sessionId);
      const execution: ExecutionResult = {
        lines: response.execution_lines,
        errors: response.execution_errors,
      };
      const plan = pendingPlan?.plan ?? {
        assistant_response: "Applied design to catalog.",
        needs_confirmation: false,
        clarification_question: null,
        actions: [],
      };
      const treeNodes = (response.tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
      setLatestTraceEvents(response.trace_events ?? []);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: "confirm", kind: "text" },
        {
          id: makeId(),
          role: "assistant",
          content: [
            "Applied design to catalog.",
            ...execution.lines,
            ...(execution.errors.length ? [`Errors: ${execution.errors.join(" | ")}`] : []),
          ].join("\n"),
          meta: "Planner execution",
          kind: treeNodes ? "tree" : "execution",
          treeNodes,
          plan,
          traceEvents: response.trace_events ?? [],
        },
      ]);
      setPendingPlan(null);
      setDraftTreeNodes([]);
      setSelectedDraftNodeId(null);
      setPlannerView("conversation");
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductAreas", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree", selectedProductId] });
    })().catch((error) => {
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "assistant", content: String(error), meta: "Planner error", kind: "error" },
      ]);
    });
  };

  const dismissPendingPlan = () => {
    if (!pendingPlan && draftTreeNodes.length === 0) {
      return;
    }
    if (sessionId) {
      void clearPlannerPending(sessionId).catch(() => {});
    }
    setPendingPlan(null);
    setDraftTreeNodes([]);
    setSelectedDraftNodeId(null);
    setPlannerView("conversation");
  };

  return {
    confirmPendingPlan,
    dismissPendingPlan,
  };
}

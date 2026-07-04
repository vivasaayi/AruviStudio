import type { Dispatch, SetStateAction } from "react";
import { useMutation } from "@tanstack/react-query";

import {
  addPlannerDraftChild,
  createPlannerSession,
  deletePlannerDraftNode,
  renamePlannerDraftNode,
  submitPlannerTurn,
} from "../../../lib/tauri";
import {
  formatDraftChildTypeLabel,
  makeId,
  mapPlannerResponseToMutationResult,
  type DraftEditOperation,
  type PlannerMessage,
  type PlannerMutationResult,
} from "../lib/plannerPageModel";
import type { PlannerTraceEvent } from "../../../lib/types";

type PlannerTurnMutationsInput = {
  selectedProductId: string | null;
  sessionId: string | null;
  setSessionId: Dispatch<SetStateAction<string | null>>;
  providerId: string;
  modelName: string;
  selectedDraftNodeId: string | null;
  setPendingVoiceTranscript: Dispatch<SetStateAction<string | null>>;
  setEditableVoiceTranscript: Dispatch<SetStateAction<string>>;
  setVoiceActivity: Dispatch<SetStateAction<string | null>>;
  setIsVoiceSubmitting: Dispatch<SetStateAction<boolean>>;
  setLatestTraceEvents: Dispatch<SetStateAction<PlannerTraceEvent[]>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  setDraftEditError: Dispatch<SetStateAction<string | null>>;
  setDraftEditMessage: Dispatch<SetStateAction<string | null>>;
  onPlannerMutationSuccess: (result: PlannerMutationResult) => void;
};

export function usePlannerTurnMutations({
  selectedProductId,
  sessionId,
  setSessionId,
  providerId,
  modelName,
  selectedDraftNodeId,
  setPendingVoiceTranscript,
  setEditableVoiceTranscript,
  setVoiceActivity,
  setIsVoiceSubmitting,
  setLatestTraceEvents,
  setMessages,
  setDraftEditError,
  setDraftEditMessage,
  onPlannerMutationSuccess,
}: PlannerTurnMutationsInput) {
  const processMutation = useMutation<PlannerMutationResult, Error, string>({
    mutationFn: async (input: string) => {
      const userInput = input.trim();
      if (!selectedProductId) {
        throw new Error("Select a product before planning.");
      }
      let activeSessionId = sessionId;
      if (!activeSessionId) {
        const session = await createPlannerSession({
          providerId: providerId || undefined,
          modelName: modelName || undefined,
        });
        activeSessionId = session.session_id;
        setSessionId(session.session_id);
      }

      const response = await submitPlannerTurn({
        sessionId: activeSessionId,
        userInput,
        selectedDraftNodeId,
        productId: selectedProductId,
      });

      return mapPlannerResponseToMutationResult(response, userInput);
    },
    onSuccess: onPlannerMutationSuccess,
    onError: (error, userInput) => {
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      setVoiceActivity(null);
      setIsVoiceSubmitting(false);
      setLatestTraceEvents([]);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: userInput, kind: "text" },
        { id: makeId(), role: "assistant", content: error instanceof Error ? error.message : String(error), meta: "Planner error", kind: "error" },
      ]);
    },
  });

  const draftEditMutation = useMutation<PlannerMutationResult, Error, DraftEditOperation>({
    mutationFn: async (operation) => {
      if (!sessionId) {
        throw new Error("Planner session is not ready.");
      }
      switch (operation.kind) {
        case "rename": {
          const response = await renamePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
            name: operation.name,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Rename this node to "${operation.name}".`,
          );
        }
        case "add_child": {
          const response = await addPlannerDraftChild({
            sessionId,
            parentNodeId: operation.parentNodeId,
            childType: operation.childType,
            name: operation.name,
            summary: operation.summary,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Add a ${formatDraftChildTypeLabel(operation.childType).toLowerCase()} called "${operation.name}".`,
          );
        }
        case "delete": {
          const response = await deletePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
          });
          return mapPlannerResponseToMutationResult(
            response,
            "Delete this node from the staged design.",
          );
        }
      }
    },
    onSuccess: onPlannerMutationSuccess,
    onError: (error) => {
      setDraftEditError(error instanceof Error ? error.message : String(error));
      setDraftEditMessage(null);
    },
  });

  return {
    processMutation,
    draftEditMutation,
  };
}

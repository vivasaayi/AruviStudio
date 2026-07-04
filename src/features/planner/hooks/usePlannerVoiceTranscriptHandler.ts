import { useCallback, type Dispatch, type SetStateAction } from "react";

import {
  createPlannerSession,
  submitPlannerVoiceTurn,
} from "../../../lib/tauri";
import type { PlannerTraceEvent } from "../../../lib/types";
import {
  getPlannerNodeType,
  getPlannerVoiceViewCommand,
  isCollapseDraftVoiceCommand,
  isDraftWideVoiceTarget,
  isExpandDraftVoiceCommand,
  makeId,
  mapPlannerResponseToMutationResult,
  normalize,
  parseVoiceNodeReference,
  resolveVoiceNodeReference,
  type PlannerMessage,
  type PlannerMutationResult,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";

type PlannerView = "conversation" | "draft" | "trace";

type PlannerVoiceTranscriptHandlerInput = {
  autoSpeak: boolean;
  draftTreeNodes: PlannerTreeNode[];
  latestTraceEvents: PlannerTraceEvent[];
  selectedProductId: string | null;
  sessionId: string | null;
  setSessionId: Dispatch<SetStateAction<string | null>>;
  providerId: string;
  modelName: string;
  selectedDraftNodeId: string | null;
  selectedDraftNodePath: PlannerTreeNode[];
  setPendingVoiceTranscript: Dispatch<SetStateAction<string | null>>;
  setEditableVoiceTranscript: Dispatch<SetStateAction<string>>;
  setVoiceActivity: Dispatch<SetStateAction<string | null>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  setPlannerView: Dispatch<SetStateAction<PlannerView>>;
  setExpandedDraftNodeIds: Dispatch<SetStateAction<string[]>>;
  expandAllDraftNodes: () => void;
  collapseAllDraftNodes: () => void;
  onPlannerMutationSuccess: (result: PlannerMutationResult) => void;
  speakAssistantReply: (text: string) => Promise<void>;
};

export function usePlannerVoiceTranscriptHandler({
  autoSpeak,
  draftTreeNodes,
  latestTraceEvents,
  selectedProductId,
  sessionId,
  setSessionId,
  providerId,
  modelName,
  selectedDraftNodeId,
  selectedDraftNodePath,
  setPendingVoiceTranscript,
  setEditableVoiceTranscript,
  setVoiceActivity,
  setMessages,
  setPlannerView,
  setExpandedDraftNodeIds,
  expandAllDraftNodes,
  collapseAllDraftNodes,
  onPlannerMutationSuccess,
  speakAssistantReply,
}: PlannerVoiceTranscriptHandlerInput) {
  const appendVoiceCommandFeedback = useCallback((transcript: string, reply: string) => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setMessages((current) => [
      ...current,
      { id: makeId(), role: "user", content: transcript, kind: "text" },
      { id: makeId(), role: "assistant", content: reply, meta: "Voice command", kind: "text" },
    ]);
    if (autoSpeak) {
      void speakAssistantReply(reply);
    }
  }, [
    autoSpeak,
    setEditableVoiceTranscript,
    setMessages,
    setPendingVoiceTranscript,
    setVoiceActivity,
    speakAssistantReply,
  ]);

  return useCallback(async (transcript: string) => {
    const spoken = transcript.trim();
    if (!spoken) {
      return true;
    }
    const normalizedTranscript = normalize(spoken);

    const viewCommand = getPlannerVoiceViewCommand(normalizedTranscript);

    if (viewCommand === "draft") {
      if (draftTreeNodes.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no staged design tree yet.");
      } else {
        setPlannerView("draft");
        appendVoiceCommandFeedback(spoken, "Opened the design review.");
      }
      return true;
    }

    if (viewCommand === "trace") {
      if (latestTraceEvents.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no planner trace available yet.");
      } else {
        setPlannerView("trace");
        appendVoiceCommandFeedback(spoken, "Opened the latest planner trace.");
      }
      return true;
    }

    if (viewCommand === "conversation") {
      setPlannerView("conversation");
      appendVoiceCommandFeedback(spoken, "Switched back to the planner conversation.");
      return true;
    }

    if (!selectedProductId) {
      appendVoiceCommandFeedback(spoken, "Select a product before planning. Create products in the Products page, then return here to design.");
      return true;
    }

    if (isExpandDraftVoiceCommand(normalizedTranscript)) {
      setPlannerView("draft");
      expandAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
      return true;
    }

    if (isCollapseDraftVoiceCommand(normalizedTranscript)) {
      collapseAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Collapsed the staged design tree.");
      return true;
    }

    const collapseMatch = normalizedTranscript.match(/^(collapse|close)\s+(.+)$/);
    if (normalizedTranscript.startsWith("expand ") || normalizedTranscript.startsWith("open ")) {
      const targetText = spoken.replace(/^(expand|open)\s+/i, "").trim();
      if (isDraftWideVoiceTarget(targetText)) {
        setPlannerView("draft");
        expandAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
        return true;
      }
    }

    if (collapseMatch) {
      const targetText = collapseMatch[2];
      if (isDraftWideVoiceTarget(targetText)) {
        collapseAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Collapsed the staged design tree.");
        return true;
      }
      const { explicitType, reference } = parseVoiceNodeReference(targetText);
      const targetNode = resolveVoiceNodeReference(draftTreeNodes, selectedDraftNodePath, reference, explicitType);
      if (!targetNode) {
        appendVoiceCommandFeedback(spoken, `I could not find a design node matching "${targetText}".`);
        return true;
      }
      setExpandedDraftNodeIds((current) => current.filter((nodeId) => nodeId !== targetNode.id));
      appendVoiceCommandFeedback(spoken, `Collapsed ${getPlannerNodeType(targetNode)} "${targetNode.label}".`);
      return true;
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

    const response = await submitPlannerVoiceTurn({
      sessionId: activeSessionId,
      transcript: spoken,
      selectedDraftNodeId,
      productId: selectedProductId,
    });
    onPlannerMutationSuccess(mapPlannerResponseToMutationResult(response, spoken));
    return true;
  }, [
    appendVoiceCommandFeedback,
    collapseAllDraftNodes,
    draftTreeNodes,
    expandAllDraftNodes,
    latestTraceEvents.length,
    modelName,
    onPlannerMutationSuccess,
    providerId,
    selectedDraftNodeId,
    selectedDraftNodePath,
    selectedProductId,
    sessionId,
    setExpandedDraftNodeIds,
    setPlannerView,
    setSessionId,
  ]);
}

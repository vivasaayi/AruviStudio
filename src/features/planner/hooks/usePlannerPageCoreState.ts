import { useRef, useState } from "react";
import type { PlannerSpeechModelSelection } from "../lib/plannerModelSelection";
import {
  DEFAULT_ASSISTANT_OPENING,
  makeId,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import type { PlannerTraceEvent } from "../../../lib/types";

export function usePlannerPageCoreState() {
  const [plannerView, setPlannerView] = useState<"conversation" | "draft" | "trace">("conversation");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PlannerMessage[]>([
    { id: makeId(), role: "assistant", content: DEFAULT_ASSISTANT_OPENING },
  ]);
  const [pendingPlan, setPendingPlan] = useState<PendingPlan | null>(null);
  const [draftTreeNodes, setDraftTreeNodes] = useState<PlannerTreeNode[]>([]);
  const [selectedDraftNodeId, setSelectedDraftNodeId] = useState<string | null>(null);
  const [expandedDraftNodeIds, setExpandedDraftNodeIds] = useState<string[]>([]);
  const [latestTraceEvents, setLatestTraceEvents] = useState<PlannerTraceEvent[]>([]);
  const [showCompactTools, setShowCompactTools] = useState(false);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const consumedRoutePromptRef = useRef<string | null>(null);
  const activePlannerProductRef = useRef<string | null>(null);
  const plannerBusyRef = useRef(false);
  const submitVoiceTranscriptRef = useRef<(transcript: string) => Promise<void>>(async () => {});
  const speechModelSelectionRef = useRef<PlannerSpeechModelSelection | null>(null);

  return {
    activePlannerProductRef,
    composerRef,
    consumedRoutePromptRef,
    draft,
    draftTreeNodes,
    expandedDraftNodeIds,
    latestTraceEvents,
    messages,
    modelName,
    pendingPlan,
    plannerBusyRef,
    plannerView,
    providerId,
    selectedDraftNodeId,
    sessionId,
    setDraft,
    setDraftTreeNodes,
    setExpandedDraftNodeIds,
    setLatestTraceEvents,
    setMessages,
    setModelName,
    setPendingPlan,
    setPlannerView,
    setProviderId,
    setSelectedDraftNodeId,
    setSessionId,
    setShowCompactTools,
    showCompactTools,
    speechModelSelectionRef,
    submitVoiceTranscriptRef,
    transcriptRef,
  };
}

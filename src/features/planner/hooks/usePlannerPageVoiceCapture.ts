import type { Dispatch, MutableRefObject, RefObject, SetStateAction } from "react";

import type { PlannerTraceEvent } from "../../../lib/types";
import { usePlannerAssistantSpeech } from "./usePlannerAssistantSpeech";
import { usePlannerVoiceCapture } from "./usePlannerVoiceCapture";
import { usePlannerVoiceSubmission } from "./usePlannerVoiceSubmission";
import { usePlannerVoiceTranscriptHandler } from "./usePlannerVoiceTranscriptHandler";
import type {
  PlannerMessage,
  PlannerMutationResult,
  PlannerSpeechModelSelection,
  PlannerTreeNode,
} from "../lib/plannerPageModel";

type PlannerView = "conversation" | "draft" | "trace";

type PlannerPageVoiceCaptureInput = {
  voiceEnabled: boolean;
  reviewVoiceBeforeSend: boolean;
  speechLocaleSetting: string;
  speechNativeVoiceSetting: string;
  speechModelSelectionRef: MutableRefObject<PlannerSpeechModelSelection | null>;
  plannerBusyRef: MutableRefObject<boolean>;
  submitVoiceTranscriptRef: MutableRefObject<(transcript: string) => Promise<void>>;
};

export function usePlannerPageVoiceCapture({
  voiceEnabled,
  reviewVoiceBeforeSend,
  speechLocaleSetting,
  speechNativeVoiceSetting,
  speechModelSelectionRef,
  plannerBusyRef,
  submitVoiceTranscriptRef,
}: PlannerPageVoiceCaptureInput) {
  const { speakAssistantReply } = usePlannerAssistantSpeech({
    speechLocaleSetting,
    speechNativeVoiceSetting,
  });
  const voiceCapture = usePlannerVoiceCapture({
    voiceEnabled,
    reviewVoiceBeforeSend,
    getSpeechModelSelection: () => speechModelSelectionRef.current,
    speechLocaleSetting,
    isPlannerBusy: () => plannerBusyRef.current,
    onSubmitVoiceTranscript: (transcript) => submitVoiceTranscriptRef.current(transcript),
  });

  return {
    speakAssistantReply,
    ...voiceCapture,
  };
}

type PlannerPageVoiceSubmissionInput = {
  autoSpeak: boolean;
  clearPendingVoiceReview: () => void;
  collapseAllDraftNodes: () => void;
  composerRef: RefObject<HTMLTextAreaElement | null>;
  draftTreeNodes: PlannerTreeNode[];
  expandAllDraftNodes: () => void;
  isPlannerBusy: boolean;
  latestTraceEvents: PlannerTraceEvent[];
  modelName: string;
  onPlannerMutationSuccess: (result: PlannerMutationResult) => void;
  providerId: string;
  selectedDraftNodeId: string | null;
  selectedDraftNodePath: PlannerTreeNode[];
  selectedProductId: string | null;
  sessionId: string | null;
  setDraft: Dispatch<SetStateAction<string>>;
  setEditableVoiceTranscript: Dispatch<SetStateAction<string>>;
  setExpandedDraftNodeIds: Dispatch<SetStateAction<string[]>>;
  setIsVoiceSubmitting: Dispatch<SetStateAction<boolean>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  setPendingVoiceTranscript: Dispatch<SetStateAction<string | null>>;
  setPlannerView: Dispatch<SetStateAction<PlannerView>>;
  setSessionId: Dispatch<SetStateAction<string | null>>;
  setSpeechError: Dispatch<SetStateAction<string | null>>;
  setVoiceActivity: Dispatch<SetStateAction<string | null>>;
  speakAssistantReply: (text: string) => Promise<void>;
  submitVoiceTranscriptRef: MutableRefObject<(transcript: string) => Promise<void>>;
};

export function usePlannerPageVoiceSubmission({
  autoSpeak,
  clearPendingVoiceReview,
  collapseAllDraftNodes,
  composerRef,
  draftTreeNodes,
  expandAllDraftNodes,
  isPlannerBusy,
  latestTraceEvents,
  modelName,
  onPlannerMutationSuccess,
  providerId,
  selectedDraftNodeId,
  selectedDraftNodePath,
  selectedProductId,
  sessionId,
  setDraft,
  setEditableVoiceTranscript,
  setExpandedDraftNodeIds,
  setIsVoiceSubmitting,
  setMessages,
  setPendingVoiceTranscript,
  setPlannerView,
  setSessionId,
  setSpeechError,
  setVoiceActivity,
  speakAssistantReply,
  submitVoiceTranscriptRef,
}: PlannerPageVoiceSubmissionInput) {
  const handleVoiceTranscript = usePlannerVoiceTranscriptHandler({
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
  });
  usePlannerVoiceSubmission({
    clearPendingVoiceReview,
    composerRef,
    handleVoiceTranscript,
    isPlannerBusy,
    selectedProductId,
    setDraft,
    setIsVoiceSubmitting,
    setPendingVoiceTranscript,
    setSpeechError,
    setVoiceActivity,
    submitVoiceTranscriptRef,
  });
}

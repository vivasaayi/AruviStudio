import { useEffect, type Dispatch, type MutableRefObject, type RefObject, type SetStateAction } from "react";

type PlannerVoiceSubmissionInput = {
  clearPendingVoiceReview: () => void;
  composerRef: RefObject<HTMLTextAreaElement | null>;
  handleVoiceTranscript: (transcript: string) => Promise<boolean>;
  isPlannerBusy: boolean;
  selectedProductId: string | null;
  setDraft: Dispatch<SetStateAction<string>>;
  setIsVoiceSubmitting: Dispatch<SetStateAction<boolean>>;
  setPendingVoiceTranscript: Dispatch<SetStateAction<string | null>>;
  setSpeechError: Dispatch<SetStateAction<string | null>>;
  setVoiceActivity: Dispatch<SetStateAction<string | null>>;
  submitVoiceTranscriptRef: MutableRefObject<(transcript: string) => Promise<void>>;
};

export function usePlannerVoiceSubmission({
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
}: PlannerVoiceSubmissionInput) {
  const submitVoiceTranscript = async (transcript: string) => {
    if (!transcript || isPlannerBusy) {
      return;
    }
    if (!selectedProductId) {
      setSpeechError("Select a product before using Planner voice input.");
      return;
    }
    setPendingVoiceTranscript(transcript);
    setVoiceActivity("Sending voice input to the planner...");
    setIsVoiceSubmitting(true);
    try {
      const handledAsVoiceCommand = await handleVoiceTranscript(transcript);
      if (!handledAsVoiceCommand) {
        setDraft((current) => (current ? `${current.trim()} ${transcript}` : transcript));
        composerRef.current?.focus();
        setVoiceActivity("Speech recognized and added to the composer.");
      }
    } catch (error) {
      setSpeechError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsVoiceSubmitting(false);
      clearPendingVoiceReview();
    }
  };

  submitVoiceTranscriptRef.current = submitVoiceTranscript;

  useEffect(() => {
    if (typeof window === "undefined" || !window.__ARUVI_E2E__) {
      return;
    }
    window.__ARUVI_E2E__.runPlannerVoiceTranscript = async (transcript: string) => {
      const handled = await handleVoiceTranscript(transcript);
      if (!handled) {
        setDraft((current) => (current ? `${current.trim()} ${transcript.trim()}` : transcript.trim()));
      }
    };
    return () => {
      if (window.__ARUVI_E2E__) {
        delete window.__ARUVI_E2E__.runPlannerVoiceTranscript;
      }
    };
  }, [handleVoiceTranscript, setDraft]);

  return {
    submitVoiceTranscript,
  };
}

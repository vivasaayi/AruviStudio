import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";

import { transcribeAudio } from "../../../lib/tauri";
import {
  blobToBase64,
  startWavCapture,
  type ActiveAudioCapture,
} from "../../shared/voice";
import type { PlannerSpeechModelSelection } from "../lib/plannerModelSelection";

type PlannerVoiceCaptureInput = {
  voiceEnabled: boolean;
  reviewVoiceBeforeSend: boolean;
  getSpeechModelSelection: () => PlannerSpeechModelSelection | null;
  isPlannerBusy: () => boolean;
  onSubmitVoiceTranscript: (transcript: string) => Promise<void>;
};

export function usePlannerVoiceCapture({
  voiceEnabled,
  reviewVoiceBeforeSend,
  getSpeechModelSelection,
  isPlannerBusy,
  onSubmitVoiceTranscript,
}: PlannerVoiceCaptureInput) {
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isVoiceSubmitting, setIsVoiceSubmitting] = useState(false);
  const [pendingVoiceTranscript, setPendingVoiceTranscript] = useState<string | null>(null);
  const [editableVoiceTranscript, setEditableVoiceTranscript] = useState("");
  const [voiceActivity, setVoiceActivity] = useState<string | null>(null);
  const [voiceCaptureStartedAt, setVoiceCaptureStartedAt] = useState<number | null>(null);
  const [voiceElapsedMs, setVoiceElapsedMs] = useState<number>(0);
  const [speechError, setSpeechError] = useState<string | null>(null);
  const audioCaptureRef = useRef<ActiveAudioCapture | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const pendingCaptureRef = useRef<AbortController | null>(null);

  const transcribeAudioMutation = useMutation<string, Error, { audioBytesBase64: string; mimeType: string }>({
    mutationFn: async ({ audioBytesBase64, mimeType }) => {
      const speechModelSelection = getSpeechModelSelection();
      const response = await transcribeAudio({
        fallbackProviderId: speechModelSelection?.useServerDefaults ? undefined : speechModelSelection?.providerId,
        fallbackModelName: speechModelSelection?.useServerDefaults ? undefined : speechModelSelection?.modelName,
        audioBytesBase64,
        mimeType,
        fallbackLocale: "en-US",
      });
      return response.transcript;
    },
    onError: (error) => {
      setSpeechError(error instanceof Error ? error.message : String(error));
    },
  });

  const clearPendingVoiceReview = () => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setVoiceElapsedMs(0);
  };

  const stopVoiceCapture = async (shouldTranscribe: boolean) => {
    const pending = pendingCaptureRef.current;
    pendingCaptureRef.current = null;
    pending?.abort();
    const capture = audioCaptureRef.current;
    if (!capture) {
      if (pending) {
        setIsListening(false);
        setVoiceActivity(null);
        setVoiceCaptureStartedAt(null);
      }
      return;
    }

    audioCaptureRef.current = null;
    mediaStreamRef.current = null;
    setIsListening(false);
    setVoiceCaptureStartedAt(null);

    try {
      const blob = await capture.stop();
      if (!shouldTranscribe || blob.size === 0) {
        setVoiceActivity(null);
        return;
      }

      setVoiceActivity("Transcribing audio...");
      setIsTranscribing(true);
      const audioBytesBase64 = await blobToBase64(blob);
      const transcript = await transcribeAudioMutation.mutateAsync({
        audioBytesBase64,
        mimeType: blob.type || "audio/wav",
      });
      const trimmedTranscript = transcript.trim();
      setIsTranscribing(false);
      if (!trimmedTranscript) {
        setVoiceActivity("No speech detected.");
        return;
      }
      if (reviewVoiceBeforeSend) {
        setPendingVoiceTranscript(trimmedTranscript);
        setEditableVoiceTranscript(trimmedTranscript);
        setVoiceActivity("Speech recognized. Review or edit before sending.");
        return;
      }
      setEditableVoiceTranscript(trimmedTranscript);
      setVoiceActivity("Speech recognized. Sending it to the planner...");
      await onSubmitVoiceTranscript(trimmedTranscript);
    } catch (error) {
      if (shouldTranscribe) {
        setSpeechError(error instanceof Error ? error.message : String(error));
      }
      setIsTranscribing(false);
      setIsVoiceSubmitting(false);
      setVoiceActivity(null);
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
    } finally {
      setIsTranscribing(false);
    }
  };

  const stopVoiceCaptureRef = useRef(stopVoiceCapture);
  stopVoiceCaptureRef.current = stopVoiceCapture;

  useEffect(() => {
    if (!voiceEnabled) {
      void stopVoiceCapture(false);
      setIsListening(false);
      return;
    }
    return () => {
      void stopVoiceCapture(false);
    };
  }, [voiceEnabled]);

  useEffect(() => {
    if (!isListening || !voiceCaptureStartedAt) {
      return undefined;
    }
    const interval = window.setInterval(() => {
      setVoiceElapsedMs(Date.now() - voiceCaptureStartedAt);
    }, 250);
    return () => window.clearInterval(interval);
  }, [isListening, voiceCaptureStartedAt]);

  const submitPendingVoiceTranscript = async () => {
    const transcript = editableVoiceTranscript.trim();
    if (!transcript || isPlannerBusy()) {
      return;
    }
    await onSubmitVoiceTranscript(transcript);
  };

  const retryVoiceCapture = async () => {
    clearPendingVoiceReview();
    await toggleListening();
  };

  const toggleListening = async () => {
    if (!voiceEnabled) {
      setSpeechError("Voice input is disabled.");
      return;
    }
    if (isListening) {
      await stopVoiceCapture(true);
      return;
    }
    if (pendingCaptureRef.current) {
      await stopVoiceCapture(false);
      setVoiceActivity(null);
      setVoiceCaptureStartedAt(null);
      return;
    }

    if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      setSpeechError("Microphone access is not available in this runtime.");
      return;
    }
    if (typeof window === "undefined" || (!window.AudioContext && !(window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext)) {
      setSpeechError("PCM audio capture is not supported in this runtime.");
      return;
    }

    const controller = new AbortController();
    pendingCaptureRef.current = controller;
    try {
      setSpeechError(null);
      setVoiceActivity("Waiting for microphone access...");
      setVoiceElapsedMs(0);
      setVoiceCaptureStartedAt(null);
      setIsListening(true);
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      const capture = await startWavCapture(controller.signal);
      if (controller.signal.aborted || pendingCaptureRef.current !== controller) {
        await capture.stop().catch(() => undefined);
        return;
      }
      audioCaptureRef.current = capture;
      mediaStreamRef.current = capture.stream;
      setVoiceActivity("Listening (up to 30 seconds)...");
      setVoiceCaptureStartedAt(Date.now());
      // Automatic completion uses the same path as a manual stop. Clearing the
      // ref before manual stop or unmount prevents duplicate transcription.
      void capture.completed.then(
        () => { if (audioCaptureRef.current === capture) void stopVoiceCaptureRef.current(true); },
        () => { if (audioCaptureRef.current === capture) void stopVoiceCaptureRef.current(true); },
      );
    } catch (error) {
      if (controller.signal.aborted) return;
      setSpeechError(error instanceof Error ? error.message : String(error));
      setIsListening(false);
      setVoiceActivity(null);
      setVoiceCaptureStartedAt(null);
      mediaStreamRef.current?.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
      audioCaptureRef.current = null;
    } finally {
      if (pendingCaptureRef.current === controller) pendingCaptureRef.current = null;
    }
  };

  return {
    isListening,
    isTranscribing,
    isVoiceSubmitting,
    setIsVoiceSubmitting,
    pendingVoiceTranscript,
    setPendingVoiceTranscript,
    editableVoiceTranscript,
    setEditableVoiceTranscript,
    voiceActivity,
    setVoiceActivity,
    voiceElapsedMs,
    speechError,
    setSpeechError,
    isVoiceCaptureBusy: transcribeAudioMutation.isPending || isVoiceSubmitting,
    clearPendingVoiceReview,
    submitPendingVoiceTranscript,
    retryVoiceCapture,
    toggleListening,
  };
}

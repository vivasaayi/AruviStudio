import { useEffect, useState } from "react";
import { Alert } from "react-native";
import {
  AudioModule,
  setAudioModeAsync,
} from "expo-audio";
import * as Speech from "expo-speech";
import * as SecureStore from "expo-secure-store";
import { PlannerMobileClient } from "../api/client";
import {
  getLoopbackFallbackBaseUrl,
  isNetworkRequestFailure,
  normalizeBaseUrlForDisplay,
} from "../lib/mobileConnection";
import { MOBILE_STORAGE_KEYS } from "../lib/mobileStorageKeys";
import {
  normalizeWhisperLanguage,
  WHISPER_MODELS,
} from "../lib/mobileVoice";
import type { MobilePlannerToolTraceEntry } from "../types";

export type VoiceMode = "assistant" | "planner";

type VoicePromptSource = "typed" | "recording";

type VoiceMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};

type VoiceAudioRecorder = {
  prepareToRecordAsync: () => Promise<void>;
  record: () => void;
  stop: () => Promise<void>;
  uri: string | null;
  getStatus: () => {
    url?: string | null;
  };
};

type SubmitPlannerPromptResult = {
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};

type MobileVoiceControllerInput = {
  mobileClient: PlannerMobileClient;
  baseUrl: string;
  token: string;
  providerId: string;
  modelName: string;
  locale: string;
  readReplies: boolean;
  canUseLocalSpeech: boolean;
  activeLocalWhisperModelId: string | null;
  audioRecorder: VoiceAudioRecorder;
  isRecorderRecording: boolean;
  setNativeVoiceStatus: (status: string) => void;
  submitPlannerPrompt: (prompt: string) => Promise<SubmitPlannerPromptResult>;
  transcribeRecording: (uri: string) => Promise<string>;
  setConnectionStatus: (status: "unchecked" | "checking" | "connected" | "offline") => void;
  applyFallbackBaseUrl: (fallbackBaseUrl: string) => Promise<void>;
  switchTab: (nextTab: "models") => void;
  describeError: (error: unknown) => string;
};

function createId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.round(Math.random() * 100000)}`;
}

export function useMobileVoiceController({
  mobileClient,
  baseUrl,
  token,
  providerId,
  modelName,
  locale,
  readReplies,
  canUseLocalSpeech,
  activeLocalWhisperModelId,
  audioRecorder,
  isRecorderRecording,
  setNativeVoiceStatus,
  submitPlannerPrompt,
  transcribeRecording,
  setConnectionStatus,
  applyFallbackBaseUrl,
  switchTab,
  describeError,
}: MobileVoiceControllerInput) {
  type ChatCompletionBody = Parameters<PlannerMobileClient["runChatCompletion"]>[0];

  const [voiceMode, setVoiceMode] = useState<VoiceMode>("assistant");
  const [isVoiceBusy, setIsVoiceBusy] = useState(false);
  const [lastVoiceTranscript, setLastVoiceTranscript] = useState("");
  const [voiceDraft, setVoiceDraft] = useState("");
  const [voiceMessages, setVoiceMessages] = useState<VoiceMessage[]>([
    {
      id: "assistant-welcome",
      role: "assistant",
      content: "Ready when you are. Tap the mic and speak naturally.",
    },
  ]);

  useEffect(() => {
    void setAudioModeAsync({
      allowsRecording: true,
      playsInSilentMode: true,
    });

    return () => {
      void Speech.stop();
    };
  }, []);

  const switchVoiceMode = (nextMode: VoiceMode) => {
    setVoiceMode(nextMode);
    void SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.voiceMode, nextMode);
    setNativeVoiceStatus(nextMode === "planner" ? "Planner chat ready" : "Ready");
  };

  const speakAssistantReply = (text: string) => {
    if (!readReplies || !text.trim()) return;
    const language = normalizeWhisperLanguage(locale);
    void Speech.stop();
    setNativeVoiceStatus("Reading reply...");
    Speech.speak(text, {
      language: language === "auto" ? undefined : language,
      rate: 0.95,
      onDone: () => setNativeVoiceStatus("Reply ready"),
      onStopped: () => setNativeVoiceStatus("Reply ready"),
      onError: () => setNativeVoiceStatus("Reply ready"),
    });
  };

  const runChatCompletionWithFallback = async (body: ChatCompletionBody) => {
    try {
      const response = await mobileClient.runChatCompletion(body);
      setConnectionStatus("connected");
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim()).runChatCompletion(body);
          await applyFallbackBaseUrl(fallbackBaseUrl);
          return response;
        } catch {
          throw new Error(
            `Cannot reach Aruvi at ${normalizeBaseUrlForDisplay(baseUrl)} or ${fallbackBaseUrl}. Check Settings base URL and that the desktop bridge is running.`,
          );
        }
      }
      throw error;
    }
  };

  const submitVoicePrompt = async (prompt: string, source: VoicePromptSource = "typed") => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using chat.");
      return;
    }

    const userMessage: VoiceMessage = {
      id: createId("voice-user"),
      role: "user",
      content: trimmed,
    };
    const history = voiceMessages
      .filter((message) => message.id !== "assistant-welcome")
      .slice(-18);
    setVoiceMessages((current) => [...current.filter((message) => message.id !== "assistant-welcome"), userMessage]);
    setVoiceDraft("");
    setNativeVoiceStatus(
      voiceMode === "planner"
        ? "Planning with MCP..."
        : source === "recording"
          ? "Sending voice prompt..."
          : "Thinking...",
    );
    setIsVoiceBusy(true);

    try {
      const assistantResult = voiceMode === "planner"
        ? await submitPlannerPrompt(trimmed)
        : await runChatCompletionWithFallback({
            provider_id: providerId.trim() || undefined,
            model_name: modelName.trim() || undefined,
            messages: [
              {
                role: "system",
                content: "You are Aruvi Studio's mobile voice assistant. Reply conversationally in one or two short sentences for spoken playback.",
              },
              ...history.map((message) => ({
                role: message.role,
                content: message.content,
              })),
              {
                role: "user",
                content: trimmed,
              },
            ],
            temperature: 0.7,
            max_tokens: 4096,
          }).then((response) => ({
            content: response.content.trim() || "(empty response)",
            toolTrace: undefined,
          }));
      const assistantText = assistantResult.content;
      const assistantMessage: VoiceMessage = {
        id: createId("voice-assistant"),
        role: "assistant",
        content: assistantText,
        toolTrace: assistantResult.toolTrace,
      };
      setVoiceMessages((current) => [...current, assistantMessage].slice(-24));
      if (readReplies) {
        speakAssistantReply(assistantText);
      } else {
        setNativeVoiceStatus("Reply ready");
      }
    } catch (error) {
      const message = describeError(error);
      const title = source === "recording" ? "Voice failed" : "Chat failed";
      setNativeVoiceStatus(`${title}: ${message}`);
      Alert.alert(title, message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const stopNativeVoiceRecording = async () => {
    try {
      setIsVoiceBusy(true);
      setNativeVoiceStatus("Stopping...");
      await audioRecorder.stop();
      const recordingUri = audioRecorder.uri ?? audioRecorder.getStatus().url;
      if (!recordingUri) {
        throw new Error("Recording did not produce an audio file.");
      }
      setNativeVoiceStatus("Transcribing...");
      const transcript = await transcribeRecording(recordingUri);
      if (!transcript) {
        setLastVoiceTranscript("");
        setNativeVoiceStatus("No speech detected");
        return;
      }
      setLastVoiceTranscript(transcript);
      setVoiceDraft(transcript);
      setNativeVoiceStatus("Sending...");
      await submitVoicePrompt(transcript, "recording");
    } catch (error) {
      const message = describeError(error);
      setNativeVoiceStatus(message);
      Alert.alert("Voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const startNativeVoiceRecording = async () => {
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using voice chat.");
      return;
    }
    if (!canUseLocalSpeech) {
      Alert.alert("Install model first", "Install an on-device Whisper model before using voice recording.");
      switchTab("models");
      return;
    }
    try {
      setIsVoiceBusy(true);
      void Speech.stop();
      const permission = await AudioModule.requestRecordingPermissionsAsync();
      if (!permission.granted) {
        throw new Error("Microphone permission was denied.");
      }
      await setAudioModeAsync({
        allowsRecording: true,
        playsInSilentMode: true,
      });
      await audioRecorder.prepareToRecordAsync();
      audioRecorder.record();
      setLastVoiceTranscript("");
      setNativeVoiceStatus("Listening...");
    } catch (error) {
      const message = describeError(error);
      setNativeVoiceStatus(message);
      Alert.alert("Voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const toggleNativeVoiceRecording = async () => {
    if (isRecorderRecording) {
      await stopNativeVoiceRecording();
    } else {
      await startNativeVoiceRecording();
    }
  };

  const speechModelLabel = canUseLocalSpeech
    ? `On-device ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModelId)?.label ?? "Whisper"}`
    : "Install Whisper";
  const speechModelDescription = canUseLocalSpeech
    ? `Using ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModelId)?.label ?? "Whisper"} on this phone for speech-to-text.`
    : "Type a message, or install Whisper to use the mic.";
  const voiceComposerStatus = token.trim()
    ? canUseLocalSpeech
      ? speechModelLabel
      : "Text chat ready"
    : "Setup required";

  return {
    voiceMode,
    setVoiceMode,
    isVoiceBusy,
    setIsVoiceBusy,
    lastVoiceTranscript,
    voiceDraft,
    setVoiceDraft,
    voiceMessages,
    speechModelDescription,
    speechModelLabel,
    voiceComposerStatus,
    switchVoiceMode,
    speakAssistantReply,
    submitVoicePrompt,
    toggleNativeVoiceRecording,
  };
}

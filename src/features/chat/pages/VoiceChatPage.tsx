import React, { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  getSetting,
  listModelDefinitions,
  listProviders,
  speakTextNatively,
  startModelChatStream,
  transcribeAudio,
} from "../../../lib/tauri";
import type { ChatMessagePayload, ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  blobToBase64,
  speakInBrowserAsync,
  startSilenceAwareWavCapture,
  type SilenceAwareAudioCapture,
} from "../../shared/voice";

const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 14, height: "100%" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" as const },
  titleWrap: { display: "flex", flexDirection: "column", gap: 4 },
  title: { margin: 0, fontSize: 22, fontWeight: 800, color: "#f3f6fb" },
  subtitle: { fontSize: 13, color: "#97a0af", maxWidth: 720 },
  panel: { backgroundColor: "#252526", border: "1px solid #333841", borderRadius: 16, padding: 16 },
  headerControls: { display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" as const },
  select: {
    minWidth: 260,
    padding: "10px 12px",
    backgroundColor: "#181b20",
    border: "1px solid #3c4452",
    borderRadius: 10,
    color: "#eef2f8",
    fontSize: 13,
  },
  statusCard: {
    background: "linear-gradient(180deg, rgba(18,34,58,0.95), rgba(18,29,42,0.95))",
    border: "1px solid #2f4969",
    borderRadius: 16,
    padding: 18,
    display: "flex",
    flexDirection: "column",
    gap: 12,
  },
  statusTitle: { fontSize: 18, fontWeight: 800, color: "#f4f8ff" },
  statusBody: { fontSize: 14, color: "#c7d3e3", lineHeight: 1.45 },
  chips: { display: "flex", gap: 8, flexWrap: "wrap" as const },
  chip: { backgroundColor: "#223851", color: "#dce9ff", borderRadius: 999, padding: "4px 10px", fontSize: 12, fontWeight: 700 },
  sessionRow: { display: "flex", gap: 10, flexWrap: "wrap" as const },
  btn: {
    padding: "11px 16px",
    fontSize: 14,
    backgroundColor: "#0e639c",
    color: "#fff",
    border: "none",
    borderRadius: 10,
    cursor: "pointer",
    fontWeight: 800,
  },
  btnGhost: {
    padding: "11px 16px",
    fontSize: 14,
    backgroundColor: "#2d323a",
    color: "#e0e5ee",
    border: "1px solid #3f4754",
    borderRadius: 10,
    cursor: "pointer",
    fontWeight: 800,
  },
  btnDanger: {
    padding: "11px 16px",
    fontSize: 14,
    backgroundColor: "#7d2a2a",
    color: "#fff",
    border: "none",
    borderRadius: 10,
    cursor: "pointer",
    fontWeight: 800,
  },
  transcript: {
    backgroundColor: "#171a20",
    border: "1px solid #313846",
    borderRadius: 14,
    padding: 14,
    minHeight: 84,
    color: "#edf2fa",
    fontSize: 15,
    lineHeight: 1.45,
  },
  conversation: {
    flex: 1,
    minHeight: 0,
    backgroundColor: "#212327",
    border: "1px solid #32353d",
    borderRadius: 16,
    padding: 14,
    overflowY: "auto" as const,
    display: "flex",
    flexDirection: "column",
    gap: 12,
  },
  bubbleUser: {
    alignSelf: "flex-end",
    maxWidth: "78%",
    backgroundColor: "#0e639c",
    borderRadius: 14,
    padding: "11px 13px",
    color: "#fff",
    whiteSpace: "pre-wrap" as const,
  },
  bubbleAssistant: {
    alignSelf: "flex-start",
    maxWidth: "85%",
    backgroundColor: "#2b2f37",
    borderRadius: 14,
    padding: "11px 13px",
    color: "#e9edf6",
    whiteSpace: "pre-wrap" as const,
  },
  bubbleMeta: { fontSize: 11, color: "#8f96a3", marginBottom: 4, fontWeight: 700, textTransform: "uppercase" as const },
  error: { color: "#ff8d8d", fontSize: 13 },
  helper: { color: "#8f96a3", fontSize: 12 },
  composer: { display: "flex", gap: 10, alignItems: "stretch" },
  textarea: {
    flex: 1,
    minHeight: 74,
    padding: "10px 12px",
    backgroundColor: "#181a1f",
    border: "1px solid #3c4048",
    borderRadius: 12,
    color: "#e0e0e0",
    resize: "vertical" as const,
    fontSize: 13,
  },
  footerActions: { display: "flex", flexDirection: "column", gap: 8 },
};

type LocalChatMessage = ChatMessagePayload & { id: string };

function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

function stopBrowserSpeech() {
  if (typeof window !== "undefined" && "speechSynthesis" in window) {
    window.speechSynthesis.cancel();
  }
}

export function VoiceChatPage() {
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [systemPrompt, setSystemPrompt] = useState(
    "You are a concise, capable voice assistant. Keep replies natural and easy to speak aloud.",
  );
  const [messages, setMessages] = useState<LocalChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [sessionActive, setSessionActive] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [status, setStatus] = useState("Ready. Start a voice session and speak naturally.");
  const [lastTranscript, setLastTranscript] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [speechProviderId, setSpeechProviderId] = useState("");
  const [speechModelName, setSpeechModelName] = useState("");
  const [speechLocale, setSpeechLocale] = useState("en-US");
  const [speechNativeVoice, setSpeechNativeVoice] = useState("");
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const audioCaptureRef = useRef<SilenceAwareAudioCapture | null>(null);
  const messagesRef = useRef<LocalChatMessage[]>([]);
  const sessionActiveRef = useRef(false);
  const isLoopingRef = useRef(false);
  const speakQueueRef = useRef(Promise.resolve());

  const { data: providers = [] } = useQuery<ModelProvider[]>({
    queryKey: ["voiceChatProviders"],
    queryFn: listProviders,
  });
  const { data: models = [] } = useQuery<ModelDefinition[]>({
    queryKey: ["voiceChatModels"],
    queryFn: listModelDefinitions,
  });

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  useEffect(() => {
    sessionActiveRef.current = sessionActive;
  }, [sessionActive]);

  useEffect(() => {
    void Promise.all([
      getSetting(SPEECH_PROVIDER_KEY),
      getSetting(SPEECH_MODEL_KEY),
      getSetting(SPEECH_LOCALE_KEY),
      getSetting(SPEECH_NATIVE_VOICE_KEY),
      getSetting(SPEECH_ENABLE_MIC_KEY),
    ]).then(([providerSetting, modelSetting, localeSetting, nativeVoiceSetting, micEnabledSetting]) => {
      if (providerSetting) setSpeechProviderId(providerSetting);
      if (modelSetting) setSpeechModelName(modelSetting);
      if (localeSetting) setSpeechLocale(localeSetting);
      if (nativeVoiceSetting) setSpeechNativeVoice(nativeVoiceSetting);
      setVoiceEnabled(parseBooleanSetting(micEnabledSetting, true));
    });
  }, []);

  const enabledProviders = useMemo(
    () => providers.filter((provider) => provider.enabled),
    [providers],
  );

  const enabledModels = useMemo(
    () => models.filter((model) => model.enabled),
    [models],
  );

  const combinedModelOptions = useMemo(
    () =>
      enabledModels
        .map((model) => {
          const provider = enabledProviders.find((entry) => entry.id === model.provider_id);
          if (!provider) {
            return null;
          }
          return {
            value: `${provider.id}::${model.name}`,
            label: `${provider.name} / ${model.name}`,
            providerId: provider.id,
            modelName: model.name,
          };
        })
        .filter((entry): entry is { value: string; label: string; providerId: string; modelName: string } => Boolean(entry)),
    [enabledModels, enabledProviders],
  );

  useEffect(() => {
    if ((!providerId || !modelName) && combinedModelOptions.length > 0) {
      setProviderId(combinedModelOptions[0].providerId);
      setModelName(combinedModelOptions[0].modelName);
    }
  }, [providerId, modelName, combinedModelOptions]);

  const selectedModelValue = providerId && modelName ? `${providerId}::${modelName}` : "";

  const stopCurrentCapture = async () => {
    const capture = audioCaptureRef.current;
    if (!capture) {
      return;
    }
    audioCaptureRef.current = null;
    setIsListening(false);
    try {
      await capture.stop();
    } catch {
      // ignore stop errors during teardown
    }
  };

  useEffect(() => () => {
    void stopCurrentCapture();
    stopBrowserSpeech();
  }, []);

  const speakReply = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) {
      return;
    }
    setIsSpeaking(true);
    setStatus("Speaking reply...");
    try {
      await speakTextNatively({
        text: trimmed,
        voice: speechNativeVoice || undefined,
        locale: speechLocale || undefined,
      });
    } catch {
      await speakInBrowserAsync(trimmed);
    } finally {
      setIsSpeaking(false);
    }
  };

  const streamAssistantReply = async (conversation: LocalChatMessage[]) => {
    const assistantMessageId = crypto.randomUUID();
    let activeStreamId: string | null = null;
    let latestAssistantText = "";
    const assistantPlaceholder: LocalChatMessage = { id: assistantMessageId, role: "assistant", content: "" };
    setMessages([...conversation, assistantPlaceholder]);
    messagesRef.current = [...conversation, assistantPlaceholder];

    await new Promise<void>(async (resolve, reject) => {
      let unlistenChunk: UnlistenFn | null = null;
      let unlistenDone: UnlistenFn | null = null;
      let unlistenError: UnlistenFn | null = null;

      const cleanup = () => {
        if (unlistenChunk) void unlistenChunk();
        if (unlistenDone) void unlistenDone();
        if (unlistenError) void unlistenError();
        unlistenChunk = null;
        unlistenDone = null;
        unlistenError = null;
      };

      try {
        unlistenChunk = await listen<{ stream_id: string; delta: string }>("chat_stream_chunk", (event) => {
          if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
            return;
          }
          latestAssistantText = `${latestAssistantText}${event.payload.delta}`;
          setMessages((current) =>
            current.map((entry) =>
              entry.id === assistantMessageId
                ? { ...entry, content: `${entry.content}${event.payload.delta}` }
                : entry,
            ),
          );
        });

        unlistenDone = await listen<{ stream_id: string }>("chat_stream_done", async (event) => {
          if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
            return;
          }
          cleanup();
          resolve();
        });

        unlistenError = await listen<{ stream_id: string; error: string }>("chat_stream_error", (event) => {
          if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
            return;
          }
          cleanup();
          reject(new Error(event.payload.error));
        });

        activeStreamId = await startModelChatStream({
          providerId,
          model: modelName,
          messages: [
            {
              role: "system",
              content: systemPrompt.trim() || "You are a concise, capable voice assistant. Keep replies natural and easy to speak aloud.",
            },
            ...conversation.map(({ role, content }) => ({ role, content })),
          ],
          temperature: 0.5,
          maxTokens: 512,
        });
      } catch (streamError) {
        cleanup();
        reject(streamError);
      }
    });

    if (latestAssistantText.trim()) {
      messagesRef.current = [...conversation, { id: assistantMessageId, role: "assistant", content: latestAssistantText }];
      speakQueueRef.current = speakQueueRef.current.then(() => speakReply(latestAssistantText));
      await speakQueueRef.current;
    }
  };

  const handleTranscript = async (transcript: string) => {
    const trimmed = transcript.trim();
    if (!trimmed) {
      setStatus("I didn't catch anything. Listening again...");
      return;
    }
    setLastTranscript(trimmed);
    setStatus("Sending your words to the model...");
    setIsSending(true);
    const userMessage: LocalChatMessage = { id: crypto.randomUUID(), role: "user", content: trimmed };
    const nextConversation = [...messagesRef.current, userMessage];
    setMessages(nextConversation);
    messagesRef.current = nextConversation;
    try {
      await streamAssistantReply(nextConversation);
      setStatus("Reply finished. Listening for the next turn...");
    } catch (turnError) {
      setError(String(turnError));
      setStatus("The voice turn failed.");
    } finally {
      setIsSending(false);
    }
  };

  const listenForTurn = async () => {
    if (
      !sessionActiveRef.current
      || !voiceEnabled
      || isLoopingRef.current
      || isListening
      || isTranscribing
      || isSending
      || isSpeaking
    ) {
      return;
    }
    if (!speechProviderId || !speechModelName) {
      setError("Configure a speech transcription provider and model in Settings before using Voice Chat.");
      setSessionActive(false);
      return;
    }
    if (!providerId || !modelName) {
      setError("Select a chat model first.");
      setSessionActive(false);
      return;
    }

    isLoopingRef.current = true;
    setError(null);
    setStatus("Listening... Speak naturally and pause when you're done.");
    try {
      const capture = await startSilenceAwareWavCapture({
        silenceDurationMs: 1100,
        minSpeechDurationMs: 500,
        maxDurationMs: 18000,
      });
      audioCaptureRef.current = capture;
      setIsListening(true);
      const audioBlob = await capture.completed;
      if (audioCaptureRef.current === capture) {
        audioCaptureRef.current = null;
      }
      setIsListening(false);
      setIsTranscribing(true);
      setStatus("Transcribing your speech...");
      const transcriptResponse = await transcribeAudio({
        providerId: speechProviderId,
        modelName: speechModelName,
        audioBytesBase64: await blobToBase64(audioBlob),
        mimeType: audioBlob.type || "audio/wav",
        locale: speechLocale || undefined,
      });
      setIsTranscribing(false);
      await handleTranscript(transcriptResponse.transcript);
    } catch (listenError) {
      setIsListening(false);
      setIsTranscribing(false);
      if (sessionActiveRef.current) {
        setError(String(listenError));
        setStatus("Voice capture failed.");
      }
    } finally {
      audioCaptureRef.current = null;
      isLoopingRef.current = false;
    }
  };

  useEffect(() => {
    if (!sessionActive || !voiceEnabled) {
      return;
    }
    if (isListening || isTranscribing || isSending || isSpeaking) {
      return;
    }
    const timer = window.setTimeout(() => {
      void listenForTurn();
    }, 350);
    return () => window.clearTimeout(timer);
  }, [sessionActive, voiceEnabled, isListening, isTranscribing, isSending, isSpeaking, providerId, modelName, speechProviderId, speechModelName]);

  const startSession = () => {
    setError(null);
    setSessionActive(true);
    setStatus("Voice session started. I'll listen, answer, and continue automatically.");
  };

  const stopSession = async () => {
    setSessionActive(false);
    setStatus("Voice session stopped.");
    await stopCurrentCapture();
    stopBrowserSpeech();
  };

  const sendTypedMessage = async () => {
    const content = draft.trim();
    if (!content || isSending) {
      return;
    }
    setDraft("");
    await handleTranscript(content);
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleWrap}>
          <h1 style={styles.title}>Voice Chat</h1>
          <div style={styles.subtitle}>
            Start one voice session and keep talking. This tab auto-transcribes, sends your words to the model, speaks the reply back, and then listens again.
          </div>
        </div>
        <div style={styles.headerControls}>
          <select
            style={styles.select}
            value={selectedModelValue}
            onChange={(event) => {
              const [nextProviderId, nextModelName] = event.target.value.split("::");
              setProviderId(nextProviderId ?? "");
              setModelName(nextModelName ?? "");
            }}
            aria-label="Voice chat model"
            data-testid="voice-chat-model-picker"
          >
            <option value="">Select model</option>
            {combinedModelOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          {!sessionActive ? (
            <button style={styles.btn} onClick={startSession} disabled={!voiceEnabled} data-testid="voice-chat-start">
              Start Voice Session
            </button>
          ) : (
            <button style={styles.btnDanger} onClick={() => void stopSession()} data-testid="voice-chat-stop">
              End Voice Session
            </button>
          )}
        </div>
      </div>

      <div style={styles.statusCard}>
        <div style={styles.statusTitle}>{sessionActive ? "Voice session live" : "Voice session idle"}</div>
        <div style={styles.statusBody}>{status}</div>
        <div style={styles.chips}>
          <div style={styles.chip}>{voiceEnabled ? "Mic enabled" : "Mic disabled in Settings"}</div>
          <div style={styles.chip}>{speechModelName || "No speech model configured"}</div>
          <div style={styles.chip}>
            {isListening
              ? "listening"
              : isTranscribing
                ? "transcribing"
                : isSending
                  ? "thinking"
                  : isSpeaking
                    ? "speaking"
                    : sessionActive
                      ? "waiting for next turn"
                      : "ready"}
          </div>
        </div>
        <div style={styles.transcript}>
          {lastTranscript
            ? lastTranscript
            : "Your latest recognized speech will appear here before the assistant answers."}
        </div>
        {error ? <div style={styles.error}>{error}</div> : null}
      </div>

      <div style={styles.conversation} data-testid="voice-chat-conversation">
        {messages.length === 0 ? (
          <div style={styles.helper}>Start a voice session and say something. The conversation transcript will accumulate here.</div>
        ) : (
          messages.map((message) => (
            <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
              <div style={styles.bubbleMeta}>{message.role === "user" ? "You said" : "Assistant replied"}</div>
              {message.content}
            </div>
          ))
        )}
      </div>

      <div style={styles.panel}>
        <div style={styles.helper}>Fallback text input. Useful when you want to test the same voice conversation model path without recording audio.</div>
        <div style={{ ...styles.composer, marginTop: 10 }}>
          <textarea
            style={styles.textarea}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Type here if you want to inject a message without speaking."
            data-testid="voice-chat-input"
          />
          <div style={styles.footerActions}>
            <button style={styles.btnGhost} onClick={() => void sendTypedMessage()} disabled={isSending || !draft.trim()}>
              Send Text
            </button>
            <button style={styles.btnGhost} onClick={() => setMessages([])} disabled={isListening || isTranscribing || isSending || isSpeaking}>
              Clear Transcript
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}


import { useEffect, useMemo, useRef, useState } from "react";
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
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { VoiceChatConversation } from "../components/VoiceChatConversation";
import { VoiceChatHeader } from "../components/VoiceChatHeader";
import type { VoiceChatModelOption } from "../components/VoiceChatHeader";
import { VoiceChatStatusCard } from "../components/VoiceChatStatusCard";
import { VoiceChatTextComposer } from "../components/VoiceChatTextComposer";
import {
  blobToBase64,
  speakInBrowserAsync,
  startSilenceAwareWavCapture,
  type SilenceAwareAudioCapture,
} from "../../shared/voice";
import { styles } from "../lib/voiceChatPageStyles";
import {
  parseBooleanSetting,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
  stopBrowserSpeech,
} from "../lib/voiceChatSettings";
import type { LocalChatMessage } from "../lib/voiceChatTypes";

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
  const sessionIdRef = useRef(crypto.randomUUID());

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
        .filter((entry): entry is VoiceChatModelOption => Boolean(entry)),
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
          sourceKind: "desktop_voice_chat",
          sourceId: sessionIdRef.current,
          sourceLabel: "Desktop Voice Chat",
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
    if (messagesRef.current.length === 0) {
      sessionIdRef.current = crypto.randomUUID();
    }
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

  const clearTranscript = () => {
    setMessages([]);
    messagesRef.current = [];
    sessionIdRef.current = crypto.randomUUID();
  };

  return (
    <div style={styles.page}>
      <VoiceChatHeader
        selectedModelValue={selectedModelValue}
        modelOptions={combinedModelOptions}
        sessionActive={sessionActive}
        voiceEnabled={voiceEnabled}
        onModelChange={(value) => {
          const [nextProviderId, nextModelName] = value.split("::");
          setProviderId(nextProviderId ?? "");
          setModelName(nextModelName ?? "");
        }}
        onStartSession={startSession}
        onStopSession={() => void stopSession()}
      />
      <VoiceChatStatusCard
        sessionActive={sessionActive}
        voiceEnabled={voiceEnabled}
        speechModelName={speechModelName}
        isListening={isListening}
        isTranscribing={isTranscribing}
        isSending={isSending}
        isSpeaking={isSpeaking}
        status={status}
        lastTranscript={lastTranscript}
        error={error}
      />
      <VoiceChatConversation messages={messages} />
      <VoiceChatTextComposer
        draft={draft}
        isSending={isSending}
        isBusy={isListening || isTranscribing || isSending || isSpeaking}
        onDraftChange={setDraft}
        onSendTypedMessage={() => void sendTypedMessage()}
        onClearTranscript={clearTranscript}
      />
    </div>
  );
}

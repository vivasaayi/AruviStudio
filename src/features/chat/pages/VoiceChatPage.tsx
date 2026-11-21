import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  speakTextNatively,
  startModelChatStream,
  transcribeAudio,
  getDesktopTranscriptionTarget,
} from "../../../lib/tauri";
import { VoiceChatPageContent } from "../components/VoiceChatPageContent";
import { useVoiceChatConfiguration } from "../hooks/useVoiceChatConfiguration";
import {
  blobToBase64,
  speakInBrowserAsync,
  startSilenceAwareWavCapture,
  type SilenceAwareAudioCapture,
} from "../../shared/voice";
import {
  stopBrowserSpeech,
} from "../lib/voiceChatSettings";
import type { LocalChatMessage } from "../lib/voiceChatTypes";

export function VoiceChatPage() {
  const {
    providerId,
    modelName,
    systemPrompt,
    speechLocale,
    speechNativeVoice,
    voiceEnabled,
    combinedModelOptions,
    selectedModelValue,
    setSelectedModelValue,
  } = useVoiceChatConfiguration();
  const [messages, setMessages] = useState<LocalChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [sessionActive, setSessionActive] = useState(false);
  const [isPreparing, setIsPreparing] = useState(false);
  const [resolvedSpeechModel, setResolvedSpeechModel] = useState("");
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [status, setStatus] = useState("Ready. Start a voice session and speak naturally.");
  const [lastTranscript, setLastTranscript] = useState("");
  const [error, setError] = useState<string | null>(null);
  const audioCaptureRef = useRef<SilenceAwareAudioCapture | null>(null);
  const messagesRef = useRef<LocalChatMessage[]>([]);
  const sessionActiveRef = useRef(false);
  const isLoopingRef = useRef<number | null>(null);
  const speakQueueRef = useRef(Promise.resolve());
  const sessionIdRef = useRef(crypto.randomUUID());
  const preflightSequenceRef = useRef(0);

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  useEffect(() => {
    sessionActiveRef.current = sessionActive;
  }, [sessionActive]);

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
    preflightSequenceRef.current += 1;
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
    const requestedStreamId = crypto.randomUUID();
    let activeStreamId: string | null = requestedStreamId;
    let latestAssistantText = "";
    const assistantPlaceholder: LocalChatMessage = { id: assistantMessageId, role: "assistant", content: "" };
    setMessages([...conversation, assistantPlaceholder]);
    messagesRef.current = [...conversation, assistantPlaceholder];

    await new Promise<void>(async (resolve, reject) => {
      let unlistenChunk: UnlistenFn | null = null;
      let unlistenDone: UnlistenFn | null = null;
      let unlistenError: UnlistenFn | null = null;

      const cleanup = () => {
        activeStreamId = null;
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

        await startModelChatStream({
          streamId: requestedStreamId,
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
    const generation = preflightSequenceRef.current;
    if (
      !sessionActiveRef.current
      || !voiceEnabled
      || isLoopingRef.current === generation
      || isListening
      || isTranscribing
      || isSending
      || isSpeaking
    ) {
      return;
    }
    if (!providerId || !modelName) {
      setError("Select a chat model first.");
      sessionActiveRef.current = false;
      setSessionActive(false);
      return;
    }

    isLoopingRef.current = generation;
    setError(null);
    setStatus("Listening... Speak naturally and pause when you're done.");
    let capture: SilenceAwareAudioCapture | null = null;
    try {
      capture = await startSilenceAwareWavCapture({
        silenceDurationMs: 1100,
        minSpeechDurationMs: 500,
        maxDurationMs: 18000,
      });
      if (!sessionActiveRef.current || generation !== preflightSequenceRef.current) { await capture.stop(); return; }
      audioCaptureRef.current = capture;
      setIsListening(true);
      const audioBlob = await capture.completed;
      if (audioCaptureRef.current === capture) {
        audioCaptureRef.current = null;
      }
      if (!sessionActiveRef.current || generation !== preflightSequenceRef.current) return;
      setIsListening(false);
      setIsTranscribing(true);
      setStatus("Transcribing your speech...");
      const audioBytesBase64 = await blobToBase64(audioBlob);
      if (!sessionActiveRef.current || generation !== preflightSequenceRef.current) return;
      const transcriptResponse = await transcribeAudio({
        audioBytesBase64,
        mimeType: audioBlob.type || "audio/wav",
        fallbackLocale: "en-US",
      });
      if (!sessionActiveRef.current || generation !== preflightSequenceRef.current) return;
      setIsTranscribing(false);
      await handleTranscript(transcriptResponse.transcript);
    } catch (listenError) {
      if (sessionActiveRef.current && generation === preflightSequenceRef.current) {
        setIsListening(false);
        setIsTranscribing(false);
        setError(String(listenError));
        setStatus("Voice capture failed.");
        sessionActiveRef.current = false;
        setSessionActive(false);
      }
    } finally {
      if (audioCaptureRef.current === capture) audioCaptureRef.current = null;
      if (isLoopingRef.current === generation) isLoopingRef.current = null;
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
  }, [sessionActive, voiceEnabled, isListening, isTranscribing, isSending, isSpeaking, providerId, modelName]);

  const startSession = async () => {
    setError(null);
    if (!voiceEnabled || isPreparing) return;
    if (!providerId || !modelName) { setError("Select a chat model first."); return; }
    const requestId = ++preflightSequenceRef.current;
    setIsPreparing(true); setStatus("Checking speech settings...");
    try {
      const target = await getDesktopTranscriptionTarget("en-US");
      if (requestId !== preflightSequenceRef.current) return;
      setResolvedSpeechModel(target.model_name);
      if (messagesRef.current.length === 0) sessionIdRef.current = crypto.randomUUID();
      sessionActiveRef.current = true;
      setSessionActive(true);
      setStatus("Voice session started. I'll listen, answer, and continue automatically.");
    } catch (preflightError) {
      if (requestId !== preflightSequenceRef.current) return;
      setResolvedSpeechModel(""); setError(String(preflightError)); setStatus("Speech settings need attention.");
    } finally {
      if (requestId === preflightSequenceRef.current) setIsPreparing(false);
    }
  };

  const stopSession = async () => {
    preflightSequenceRef.current += 1;
    sessionActiveRef.current = false;
    setSessionActive(false);
    setIsListening(false);
    setIsTranscribing(false);
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
    <VoiceChatPageContent
      selectedModelValue={selectedModelValue}
      modelOptions={combinedModelOptions}
      sessionActive={sessionActive}
      voiceEnabled={voiceEnabled}
      isPreparing={isPreparing}
      speechModelName={resolvedSpeechModel}
      isListening={isListening}
      isTranscribing={isTranscribing}
      isSending={isSending}
      isSpeaking={isSpeaking}
      status={status}
      lastTranscript={lastTranscript}
      error={error}
      messages={messages}
      draft={draft}
      onModelChange={setSelectedModelValue}
      onStartSession={() => void startSession()}
      onStopSession={() => void stopSession()}
      onDraftChange={setDraft}
      onSendTypedMessage={() => void sendTypedMessage()}
      onClearTranscript={clearTranscript}
    />
  );
}

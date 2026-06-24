import { VoiceChatConversation } from "./VoiceChatConversation";
import { VoiceChatHeader, type VoiceChatModelOption } from "./VoiceChatHeader";
import { VoiceChatStatusCard } from "./VoiceChatStatusCard";
import { VoiceChatTextComposer } from "./VoiceChatTextComposer";
import { styles } from "../lib/voiceChatPageStyles";
import type { LocalChatMessage } from "../lib/voiceChatTypes";

type VoiceChatPageContentProps = {
  selectedModelValue: string;
  modelOptions: VoiceChatModelOption[];
  sessionActive: boolean;
  voiceEnabled: boolean;
  speechModelName: string;
  isListening: boolean;
  isTranscribing: boolean;
  isSending: boolean;
  isSpeaking: boolean;
  status: string;
  lastTranscript: string;
  error: string | null;
  messages: LocalChatMessage[];
  draft: string;
  onModelChange: (value: string) => void;
  onStartSession: () => void;
  onStopSession: () => void;
  onDraftChange: (value: string) => void;
  onSendTypedMessage: () => void;
  onClearTranscript: () => void;
};

export function VoiceChatPageContent({
  selectedModelValue,
  modelOptions,
  sessionActive,
  voiceEnabled,
  speechModelName,
  isListening,
  isTranscribing,
  isSending,
  isSpeaking,
  status,
  lastTranscript,
  error,
  messages,
  draft,
  onModelChange,
  onStartSession,
  onStopSession,
  onDraftChange,
  onSendTypedMessage,
  onClearTranscript,
}: VoiceChatPageContentProps) {
  return (
    <div style={styles.page}>
      <VoiceChatHeader
        selectedModelValue={selectedModelValue}
        modelOptions={modelOptions}
        sessionActive={sessionActive}
        voiceEnabled={voiceEnabled}
        onModelChange={onModelChange}
        onStartSession={onStartSession}
        onStopSession={onStopSession}
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
        onDraftChange={onDraftChange}
        onSendTypedMessage={onSendTypedMessage}
        onClearTranscript={onClearTranscript}
      />
    </div>
  );
}

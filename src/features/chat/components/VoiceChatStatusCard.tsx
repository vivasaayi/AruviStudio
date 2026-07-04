import { styles } from "../lib/voiceChatPageStyles";

interface VoiceChatStatusCardProps {
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
}

export function VoiceChatStatusCard({
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
}: VoiceChatStatusCardProps) {
  const turnState = isListening
    ? "listening"
    : isTranscribing
      ? "transcribing"
      : isSending
        ? "thinking"
        : isSpeaking
          ? "speaking"
          : sessionActive
            ? "waiting for next turn"
            : "ready";

  return (
    <div style={styles.statusCard}>
      <div style={styles.statusTitle}>{sessionActive ? "Voice session live" : "Voice session idle"}</div>
      <div style={styles.statusBody}>{status}</div>
      <div style={styles.chips}>
        <div style={styles.chip}>{voiceEnabled ? "Mic enabled" : "Mic disabled in Settings"}</div>
        <div style={styles.chip}>{speechModelName || "No speech model configured"}</div>
        <div style={styles.chip}>{turnState}</div>
      </div>
      <div style={styles.transcript}>
        {lastTranscript
          ? lastTranscript
          : "Your latest recognized speech will appear here before the assistant answers."}
      </div>
      {error ? <div style={styles.error}>{error}</div> : null}
    </div>
  );
}

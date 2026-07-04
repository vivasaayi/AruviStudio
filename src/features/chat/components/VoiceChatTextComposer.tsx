import { styles } from "../lib/voiceChatPageStyles";

interface VoiceChatTextComposerProps {
  draft: string;
  isSending: boolean;
  isBusy: boolean;
  onDraftChange: (value: string) => void;
  onSendTypedMessage: () => void;
  onClearTranscript: () => void;
}

export function VoiceChatTextComposer({
  draft,
  isSending,
  isBusy,
  onDraftChange,
  onSendTypedMessage,
  onClearTranscript,
}: VoiceChatTextComposerProps) {
  return (
    <div style={styles.panel}>
      <div style={styles.helper}>Fallback text input. Useful when you want to test the same voice conversation model path without recording audio.</div>
      <div style={{ ...styles.composer, marginTop: 10 }}>
        <textarea
          style={styles.textarea}
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
          placeholder="Type here if you want to inject a message without speaking."
          data-testid="voice-chat-input"
        />
        <div style={styles.footerActions}>
          <button style={styles.btnGhost} onClick={onSendTypedMessage} disabled={isSending || !draft.trim()}>
            Send Text
          </button>
          <button style={styles.btnGhost} onClick={onClearTranscript} disabled={isBusy}>
            Clear Transcript
          </button>
        </div>
      </div>
    </div>
  );
}

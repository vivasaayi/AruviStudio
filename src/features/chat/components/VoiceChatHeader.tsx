import { styles } from "../lib/voiceChatPageStyles";

export interface VoiceChatModelOption {
  value: string;
  label: string;
  providerId: string;
  modelName: string;
}

interface VoiceChatHeaderProps {
  selectedModelValue: string;
  modelOptions: VoiceChatModelOption[];
  sessionActive: boolean;
  voiceEnabled: boolean;
  onModelChange: (value: string) => void;
  onStartSession: () => void;
  onStopSession: () => void;
}

export function VoiceChatHeader({
  selectedModelValue,
  modelOptions,
  sessionActive,
  voiceEnabled,
  onModelChange,
  onStartSession,
  onStopSession,
}: VoiceChatHeaderProps) {
  return (
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
          onChange={(event) => onModelChange(event.target.value)}
          aria-label="Voice chat model"
          data-testid="voice-chat-model-picker"
        >
          <option value="">Select model</option>
          {modelOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        {!sessionActive ? (
          <button style={styles.btn} onClick={onStartSession} disabled={!voiceEnabled} data-testid="voice-chat-start">
            Start Voice Session
          </button>
        ) : (
          <button style={styles.btnDanger} onClick={onStopSession} data-testid="voice-chat-stop">
            End Voice Session
          </button>
        )}
      </div>
    </div>
  );
}

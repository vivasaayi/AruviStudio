import { formatElapsedMs } from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";

type PlannerVoiceReviewCardProps = {
  voiceElapsedMs: number;
  isVoiceSubmitting: boolean;
  editableVoiceTranscript: string;
  onEditableVoiceTranscriptChange: (transcript: string) => void;
  onSubmitPendingVoiceTranscript: () => void;
  onRetryVoiceCapture: () => void;
  onClearPendingVoiceReview: () => void;
  isPlannerBusy: boolean;
};

export function PlannerVoiceReviewCard({
  voiceElapsedMs,
  isVoiceSubmitting,
  editableVoiceTranscript,
  onEditableVoiceTranscriptChange,
  onSubmitPendingVoiceTranscript,
  onRetryVoiceCapture,
  onClearPendingVoiceReview,
  isPlannerBusy,
}: PlannerVoiceReviewCardProps) {
  return (
    <div style={styles.voiceReviewCard}>
      <div style={styles.voiceReviewHeader}>
        <div>
          <div style={styles.voiceReviewTitle}>Voice Transcript Preview</div>
          <div style={styles.helper}>
            Review or edit the recognized speech before sending it to the planner.
          </div>
        </div>
        <div style={styles.chipRow}>
          <div style={styles.chip}>elapsed {formatElapsedMs(voiceElapsedMs)}</div>
          <div style={styles.chip}>{isVoiceSubmitting ? "sending" : "ready to send"}</div>
        </div>
      </div>
      <textarea
        style={{ ...styles.compactTextarea, minHeight: 88 }}
        value={editableVoiceTranscript}
        onChange={(event) => onEditableVoiceTranscriptChange(event.target.value)}
      />
      <div style={styles.inlineButtonRow}>
        <button style={styles.btn} onClick={onSubmitPendingVoiceTranscript} disabled={!editableVoiceTranscript.trim() || isPlannerBusy}>
          Send Transcript
        </button>
        <button style={styles.btnGhost} onClick={onRetryVoiceCapture} disabled={isPlannerBusy}>
          Retry
        </button>
        <button style={styles.btnDanger} onClick={onClearPendingVoiceReview} disabled={isPlannerBusy}>
          Cancel
        </button>
      </div>
    </div>
  );
}

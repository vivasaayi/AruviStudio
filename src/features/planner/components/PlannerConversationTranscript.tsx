import type React from "react";

import { PlannerAssistantMessage } from "./PlannerAssistantMessage";
import { PlannerVoiceReviewCard } from "./PlannerVoiceReviewCard";
import { styles } from "../lib/plannerPageStyles";
import type {
  PendingPlan,
  PlannerMessage,
  PlannerTreeNode,
} from "../lib/plannerPageModel";

type Props = {
  transcriptRef: React.RefObject<HTMLDivElement | null>;
  pendingVoiceTranscript: string | null;
  reviewVoiceBeforeSend: boolean;
  voiceElapsedMs: number;
  isVoiceSubmitting: boolean;
  editableVoiceTranscript: string;
  isPlannerBusy: boolean;
  messages: PlannerMessage[];
  isExportingDesignPacket: boolean;
  pendingPlan: PendingPlan | null;
  draftTreeNodes: PlannerTreeNode[];
  designPacketPath: string | null;
  designPacketError: string | null;
  onEditableVoiceTranscriptChange: (value: string) => void;
  onSubmitPendingVoiceTranscript: () => void;
  onRetryVoiceCapture: () => void;
  onClearPendingVoiceReview: () => void;
  onExportDesignReviewPacket: () => void;
  onConfirmPendingPlan: () => void;
  onDismissPendingPlan: () => void;
};

export function PlannerConversationTranscript({
  transcriptRef,
  pendingVoiceTranscript,
  reviewVoiceBeforeSend,
  voiceElapsedMs,
  isVoiceSubmitting,
  editableVoiceTranscript,
  isPlannerBusy,
  messages,
  isExportingDesignPacket,
  pendingPlan,
  draftTreeNodes,
  designPacketPath,
  designPacketError,
  onEditableVoiceTranscriptChange,
  onSubmitPendingVoiceTranscript,
  onRetryVoiceCapture,
  onClearPendingVoiceReview,
  onExportDesignReviewPacket,
  onConfirmPendingPlan,
  onDismissPendingPlan,
}: Props) {
  return (
    <div ref={transcriptRef} style={{ ...styles.transcript, flex: 1, minHeight: 0, overflow: "auto" }}>
      {pendingVoiceTranscript && reviewVoiceBeforeSend ? (
        <PlannerVoiceReviewCard
          voiceElapsedMs={voiceElapsedMs}
          isVoiceSubmitting={isVoiceSubmitting}
          editableVoiceTranscript={editableVoiceTranscript}
          onEditableVoiceTranscriptChange={onEditableVoiceTranscriptChange}
          onSubmitPendingVoiceTranscript={onSubmitPendingVoiceTranscript}
          onRetryVoiceCapture={onRetryVoiceCapture}
          onClearPendingVoiceReview={onClearPendingVoiceReview}
          isPlannerBusy={isPlannerBusy}
        />
      ) : null}
      {messages.map((message) => (
        <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
          {message.role === "assistant" ? (
            <PlannerAssistantMessage
              message={message}
              isExportingDesignPacket={isExportingDesignPacket}
              onExportDesignReviewPacket={onExportDesignReviewPacket}
              onConfirmPendingPlan={onConfirmPendingPlan}
              onDismissPendingPlan={onDismissPendingPlan}
              isPlannerBusy={isPlannerBusy}
              pendingPlan={pendingPlan}
              draftTreeNodes={draftTreeNodes}
              designPacketPath={designPacketPath}
              designPacketError={designPacketError}
            />
          ) : message.content}
          {message.meta ? <span style={styles.bubbleMeta}>{message.meta}</span> : null}
        </div>
      ))}
    </div>
  );
}

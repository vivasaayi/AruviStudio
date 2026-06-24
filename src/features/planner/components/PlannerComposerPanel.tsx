import type React from "react";

import {
  PLANNER_COMPOSER_SCOPE_HINT,
  PlannerComposer,
  type PendingPlan,
} from "../lib/plannerPageModel";

type PlannerComposerPanelProps = {
  draft: string;
  onDraftChange: (value: string) => void;
  onSend: () => void;
  onToggleListening: () => void;
  onOpenDraftWorkspace: () => void;
  onConfirm: () => void;
  onDismiss: () => void;
  isPlannerBusy: boolean;
  voiceEnabled: boolean;
  isListening: boolean;
  isTranscribing: boolean;
  isVoiceSubmitting: boolean;
  pendingVoiceTranscript: string | null;
  draftTreeNodesLength: number;
  pendingPlan: PendingPlan | null;
  voiceActivity: string | null;
  composerRef: React.RefObject<HTMLTextAreaElement | null>;
  scopeChips: string[];
  isProductSelected: boolean;
};

export function PlannerComposerPanel({
  draft,
  onDraftChange,
  onSend,
  onToggleListening,
  onOpenDraftWorkspace,
  onConfirm,
  onDismiss,
  isPlannerBusy,
  voiceEnabled,
  isListening,
  isTranscribing,
  isVoiceSubmitting,
  pendingVoiceTranscript,
  draftTreeNodesLength,
  pendingPlan,
  voiceActivity,
  composerRef,
  scopeChips,
  isProductSelected,
}: PlannerComposerPanelProps) {
  return (
    <PlannerComposer
      draft={draft}
      onDraftChange={onDraftChange}
      onSend={onSend}
      onToggleListening={onToggleListening}
      onOpenDraftWorkspace={onOpenDraftWorkspace}
      onConfirm={onConfirm}
      onDismiss={onDismiss}
      isPlannerBusy={isPlannerBusy}
      voiceEnabled={voiceEnabled}
      isListening={isListening}
      isTranscribing={isTranscribing}
      isVoiceSubmitting={isVoiceSubmitting}
      pendingVoiceTranscript={pendingVoiceTranscript}
      draftTreeNodesLength={draftTreeNodesLength}
      pendingPlan={pendingPlan}
      voiceActivity={voiceActivity}
      composerRef={composerRef}
      scopeChips={scopeChips}
      scopeHint={PLANNER_COMPOSER_SCOPE_HINT}
      isProductSelected={isProductSelected}
    />
  );
}

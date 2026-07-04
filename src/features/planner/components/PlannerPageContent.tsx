import type React from "react";

import type { PlannerDraftChildType } from "../../../lib/types";
import { PlannerConversationTranscript } from "./PlannerConversationTranscript";
import { PlannerDraftCanvas } from "./PlannerDraftCanvas";
import { PlannerDraftSidePanel } from "./PlannerDraftSidePanel";
import { PlannerTraceView } from "./PlannerTraceView";
import type {
  DraftValidationSummary,
  PendingPlan,
  PlannerAction,
  PlannerMessage,
  PlannerPlan,
  PlannerTreeNode,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";
import type { PlannerTraceEvent } from "../../../lib/types";

type PlannerView = "conversation" | "draft" | "trace";

type PlannerPageContentProps = {
  plannerView: PlannerView;
  plannerComposer: React.ReactNode;
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
  selectedDraftNode: PlannerTreeNode | null;
  draftValidation: DraftValidationSummary;
  selectedDraftNodeId: string | null;
  expandedDraftNodeIds: Set<string>;
  selectedDraftNodePath: PlannerTreeNode[];
  renameDraftName: string;
  allowedDraftChildTypes: PlannerDraftChildType[];
  draftChildType: PlannerDraftChildType;
  draftChildName: string;
  draftChildSummary: string;
  draftEditMessage: string | null;
  draftEditError: string | null;
  selectedDraftNodePrompts: string[];
  selectedNodeRecentActions: PlannerAction[];
  latestDraftPlan: PlannerPlan | null;
  latestTraceEvents: PlannerTraceEvent[];
  onEditableVoiceTranscriptChange: (value: string) => void;
  onSubmitPendingVoiceTranscript: () => void;
  onRetryVoiceCapture: () => void;
  onClearPendingVoiceReview: () => void;
  onExportDesignReviewPacket: () => void;
  onConfirmPendingPlan: () => void;
  onDismissPendingPlan: () => void;
  onSelectDraftNode: (nodeId: string | null) => void;
  onToggleDraftNodeExpanded: (nodeId: string) => void;
  onExpandAllDraftNodes: () => void;
  onCollapseAllDraftNodes: () => void;
  onRenameDraftNameChange: (name: string) => void;
  onRenameSelectedDraftNode: () => void;
  onDeleteSelectedDraftNode: () => void;
  onDraftChildTypeChange: (childType: PlannerDraftChildType) => void;
  onDraftChildNameChange: (name: string) => void;
  onDraftChildSummaryChange: (summary: string) => void;
  onAddChildToSelectedDraftNode: () => void;
  onApplyPromptSuggestion: (prompt: string) => void;
  onRevealDesignPacket: (path: string) => void;
  onBackToChat: () => void;
};

export function PlannerPageContent({
  plannerView,
  plannerComposer,
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
  selectedDraftNode,
  draftValidation,
  selectedDraftNodeId,
  expandedDraftNodeIds,
  selectedDraftNodePath,
  renameDraftName,
  allowedDraftChildTypes,
  draftChildType,
  draftChildName,
  draftChildSummary,
  draftEditMessage,
  draftEditError,
  selectedDraftNodePrompts,
  selectedNodeRecentActions,
  latestDraftPlan,
  latestTraceEvents,
  onEditableVoiceTranscriptChange,
  onSubmitPendingVoiceTranscript,
  onRetryVoiceCapture,
  onClearPendingVoiceReview,
  onExportDesignReviewPacket,
  onConfirmPendingPlan,
  onDismissPendingPlan,
  onSelectDraftNode,
  onToggleDraftNodeExpanded,
  onExpandAllDraftNodes,
  onCollapseAllDraftNodes,
  onRenameDraftNameChange,
  onRenameSelectedDraftNode,
  onDeleteSelectedDraftNode,
  onDraftChildTypeChange,
  onDraftChildNameChange,
  onDraftChildSummaryChange,
  onAddChildToSelectedDraftNode,
  onApplyPromptSuggestion,
  onRevealDesignPacket,
  onBackToChat,
}: PlannerPageContentProps) {
  if (plannerView === "draft") {
    return (
      <div style={styles.draftWorkspace}>
        <div style={styles.draftWorkspaceMain}>
          <PlannerDraftCanvas
            selectedDraftNode={selectedDraftNode}
            draftTreeNodes={draftTreeNodes}
            draftValidation={draftValidation}
            selectedDraftNodeId={selectedDraftNodeId}
            expandedDraftNodeIds={expandedDraftNodeIds}
            onSelectDraftNode={onSelectDraftNode}
            onToggleDraftNode={onToggleDraftNodeExpanded}
            onExpandAllDraftNodes={onExpandAllDraftNodes}
            onCollapseAllDraftNodes={onCollapseAllDraftNodes}
          />
          {plannerComposer}
        </div>

        <PlannerDraftSidePanel
          selectedDraftNode={selectedDraftNode}
          selectedDraftNodePath={selectedDraftNodePath}
          renameDraftName={renameDraftName}
          onRenameDraftNameChange={onRenameDraftNameChange}
          onRenameSelectedDraftNode={onRenameSelectedDraftNode}
          onDeleteSelectedDraftNode={onDeleteSelectedDraftNode}
          isPlannerBusy={isPlannerBusy}
          allowedDraftChildTypes={allowedDraftChildTypes}
          draftChildType={draftChildType}
          onDraftChildTypeChange={onDraftChildTypeChange}
          draftChildName={draftChildName}
          onDraftChildNameChange={onDraftChildNameChange}
          draftChildSummary={draftChildSummary}
          onDraftChildSummaryChange={onDraftChildSummaryChange}
          onAddChildToSelectedDraftNode={onAddChildToSelectedDraftNode}
          draftEditMessage={draftEditMessage}
          draftEditError={draftEditError}
          selectedDraftNodePrompts={selectedDraftNodePrompts}
          onApplyPromptSuggestion={onApplyPromptSuggestion}
          selectedNodeRecentActions={selectedNodeRecentActions}
          draftValidation={draftValidation}
          isExportingDesignPacket={isExportingDesignPacket}
          onExportDesignReviewPacket={onExportDesignReviewPacket}
          designPacketPath={designPacketPath}
          onRevealDesignPacket={onRevealDesignPacket}
          designPacketError={designPacketError}
          onConfirmPendingPlan={onConfirmPendingPlan}
          draftTreeNodeCount={draftTreeNodes.length}
          onBackToChat={onBackToChat}
          onDismissPendingPlan={onDismissPendingPlan}
          latestDraftPlan={latestDraftPlan}
        />
      </div>
    );
  }

  if (plannerView === "trace") {
    return <PlannerTraceView events={latestTraceEvents} />;
  }

  return (
    <>
      <PlannerConversationTranscript
        transcriptRef={transcriptRef}
        pendingVoiceTranscript={pendingVoiceTranscript}
        reviewVoiceBeforeSend={reviewVoiceBeforeSend}
        voiceElapsedMs={voiceElapsedMs}
        isVoiceSubmitting={isVoiceSubmitting}
        editableVoiceTranscript={editableVoiceTranscript}
        isPlannerBusy={isPlannerBusy}
        messages={messages}
        isExportingDesignPacket={isExportingDesignPacket}
        pendingPlan={pendingPlan}
        draftTreeNodes={draftTreeNodes}
        designPacketPath={designPacketPath}
        designPacketError={designPacketError}
        onEditableVoiceTranscriptChange={onEditableVoiceTranscriptChange}
        onSubmitPendingVoiceTranscript={onSubmitPendingVoiceTranscript}
        onRetryVoiceCapture={onRetryVoiceCapture}
        onClearPendingVoiceReview={onClearPendingVoiceReview}
        onExportDesignReviewPacket={onExportDesignReviewPacket}
        onConfirmPendingPlan={onConfirmPendingPlan}
        onDismissPendingPlan={onDismissPendingPlan}
      />
      {plannerComposer}
    </>
  );
}

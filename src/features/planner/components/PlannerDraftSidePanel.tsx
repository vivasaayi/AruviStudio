import type { PlannerDraftChildType } from "../../../lib/types";
import {
  type DraftValidationSummary,
  type PlannerAction,
  type PlannerPlan,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";
import { PlannerDraftActionList } from "./PlannerDraftActionList";
import { PlannerDraftNodeCard } from "./PlannerDraftNodeCard";

type PlannerDraftSidePanelProps = {
  selectedDraftNode: PlannerTreeNode | null;
  selectedDraftNodePath: PlannerTreeNode[];
  renameDraftName: string;
  onRenameDraftNameChange: (name: string) => void;
  onRenameSelectedDraftNode: () => void;
  onDeleteSelectedDraftNode: () => void;
  isPlannerBusy: boolean;
  allowedDraftChildTypes: PlannerDraftChildType[];
  draftChildType: PlannerDraftChildType;
  onDraftChildTypeChange: (childType: PlannerDraftChildType) => void;
  draftChildName: string;
  onDraftChildNameChange: (name: string) => void;
  draftChildSummary: string;
  onDraftChildSummaryChange: (summary: string) => void;
  onAddChildToSelectedDraftNode: () => void;
  draftEditMessage: string | null;
  draftEditError: string | null;
  selectedDraftNodePrompts: string[];
  onApplyPromptSuggestion: (prompt: string) => void;
  selectedNodeRecentActions: PlannerAction[];
  draftValidation: DraftValidationSummary;
  isExportingDesignPacket: boolean;
  onExportDesignReviewPacket: () => void;
  designPacketPath: string | null;
  onRevealDesignPacket: (path: string) => void;
  designPacketError: string | null;
  onConfirmPendingPlan: () => void;
  draftTreeNodeCount: number;
  onBackToChat: () => void;
  onDismissPendingPlan: () => void;
  latestDraftPlan: PlannerPlan | null;
};

export function PlannerDraftSidePanel({
  selectedDraftNode,
  selectedDraftNodePath,
  renameDraftName,
  onRenameDraftNameChange,
  onRenameSelectedDraftNode,
  onDeleteSelectedDraftNode,
  isPlannerBusy,
  allowedDraftChildTypes,
  draftChildType,
  onDraftChildTypeChange,
  draftChildName,
  onDraftChildNameChange,
  draftChildSummary,
  onDraftChildSummaryChange,
  onAddChildToSelectedDraftNode,
  draftEditMessage,
  draftEditError,
  selectedDraftNodePrompts,
  onApplyPromptSuggestion,
  selectedNodeRecentActions,
  draftValidation,
  isExportingDesignPacket,
  onExportDesignReviewPacket,
  designPacketPath,
  onRevealDesignPacket,
  designPacketError,
  onConfirmPendingPlan,
  draftTreeNodeCount,
  onBackToChat,
  onDismissPendingPlan,
  latestDraftPlan,
}: PlannerDraftSidePanelProps) {
  return (
    <div style={styles.draftWorkspaceSide}>
      <PlannerDraftNodeCard
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
      />

      <div style={styles.sideCard}>
        <div style={styles.label}>Design Validation</div>
        <div style={styles.helper}>
          Structural checks for the staged tree before you apply it into the real catalog.
        </div>
        <div style={styles.issueList}>
          {draftValidation.issues.slice(0, 6).map((issue, index) => {
            const issueStyle = issue.tone === "ok"
              ? styles.issueCardOk
              : styles.issueCardWarn;
            return (
              <div key={`${issue.title}-${index}`} style={issueStyle}>
                <div style={styles.issueTitle}>{issue.title}</div>
                <div style={styles.issueDetail}>{issue.detail}</div>
              </div>
            );
          })}
        </div>
      </div>

      <div style={styles.sideCard}>
        <div style={styles.label}>Design Review Packet</div>
        <div style={styles.helper}>
          Generate a reviewable HTML packet with architecture diagrams, feature changes, risks, work breakdown, and the approval-ready change set before applying anything.
        </div>
        <div style={styles.inlineButtonRow}>
          <button style={styles.btnGhost} onClick={onExportDesignReviewPacket} disabled={isExportingDesignPacket}>
            {isExportingDesignPacket ? "Generating..." : "Generate Packet"}
          </button>
          {designPacketPath ? (
            <button style={styles.btnGhost} onClick={() => onRevealDesignPacket(designPacketPath)}>
              Reveal Packet
            </button>
          ) : null}
          <button data-testid="draft-commit" style={styles.btn} onClick={onConfirmPendingPlan} disabled={draftTreeNodeCount === 0 || isPlannerBusy}>
            Apply Design
          </button>
          <button style={styles.btnGhost} onClick={onBackToChat}>
            Back to Chat
          </button>
          <button style={styles.btnDanger} onClick={onDismissPendingPlan} disabled={draftTreeNodeCount === 0}>
            Clear Design
          </button>
        </div>
        {designPacketPath ? <div style={{ ...styles.success, marginTop: 10 }}>Packet exported to {designPacketPath}</div> : null}
        {designPacketError ? <div style={{ ...styles.error, marginTop: 10 }}>{designPacketError}</div> : null}
      </div>

      <div style={styles.sideCard}>
        <div style={styles.label}>Latest Design Ops</div>
        {latestDraftPlan ? (
          <>
            <div style={styles.helper}>{latestDraftPlan.assistant_response}</div>
            <PlannerDraftActionList actions={latestDraftPlan.actions.slice(0, 8)} />
          </>
        ) : (
          <div style={styles.helper}>
            No pending proposal snapshot. Use the chat to add structure, then review and keep refining the staged tree here.
          </div>
        )}
      </div>
    </div>
  );
}

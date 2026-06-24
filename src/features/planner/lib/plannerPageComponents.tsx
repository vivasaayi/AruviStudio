import React from "react";

import { styles } from "./plannerPageStyles";
import { getPlannerNodeType } from "./plannerDraftTree";
import type { PendingPlan, PlannerTreeNode } from "./plannerPageTypes";

export function TreeNodeView({ node }: { node: PlannerTreeNode }) {
  if (node.children.length === 0) {
    return (
      <div style={styles.treeLeaf}>
        {node.label}
        {node.meta ? <span style={styles.treeMeta}>{node.meta}</span> : null}
      </div>
    );
  }
  return (
    <details open style={styles.treeNode}>
      <summary style={styles.treeSummary}>
        {node.label}
        {node.meta ? <span style={styles.treeMeta}>{node.meta}</span> : null}
      </summary>
      <div style={styles.treeChildren}>
        {node.children.map((child) => (
          <TreeNodeView key={child.id} node={child} />
        ))}
      </div>
    </details>
  );
}

export function FormattedPlannerText({ content }: { content: string }) {
  const lines = content.split("\n");

  return (
    <div style={styles.messageText}>
      {lines.map((line, lineIndex) => (
        <React.Fragment key={`${line}-${lineIndex}`}>
          {renderInlinePlannerMarkdown(line)}
          {lineIndex < lines.length - 1 ? <br /> : null}
        </React.Fragment>
      ))}
    </div>
  );
}

export function renderInlinePlannerMarkdown(line: string) {
  return line.split(/(\*\*[^*]+\*\*)/g).map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      return <strong key={`${part}-${index}`}>{part.slice(2, -2)}</strong>;
    }
    return <React.Fragment key={`${part}-${index}`}>{part}</React.Fragment>;
  });
}

export function PlannerComposer({
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
  scopeHint,
  isProductSelected,
}: {
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
  scopeHint: string;
  isProductSelected: boolean;
}) {
  return (
    <div style={styles.composerWrap}>
      <textarea
        ref={composerRef}
        data-testid="planner-input"
        style={styles.textarea}
        value={draft}
        onChange={(event) => onDraftChange(event.target.value)}
        placeholder={isProductSelected
          ? "Describe what to design inside the selected product. Example: Add reporting capabilities, features, and starter stories."
          : "Select a product before planning. Create products in the Products page."}
      />
      {scopeChips.length > 0 ? (
        <div style={styles.composerScopeCard}>
          <div style={styles.composerScopeTitle}>Current Scope</div>
          <div style={{ ...styles.chipRow, marginTop: 0 }}>
            {scopeChips.map((chip) => (
              <div key={chip} style={styles.chip}>
                {chip}
              </div>
            ))}
          </div>
          <div style={{ ...styles.helper, marginTop: 8 }}>{scopeHint}</div>
        </div>
      ) : null}
      {draftTreeNodesLength > 0 ? (
        <div style={styles.inlineButtonRow}>
          <button style={styles.btnGhost} onClick={onOpenDraftWorkspace}>
            Open Review
          </button>
          <button style={styles.btnGhost} onClick={onConfirm} disabled={isPlannerBusy}>
            Apply Design
          </button>
          <button style={styles.btnDanger} onClick={onDismiss} disabled={isPlannerBusy}>
            Clear Design
          </button>
        </div>
      ) : null}
      <div style={styles.actionRow}>
        <button data-testid="planner-send" style={styles.btn} onClick={onSend} disabled={isPlannerBusy || !isProductSelected}>
          {isPlannerBusy ? "Working..." : isProductSelected ? "Send" : "Select Product"}
        </button>
        <button style={styles.btnGhost} onClick={onToggleListening} disabled={!isProductSelected || !voiceEnabled || isTranscribing || isVoiceSubmitting || Boolean(pendingVoiceTranscript)}>
          {isListening
            ? "Stop Recording"
            : isTranscribing
              ? "Transcribing..."
              : isVoiceSubmitting
                ? "Sending Voice..."
                : "Start Voice Input"}
        </button>
        {draftTreeNodesLength === 0 ? (
          <>
            <button style={styles.btnGhost} onClick={onConfirm} disabled={!pendingPlan}>
              Apply Proposal
            </button>
            <button style={styles.btnDanger} onClick={onDismiss} disabled={!pendingPlan}>
              Clear Pending
            </button>
          </>
        ) : null}
        <span style={styles.status}>
          {voiceActivity
            ? voiceActivity
            : pendingVoiceTranscript
              ? "Voice transcript is ready. Review, edit, send, retry, or cancel."
              : draftTreeNodesLength > 0
                ? "A staged design is active. Review it, export the packet, then apply when ready."
                : pendingPlan
                  ? "A proposed plan is waiting for confirmation."
                  : isProductSelected
                    ? "No pending proposal."
                    : "Select a product to begin planning."}
        </span>
      </div>
    </div>
  );
}

export function SelectableTreeNodeView({
  node,
  selectedNodeId,
  onSelect,
  expandedNodeIds,
  onToggle,
}: {
  node: PlannerTreeNode;
  selectedNodeId: string | null;
  onSelect: (nodeId: string) => void;
  expandedNodeIds: Set<string>;
  onToggle: (nodeId: string) => void;
}) {
  const isSelected = node.id === selectedNodeId;
  const hasChildren = node.children.length > 0;
  const isExpanded = expandedNodeIds.has(node.id);
  const nodeType = getPlannerNodeType(node);
  const cardStyle = isSelected ? { ...styles.treeCard, ...styles.treeCardSelected } : styles.treeCard;
  return (
    <div style={styles.treeLevel}>
      <div style={styles.treeRow}>
        {hasChildren ? (
          <button type="button" style={styles.treeToggle} onClick={() => onToggle(node.id)} data-testid={`draft-node-toggle-${node.id}`}>
            {isExpanded ? "▾" : "▸"}
          </button>
        ) : (
          <div style={styles.treeToggleGhost}>•</div>
        )}
        <button type="button" style={cardStyle} onClick={() => onSelect(node.id)} data-testid={`draft-node-${node.id}`}>
          <div style={styles.treeCardHeader}>
            <div style={styles.treeCardTitle}>{node.label}</div>
            <div style={styles.treeCardMetaRow}>
              <span style={styles.treeTypeBadge}>{nodeType}</span>
              {hasChildren ? <span style={styles.treeCountBadge}>{node.children.length} children</span> : null}
              {node.confidence ? <span style={styles.treeCountBadge}>{node.confidence} confidence</span> : null}
            </div>
          </div>
          {node.summary ? <div style={styles.diffSecondary}>{node.summary}</div> : null}
          {node.meta ? <div style={styles.diffSecondary}>{node.meta}</div> : null}
        </button>
      </div>
      {hasChildren && isExpanded ? (
        <div style={styles.treeRowChildren}>
          {node.children.map((child) => (
            <SelectableTreeNodeView
              key={child.id}
              node={child}
              selectedNodeId={selectedNodeId}
              onSelect={onSelect}
              expandedNodeIds={expandedNodeIds}
              onToggle={onToggle}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

import {
  buildProposalTreeNodes,
  FormattedPlannerText,
  TreeNodeView,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
  summarizeAction,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";

type PlannerAssistantMessageProps = {
  message: PlannerMessage;
  isExportingDesignPacket: boolean;
  onExportDesignReviewPacket: () => void;
  onConfirmPendingPlan: () => void;
  onDismissPendingPlan: () => void;
  isPlannerBusy: boolean;
  pendingPlan: PendingPlan | null;
  draftTreeNodes: PlannerTreeNode[];
  designPacketPath: string | null;
  designPacketError: string | null;
};

export function PlannerAssistantMessage({
  message,
  isExportingDesignPacket,
  onExportDesignReviewPacket,
  onConfirmPendingPlan,
  onDismissPendingPlan,
  isPlannerBusy,
  pendingPlan,
  draftTreeNodes,
  designPacketPath,
  designPacketError,
}: PlannerAssistantMessageProps) {
  if (message.kind === "proposal" && message.plan) {
    const proposalTreeNodes = buildProposalTreeNodes(message.plan);
    return (
      <>
        <FormattedPlannerText content={message.content} />
        <div style={styles.card}>
          <div style={styles.cardTitle}>Proposed Changes</div>
          {message.plan.actions.map((action, index) => {
            const summary = summarizeAction(action);
            const symbolStyle = summary.tone === "add"
              ? styles.diffSymbolAdd
              : summary.tone === "update"
                ? styles.diffSymbolUpdate
                : styles.diffSymbolWarn;
            return (
              <div key={`${action.type}-${index}`} style={styles.diffRow}>
                <div style={symbolStyle}>{summary.symbol}</div>
                <div>
                  <div style={styles.diffPrimary}>{summary.title}</div>
                  {summary.detail ? <div style={styles.diffSecondary}>{summary.detail}</div> : null}
                </div>
              </div>
            );
          })}
          {proposalTreeNodes.length > 0 ? (
            <div style={styles.cardSection}>
              <div style={styles.cardTitle}>Proposed Design</div>
              <div style={styles.treePanel}>
                {proposalTreeNodes.map((node) => (
                  <TreeNodeView key={node.id} node={node} />
                ))}
              </div>
            </div>
          ) : null}
          {message.treeNodes && message.treeNodes.length > 0 ? (
            <div style={styles.cardSection}>
              <div style={styles.cardTitle}>Current Design</div>
              <div style={styles.treePanel}>
                {message.treeNodes.map((node) => (
                  <TreeNodeView key={node.id} node={node} />
                ))}
              </div>
            </div>
          ) : null}
          <div style={styles.inlineButtonRow}>
            <button style={styles.btnGhost} onClick={onExportDesignReviewPacket} disabled={isExportingDesignPacket}>
              {isExportingDesignPacket ? "Generating Packet..." : "Generate Review Packet"}
            </button>
            <button style={styles.btn} onClick={onConfirmPendingPlan} disabled={isPlannerBusy || (!pendingPlan && draftTreeNodes.length === 0)}>
              {draftTreeNodes.length > 0 ? "Apply Design" : "Confirm Proposal"}
            </button>
            <button style={styles.btnGhost} onClick={onDismissPendingPlan} disabled={!pendingPlan && draftTreeNodes.length === 0}>
              {draftTreeNodes.length > 0 ? "Clear Design" : "Dismiss"}
            </button>
          </div>
          {designPacketPath ? (
            <div style={{ ...styles.helper, marginTop: 8 }}>
              Packet exported: {designPacketPath}
            </div>
          ) : null}
          {designPacketError ? <div style={styles.error}>{designPacketError}</div> : null}
        </div>
      </>
    );
  }

  if (message.kind === "tree" && message.treeNodes) {
    return (
      <>
        <FormattedPlannerText content={message.content} />
        <div style={styles.treePanel}>
          {message.treeNodes.map((node) => (
            <TreeNodeView key={node.id} node={node} />
          ))}
        </div>
      </>
    );
  }

  return <FormattedPlannerText content={message.content} />;
}

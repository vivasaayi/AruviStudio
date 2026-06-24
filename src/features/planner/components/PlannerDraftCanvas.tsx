import {
  SelectableTreeNodeView,
  type DraftValidationSummary,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";

type Props = {
  selectedDraftNode: PlannerTreeNode | null;
  draftTreeNodes: PlannerTreeNode[];
  draftValidation: DraftValidationSummary;
  selectedDraftNodeId: string | null;
  expandedDraftNodeIds: Set<string>;
  onSelectDraftNode: (nodeId: string) => void;
  onToggleDraftNode: (nodeId: string) => void;
  onExpandAllDraftNodes: () => void;
  onCollapseAllDraftNodes: () => void;
};

export function PlannerDraftCanvas({
  selectedDraftNode,
  draftTreeNodes,
  draftValidation,
  selectedDraftNodeId,
  expandedDraftNodeIds,
  onSelectDraftNode,
  onToggleDraftNode,
  onExpandAllDraftNodes,
  onCollapseAllDraftNodes,
}: Props) {
  const warningCount = draftValidation.issues.filter((issue) => issue.tone === "warn").length;

  return (
    <div style={styles.draftCanvas}>
      <div style={styles.draftCanvasHeader}>
        <div>
          <div style={styles.draftCanvasTitle}>Staged Design Tree</div>
          <div style={styles.helper}>
            Select a node, then refine it in natural language. The composer below will use the selected design node as planning context.
          </div>
        </div>
        <div style={styles.chipRow}>
          {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
          <div style={styles.chip}>{draftTreeNodes.length} root {draftTreeNodes.length === 1 ? "node" : "nodes"}</div>
        </div>
      </div>
      <div style={styles.readinessBanner}>
        <div>
          <div style={styles.label}>Apply Readiness</div>
          <div style={styles.readinessMeta}>
            {warningCount === 0
              ? "This design is structurally solid enough to apply."
              : "There are still weak spots in the staged tree. Fix them before applying if you want a cleaner catalog."}
          </div>
        </div>
        <div style={styles.readinessScore}>{draftValidation.score}</div>
      </div>
      <div style={styles.metricGrid}>
        <div style={styles.metricCard}>
          <div style={styles.metricLabel}>Products</div>
          <div style={styles.metricValue}>{draftValidation.counts.product}</div>
        </div>
        <div style={styles.metricCard}>
          <div style={styles.metricLabel}>Product Areas</div>
          <div style={styles.metricValue}>{draftValidation.counts["product area"]}</div>
        </div>
        <div style={styles.metricCard}>
          <div style={styles.metricLabel}>Capabilities</div>
          <div style={styles.metricValue}>{draftValidation.counts.capability}</div>
        </div>
        <div style={styles.metricCard}>
          <div style={styles.metricLabel}>Delivery Items</div>
          <div style={styles.metricValue}>{draftValidation.counts["work item"]}</div>
        </div>
      </div>
      <div style={styles.treeToolbar}>
        <button data-testid="draft-expand-all" style={styles.btnGhost} onClick={onExpandAllDraftNodes} disabled={draftTreeNodes.length === 0}>
          Expand All
        </button>
        <button data-testid="draft-collapse-all" style={styles.btnGhost} onClick={onCollapseAllDraftNodes} disabled={draftTreeNodes.length === 0}>
          Collapse All
        </button>
        <div style={styles.treeToolbarSpacer} />
        <div style={styles.helper}>
          Select a node to scope prompts. Expand branches to inspect the staged structure.
        </div>
      </div>
      {draftTreeNodes.length > 0 ? (
        <div style={styles.treeExplorer}>
          {draftTreeNodes.map((node) => (
            <SelectableTreeNodeView
              key={node.id}
              node={node}
              selectedNodeId={selectedDraftNodeId}
              onSelect={onSelectDraftNode}
              expandedNodeIds={expandedDraftNodeIds}
              onToggle={onToggleDraftNode}
            />
          ))}
        </div>
      ) : (
        <div style={styles.emptyState}>
          No staged design yet. Ask the planner to design product areas, capabilities, features, stories, or tasks for the selected product, then switch back here to inspect and refine it.
        </div>
      )}
    </div>
  );
}

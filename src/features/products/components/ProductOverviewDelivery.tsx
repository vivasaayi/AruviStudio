import type { Product, WorkItem } from "../../../lib/types";
import {
  getWorkItemPresentation,
  type WorkItemMetrics,
  type WorkItemNode,
} from "../lib/productOverview";
import type { ProductOverviewPlannerAction } from "./ProductOverviewDocument";
import { styles } from "./ProductOverviewDocument.styles";

type LightWorkItemTone = {
  accentColor: string;
  borderColor: string;
  backgroundColor: string;
  badgeBackground: string;
  badgeColor: string;
};

function getLightWorkItemTone(bucket: ReturnType<typeof getWorkItemPresentation>["bucket"]): LightWorkItemTone {
  if (bucket === "done") {
    return {
      accentColor: "#16a34a",
      borderColor: "#bbf7d0",
      backgroundColor: "#f0fdf4",
      badgeBackground: "#dcfce7",
      badgeColor: "#166534",
    };
  }

  if (bucket === "wip") {
    return {
      accentColor: "#ca8a04",
      borderColor: "#fde68a",
      backgroundColor: "#fffbeb",
      badgeBackground: "#fef3c7",
      badgeColor: "#854d0e",
    };
  }

  if (bucket === "blocked") {
    return {
      accentColor: "#dc2626",
      borderColor: "#fecaca",
      backgroundColor: "#fff1f2",
      badgeBackground: "#fee2e2",
      badgeColor: "#991b1b",
    };
  }

  return {
    accentColor: "#2563eb",
    borderColor: "#bfdbfe",
    backgroundColor: "#eff6ff",
    badgeBackground: "#dbeafe",
    badgeColor: "#1e40af",
  };
}

export function WorkItemTree({
  product,
  nodes,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  nodes: WorkItemNode[];
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  return (
    <div style={styles.workItemList}>
      {nodes.map((node) => (
        <WorkItemCard key={node.workItem.id} product={product} node={node} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
      ))}
    </div>
  );
}

function WorkItemCard({
  product,
  node,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  node: WorkItemNode;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  const presentation = getWorkItemPresentation(node.workItem.status);
  const tone = getLightWorkItemTone(presentation.bucket);
  const noteText = node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria;
  const excerpt = noteText ? summarizeText(noteText) : "";

  return (
    <div
      style={{
        ...styles.workItemCard,
        borderColor: tone.borderColor,
        backgroundColor: tone.backgroundColor,
        borderLeft: `4px solid ${tone.accentColor}`,
      }}
      onClick={() => onOpenWorkItem(node.workItem)}
    >
      <div style={styles.workItemHeader}>
        <div style={{ minWidth: 0 }}>
          <h5 style={styles.workItemTitle}>{node.workItem.title}</h5>
          <div style={{ ...styles.metaRow, marginTop: 6 }}>
            <span
              style={{
                ...styles.statePill,
                backgroundColor: tone.badgeBackground,
                color: tone.badgeColor,
              }}
            >
              {presentation.label}
            </span>
            <span style={styles.metaPill}>{node.workItem.work_item_type.replace(/_/g, " ")}</span>
            <span style={styles.metaPill}>{node.workItem.priority} priority</span>
            <span style={styles.metaPill}>{node.workItem.complexity.replace(/_/g, " ")}</span>
            {node.children.length > 0 ? <span style={styles.metaPill}>{node.children.length} sub-item{node.children.length === 1 ? "" : "s"}</span> : null}
          </div>
        </div>
        <button
          style={styles.subtleButton}
          onClick={(event) => {
            event.stopPropagation();
            onPlanFromItem({ kind: "enhance_work_item", product, workItem: node.workItem });
          }}
        >
          Improve
        </button>
      </div>
      {excerpt ? <div style={styles.workItemText}>{excerpt}</div> : null}
      {node.children.length > 0 ? (
        <div style={styles.workItemChildren}>
          <WorkItemTree product={product} nodes={node.children} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
        </div>
      ) : null}
    </div>
  );
}

export function MetricPills({ metrics }: { metrics: WorkItemMetrics }) {
  if (metrics.total === 0) {
    return <span style={styles.summaryPill}>No stories</span>;
  }

  return (
    <>
      {metrics.done > 0 ? <StatusTonePill label={`${metrics.done} done`} tone="#15803d" /> : null}
      {metrics.wip > 0 ? <StatusTonePill label={`${metrics.wip} WIP`} tone="#a16207" /> : null}
      {metrics.tbd > 0 ? <StatusTonePill label={`${metrics.tbd} TBD`} tone="#1d4ed8" /> : null}
      {metrics.blocked > 0 ? <StatusTonePill label={`${metrics.blocked} blocked`} tone="#b91c1c" /> : null}
    </>
  );
}

function StatusTonePill({ label, tone }: { label: string; tone: string }) {
  return (
    <span style={{ ...styles.statePill, color: tone }}>
      {label}
    </span>
  );
}

function summarizeText(text: string, maxLength = 220) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 1).trimEnd()}…`;
}

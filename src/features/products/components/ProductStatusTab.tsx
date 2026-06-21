import type React from "react";

import type { Product } from "../../../lib/types";
import type { ProductStatusSummary, StatusRow } from "../lib/productStatusSummary";

export type ProductStatusGroupBy = "node" | "kind" | "work_status";

type ProductStatusTabStyles = {
  statusToolbar: React.CSSProperties;
  controlLabel: React.CSSProperties;
  select: React.CSSProperties;
  statusMetrics: React.CSSProperties;
  statusMetric: React.CSSProperties;
  metricLabel: React.CSSProperties;
  statusMetricValue: React.CSSProperties;
  statusMetricHelp: React.CSSProperties;
  table: React.CSSProperties;
  statusTableHeader: React.CSSProperties;
  statusTableRow: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  rowSecondary: React.CSSProperties;
  rowCell: React.CSSProperties;
  progressTrack: React.CSSProperties;
  progressFill: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductStatusTab({
  products,
  statusProductId,
  statusDepth,
  statusGroupBy,
  statusSummary,
  statusRows,
  isLoading,
  onStatusProductChange,
  onStatusDepthChange,
  onStatusGroupByChange,
  onOpenStatusRow,
  styles,
}: {
  products: Product[];
  statusProductId: string;
  statusDepth: number;
  statusGroupBy: ProductStatusGroupBy;
  statusSummary: ProductStatusSummary;
  statusRows: StatusRow[];
  isLoading: boolean;
  onStatusProductChange: (productId: string) => void;
  onStatusDepthChange: (depth: number) => void;
  onStatusGroupByChange: (groupBy: ProductStatusGroupBy) => void;
  onOpenStatusRow: (row: StatusRow) => void;
  styles: ProductStatusTabStyles;
}) {
  return (
    <>
      <div style={styles.statusToolbar}>
        <div>
          <div style={styles.controlLabel}>Product</div>
          <select
            style={styles.select}
            value={statusProductId}
            onChange={(event) => onStatusProductChange(event.target.value)}
          >
            <option value="all">All visible products</option>
            {products.map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
          </select>
        </div>
        <div>
          <div style={styles.controlLabel}>Visible levels</div>
          <select style={styles.select} value={statusDepth} onChange={(event) => onStatusDepthChange(Number(event.target.value))}>
            {[1, 2, 3, 4, 5, 6].map((depth) => <option key={depth} value={depth}>{depth} {depth === 1 ? "level" : "levels"}</option>)}
          </select>
        </div>
        <div>
          <div style={styles.controlLabel}>Pivot</div>
          <select style={styles.select} value={statusGroupBy} onChange={(event) => onStatusGroupByChange(event.target.value as ProductStatusGroupBy)}>
            <option value="node">Tree nodes</option>
            <option value="kind">Node kind</option>
            <option value="work_status">Work status</option>
          </select>
        </div>
        <div style={styles.statusMetrics}>
          <div style={styles.statusMetric}>
            <div style={styles.metricLabel}>Products</div>
            <div style={styles.statusMetricValue}>{statusSummary.productCount}</div>
            <div style={styles.statusMetricHelp}>included</div>
          </div>
          <div style={styles.statusMetric}>
            <div style={styles.metricLabel}>Nodes</div>
            <div style={styles.statusMetricValue}>{statusSummary.nodeCount}</div>
            <div style={styles.statusMetricHelp}>{statusSummary.leafCount} leaf</div>
          </div>
          <div style={styles.statusMetric}>
            <div style={styles.metricLabel}>Stories</div>
            <div style={styles.statusMetricValue}>{statusSummary.workItemCount}</div>
            <div style={styles.statusMetricHelp}>{statusSummary.activeWorkItemCount} active · {statusSummary.doneWorkItemCount} done stories</div>
          </div>
          <div style={styles.statusMetric}>
            <div style={styles.metricLabel}>Progress</div>
            <div style={styles.statusMetricValue}>{statusSummary.progress.percent}%</div>
            <div style={styles.statusMetricHelp}>{statusSummary.progress.done}/{statusSummary.progress.total}</div>
          </div>
        </div>
      </div>
      <div style={styles.table}>
        <div style={styles.statusTableHeader}>
          <div>{statusGroupBy === "node" ? "Scope" : "Group"}</div>
          <div>Level</div>
          <div>Kind</div>
          <div>Nodes</div>
          <div>Work</div>
          <div>Progress</div>
        </div>
        {statusRows.length > 0 ? statusRows.map((row) => (
          <div
            key={row.id}
            style={styles.statusTableRow}
            onClick={() => onOpenStatusRow(row)}
          >
            <div style={{ paddingLeft: statusGroupBy === "node" ? Math.max(0, row.level - 1) * 16 : 0 }}>
              <div style={{ ...styles.rowPrimary, fontSize: 12 }}>{row.name}</div>
              <div style={{ ...styles.rowSecondary, fontSize: 10, marginTop: 2 }}>{row.subtitle}</div>
            </div>
            <div style={styles.rowCell}>{row.level}</div>
            <div style={styles.rowCell}>{row.kind}</div>
            <div style={styles.rowCell}>{row.nodeCount} · {row.childCount} child</div>
            <div style={styles.rowCell}>{row.workItemCount} · {row.activeWorkItemCount} active stories</div>
            <div>
              <div style={{ display: "grid", gridTemplateColumns: "38px minmax(0, 1fr) 46px", gap: 6, alignItems: "center" }}>
                <span style={styles.rowCell}>{row.progress.percent}%</span>
                <div style={styles.progressTrack}><div style={{ ...styles.progressFill, width: `${row.progress.percent}%` }} /></div>
                <span style={{ ...styles.rowSecondary, marginTop: 0 }}>{row.progress.done}/{row.progress.total}</span>
              </div>
            </div>
          </div>
        )) : (
          <div style={styles.empty}>{isLoading ? "Loading status..." : "No status rows are available for this selection."}</div>
        )}
      </div>
    </>
  );
}

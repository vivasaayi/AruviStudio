import type { WorkflowRun } from "../../../lib/types";
import {
  WORKFLOW_DAG_LANES,
  WORKFLOW_DAG_LINKS,
  WORKFLOW_DAG_NODES,
  type WorkflowDagNode,
} from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

export type WorkflowLaneStatus = {
  done: number;
  active: number;
  pending: number;
  failed: number;
};

type WorkItemWorkflowDagProps = {
  activeWorkflowStage: string | null;
  completedStages: Set<string>;
  dagNodeById: Map<string, WorkflowDagNode>;
  laneStatusById: Map<string, WorkflowLaneStatus>;
  selectedDagNode: WorkflowDagNode;
  selectedDagNodeId: string;
  latestWorkflowRun: WorkflowRun | null | undefined;
  onSelectArtifactStage: (stageId: string) => void;
};

export function WorkItemWorkflowDag({
  activeWorkflowStage,
  completedStages,
  dagNodeById,
  laneStatusById,
  selectedDagNode,
  selectedDagNodeId,
  latestWorkflowRun: _latestWorkflowRun,
  onSelectArtifactStage,
}: WorkItemWorkflowDagProps) {
  return (
    <div style={{ marginTop: 12 }}>
      <div style={styles.detailLabel}>Stage Artifacts</div>
      <div style={styles.dagLegend}>
        <span style={styles.dagLegendItem}>
          <span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#4ec9b0", display: "inline-block" }} /> done
        </span>
        <span style={styles.dagLegendItem}>
          <span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#0e639c", display: "inline-block" }} /> active
        </span>
        <span style={styles.dagLegendItem}>
          <span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#3f4a59", display: "inline-block" }} /> pending
        </span>
      </div>
      <div style={styles.dagWrap}>
        <svg width={2520} height={260} viewBox="0 0 2520 260" role="img" aria-label="Workflow DAG">
          {WORKFLOW_DAG_LANES.map((lane) => (
            <g key={lane.id}>
              <rect
                x={lane.x}
                y={lane.y}
                width={lane.width}
                height={lane.height}
                rx={12}
                fill="#131821"
                stroke="#273140"
                strokeWidth={1}
              />
              <text x={lane.x + 14} y={lane.y + 22} fill="#8f96a3" fontSize={11} fontWeight={700} letterSpacing={0.8}>
                {lane.label}
              </text>
              {(() => {
                const summary = laneStatusById.get(lane.id);
                if (!summary) return null;
                const parts = [
                  `done ${summary.done}`,
                  `active ${summary.active}`,
                  `pending ${summary.pending}`,
                ];
                if (summary.failed > 0) {
                  parts.push(`failed ${summary.failed}`);
                }
                return (
                  <text
                    x={lane.x + lane.width - 14}
                    y={lane.y + 22}
                    fill={summary.failed > 0 ? "#ff9b9b" : "#6f7b8e"}
                    fontSize={10}
                    fontWeight={600}
                    textAnchor="end"
                  >
                    {parts.join(" · ")}
                  </text>
                );
              })()}
            </g>
          ))}
          {WORKFLOW_DAG_LINKS.map(([from, to]) => {
            const fromNode = dagNodeById.get(from);
            const toNode = dagNodeById.get(to);
            if (!fromNode || !toNode) return null;
            return (
              <line
                key={`${from}-${to}`}
                x1={fromNode.x + (fromNode.kind ? 20 : 52)}
                y1={fromNode.y}
                x2={toNode.x - (toNode.kind ? 20 : 52)}
                y2={toNode.y}
                stroke="#3c4048"
                strokeWidth={2}
              />
            );
          })}
          {WORKFLOW_DAG_NODES.map((node) => {
            const hasActualStages = node.actualStageIds.length > 0;
            const isDone = hasActualStages && node.actualStageIds.every((stageId) => completedStages.has(stageId));
            const isActive = hasActualStages && node.actualStageIds.includes(activeWorkflowStage ?? "");
            const isSelected = selectedDagNodeId === node.id;
            const fill = isDone ? "#2d6a3f" : isActive ? "#0e639c" : node.kind ? "#232833" : "#2c3139";
            const stroke = isSelected ? "#8ecbff" : isDone ? "#4ec9b0" : isActive ? "#57b0e5" : "#3c4048";
            return (
              <g key={node.id} onClick={() => onSelectArtifactStage(node.id)} style={{ cursor: "pointer" }}>
                {node.kind ? (
                  <>
                    <polygon
                      points={`${node.x},${node.y - 22} ${node.x + 22},${node.y} ${node.x},${node.y + 22} ${node.x - 22},${node.y}`}
                      fill={fill}
                      stroke={stroke}
                      strokeWidth={2}
                    />
                    <text x={node.x} y={node.y + 38} textAnchor="middle" fill="#e8edf7" fontSize={10} fontWeight={700}>
                      {node.label}
                    </text>
                  </>
                ) : (
                  <>
                    <rect x={node.x - 52} y={node.y - 20} width={104} height={40} rx={8} fill={fill} stroke={stroke} strokeWidth={2} />
                    <text x={node.x} y={node.y + 4} textAnchor="middle" fill="#e8edf7" fontSize={10} fontWeight={700}>
                      {node.label}
                    </text>
                  </>
                )}
              </g>
            );
          })}
        </svg>
      </div>
      <div style={styles.smallText}>
        Selected node: <strong>{selectedDagNode.label}</strong>
        {selectedDagNode.actualStageIds.length > 0 ? ` · Runtime stages: ${selectedDagNode.actualStageIds.join(", ")}` : " · Structural split/merge node"}
      </div>
    </div>
  );
}

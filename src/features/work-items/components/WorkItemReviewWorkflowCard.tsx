import type {
  AgentRun,
  Artifact,
  ModelCall,
  WorkflowRun,
  WorkflowStageHistory,
} from "../../../lib/types";
import {
  WORKFLOW_DAG_LANES,
  WORKFLOW_DAG_LINKS,
  WORKFLOW_DAG_NODES,
  formatDurationMs,
  formatInteger,
  getArtifactFileName,
  summarizeModelUsage,
  type ModelUsageSummary,
  type WorkflowDagNode,
} from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

type LaneStatus = {
  done: number;
  active: number;
  pending: number;
  failed: number;
};

type Props = {
  workflowRunId: string | null;
  latestWorkflowRun: WorkflowRun | null | undefined;
  activeWorkflowStage: string | null;
  stageLabel: string | null;
  workflowElapsedLabel: string | null;
  isStaleRun: boolean;
  workflowModelUsage: ModelUsageSummary;
  laneStatusById: Map<string, LaneStatus>;
  dagNodeById: Map<string, WorkflowDagNode>;
  completedStages: Set<string>;
  selectedDagNodeId: string;
  selectedDagNode: WorkflowDagNode;
  stageRuns: AgentRun[];
  artifactsByAgentRunId: Map<string, Artifact[]>;
  modelCallsByAgentRunId: Map<string, ModelCall[]>;
  stageHistoryForFocusedStage: WorkflowStageHistory[];
  isFailWorkflowRunPending: boolean;
  isRestartWorkflowPending: boolean;
  isPlanApprovalPending: boolean;
  isPlanRejectPending: boolean;
  isTestReviewApprovePending: boolean;
  isTestReviewRejectPending: boolean;
  onFailWorkflowRun: () => void;
  onRestartWorkflow: () => void;
  onApprovePlan: () => void;
  onRejectPlan: () => void;
  onApproveTestReview: () => void;
  onRejectTestReview: () => void;
  onSelectArtifactStage: (stageId: string) => void;
  onOpenArtifact: (artifact: Artifact) => void;
};

export function WorkItemReviewWorkflowCard({
  workflowRunId,
  latestWorkflowRun,
  activeWorkflowStage,
  stageLabel,
  workflowElapsedLabel,
  isStaleRun,
  workflowModelUsage,
  laneStatusById,
  dagNodeById,
  completedStages,
  selectedDagNodeId,
  selectedDagNode,
  stageRuns,
  artifactsByAgentRunId,
  modelCallsByAgentRunId,
  stageHistoryForFocusedStage,
  isFailWorkflowRunPending,
  isRestartWorkflowPending,
  isPlanApprovalPending,
  isPlanRejectPending,
  isTestReviewApprovePending,
  isTestReviewRejectPending,
  onFailWorkflowRun,
  onRestartWorkflow,
  onApprovePlan,
  onRejectPlan,
  onApproveTestReview,
  onRejectTestReview,
  onSelectArtifactStage,
  onOpenArtifact,
}: Props) {
  return (
    <div style={styles.detailCard}>
      <div style={styles.detailLabel}>Workflow</div>
      {workflowRunId ? (
        <>
          <div style={styles.detailValue}>
            Run: <code>{workflowRunId}</code>
          </div>
          <div style={styles.smallText}>
            Stage: {stageLabel ?? "unknown"} · Status: {latestWorkflowRun?.status ?? "unknown"}
          </div>
          {workflowElapsedLabel && (
            <div style={styles.smallText}>
              Active stage elapsed: {workflowElapsedLabel}
            </div>
          )}
          {isStaleRun && (
            <div style={styles.infoCard}>
              <div style={styles.detailValue}>
                This run appears stale. No completion/error has been recorded for the active stage.
              </div>
              <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
                <button style={styles.btnDanger} onClick={onFailWorkflowRun}>
                  {isFailWorkflowRunPending ? "Failing..." : "Mark Run Failed"}
                </button>
                <button style={styles.btn} onClick={onRestartWorkflow}>
                  {isRestartWorkflowPending ? "Restarting..." : "Restart Workflow"}
                </button>
              </div>
            </div>
          )}
          {activeWorkflowStage === "pending_plan_approval" && (
            <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
              <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={onApprovePlan}>
                {isPlanApprovalPending ? "Approving Plan..." : "Approve Plan"}
              </button>
              <button style={styles.btnDanger} onClick={onRejectPlan}>
                {isPlanRejectPending ? "Rejecting..." : "Reject Plan"}
              </button>
            </div>
          )}
          <WorkflowCostVisibility usage={workflowModelUsage} />
          <WorkflowDag
            activeWorkflowStage={activeWorkflowStage}
            completedStages={completedStages}
            dagNodeById={dagNodeById}
            laneStatusById={laneStatusById}
            selectedDagNode={selectedDagNode}
            selectedDagNodeId={selectedDagNodeId}
            latestWorkflowRun={latestWorkflowRun}
            onSelectArtifactStage={onSelectArtifactStage}
          />
          <SelectedStageDetails
            selectedDagNode={selectedDagNode}
            stageRuns={stageRuns}
            artifactsByAgentRunId={artifactsByAgentRunId}
            modelCallsByAgentRunId={modelCallsByAgentRunId}
            stageHistoryForFocusedStage={stageHistoryForFocusedStage}
            onOpenArtifact={onOpenArtifact}
          />
          {activeWorkflowStage === "pending_test_review" && (
            <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
              <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={onApproveTestReview}>
                {isTestReviewApprovePending ? "Approving Tests..." : "Approve Test Review"}
              </button>
              <button style={styles.btnDanger} onClick={onRejectTestReview}>
                {isTestReviewRejectPending ? "Rejecting..." : "Reject Test Review"}
              </button>
            </div>
          )}
        </>
      ) : (
        <div style={styles.detailValue}>No workflow run yet. Start a workflow from the Story Detail tab.</div>
      )}
    </div>
  );
}

function WorkflowCostVisibility({ usage }: { usage: ModelUsageSummary }) {
  return (
    <div style={{ marginTop: 12, paddingTop: 10, borderTop: "1px solid #32353d" }}>
      <div style={styles.detailLabel}>Cost Visibility</div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
        <span style={styles.smallText}>Calls: {formatInteger(usage.callCount)}</span>
        <span style={styles.smallText}>Failed: {formatInteger(usage.failedCallCount)}</span>
        <span style={styles.smallText}>Input tokens: {formatInteger(usage.inputTokens)}</span>
        <span style={styles.smallText}>Output tokens: {formatInteger(usage.outputTokens)}</span>
        <span style={styles.smallText}>Prompt chars: {formatInteger(usage.promptChars)}</span>
        <span style={styles.smallText}>Response chars: {formatInteger(usage.responseChars)}</span>
        <span style={styles.smallText}>Model time: {formatDurationMs(usage.durationMs)}</span>
      </div>
      {usage.providerLabels.length > 0 && (
        <div style={styles.smallText}>
          Providers: {usage.providerLabels.slice(0, 3).join(", ")}
          {usage.providerLabels.length > 3 ? ` +${usage.providerLabels.length - 3} more` : ""}
        </div>
      )}
      <div style={styles.smallText}>
        {usage.source === "per_call"
          ? "Source: per-call telemetry. Dollar estimate is hidden until provider pricing is configured."
          : usage.source === "agent_run"
            ? "Source: legacy agent-run token totals. Per-call telemetry starts with new model calls."
            : "No token or call telemetry has been recorded for this workflow yet."}
      </div>
    </div>
  );
}

function WorkflowDag({
  activeWorkflowStage,
  completedStages,
  dagNodeById,
  laneStatusById,
  selectedDagNode,
  selectedDagNodeId,
  latestWorkflowRun,
  onSelectArtifactStage,
}: {
  activeWorkflowStage: string | null;
  completedStages: Set<string>;
  dagNodeById: Map<string, WorkflowDagNode>;
  laneStatusById: Map<string, LaneStatus>;
  selectedDagNode: WorkflowDagNode;
  selectedDagNodeId: string;
  latestWorkflowRun: WorkflowRun | null | undefined;
  onSelectArtifactStage: (stageId: string) => void;
}) {
  return (
    <div style={{ marginTop: 12 }}>
      <div style={styles.detailLabel}>Stage Artifacts</div>
      <div style={styles.dagLegend}>
        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#4ec9b0", display: "inline-block" }} /> done</span>
        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#0e639c", display: "inline-block" }} /> active</span>
        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#3f4a59", display: "inline-block" }} /> pending</span>
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

function SelectedStageDetails({
  selectedDagNode,
  stageRuns,
  artifactsByAgentRunId,
  modelCallsByAgentRunId,
  stageHistoryForFocusedStage,
  onOpenArtifact,
}: {
  selectedDagNode: WorkflowDagNode;
  stageRuns: AgentRun[];
  artifactsByAgentRunId: Map<string, Artifact[]>;
  modelCallsByAgentRunId: Map<string, ModelCall[]>;
  stageHistoryForFocusedStage: WorkflowStageHistory[];
  onOpenArtifact: (artifact: Artifact) => void;
}) {
  return (
    <div style={{ marginTop: 12 }}>
      <div style={styles.detailLabel}>Selected Stage Details</div>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Node</div>
        <div style={styles.detailValue}>{selectedDagNode.label}</div>
        <div style={styles.smallText}>
          {selectedDagNode.actualStageIds.length > 0
            ? `Backed by runtime stage${selectedDagNode.actualStageIds.length > 1 ? "s" : ""}: ${selectedDagNode.actualStageIds.join(", ")}`
            : "This is a structural split/merge node used to explain the orchestrated flow."}
        </div>

        <div style={{ ...styles.detailLabel, marginTop: 12 }}>Stage Agent Runs</div>
        {selectedDagNode.actualStageIds.length === 0 ? (
          <div style={styles.smallText}>No direct agent run is attached to this structural node.</div>
        ) : stageRuns.length > 0 ? (
          <div style={styles.list}>
            {stageRuns.map((run) => {
              const runArtifacts = (artifactsByAgentRunId.get(run.id) ?? []).sort((a, b) =>
                a.created_at.localeCompare(b.created_at),
              );
              const runCalls = modelCallsByAgentRunId.get(run.id) ?? [];
              const runUsage = summarizeModelUsage(runCalls, [run]);
              const visibleRunCalls = runCalls.slice(-8);
              return (
                <div key={run.id} style={styles.listItem}>
                  <div style={styles.taskTitle}>{run.status} · {run.agent_id}</div>
                  <div style={styles.smallText}>Run: {run.id}</div>
                  <div style={styles.smallText}>Stage: {run.stage}</div>
                  <div style={styles.smallText}>
                    Started: {run.started_at}{run.ended_at ? ` · Ended: ${run.ended_at}` : ""}
                  </div>
                  <div style={styles.smallText}>
                    Usage: calls {formatInteger(runUsage.callCount)} · failed {formatInteger(runUsage.failedCallCount)} · input {formatInteger(runUsage.inputTokens)} · output {formatInteger(runUsage.outputTokens)} · model time {formatDurationMs(runUsage.durationMs)}
                  </div>
                  {runUsage.source === "agent_run" && (
                    <div style={styles.smallText}>Per-call rows were not recorded for this older run; showing aggregate run tokens.</div>
                  )}
                  {visibleRunCalls.length > 0 && (
                    <div style={{ marginTop: 8, paddingTop: 8, borderTop: "1px solid #2d3139" }}>
                      <div style={styles.detailLabel}>Model Calls</div>
                      <div style={styles.list}>
                        {visibleRunCalls.map((call) => (
                          <div key={call.id} style={{ borderLeft: "2px solid #36506f", paddingLeft: 8 }}>
                            <div style={styles.smallText}>
                              #{call.call_index} · {call.status} · {call.provider_name || call.provider_id} / {call.model_name}
                            </div>
                            <div style={styles.smallText}>
                              Input {formatInteger(call.token_count_input)} · Output {formatInteger(call.token_count_output)} · Prompt {formatInteger(call.prompt_chars)} chars · Response {formatInteger(call.response_chars)} chars · Max {formatInteger(call.max_tokens)} · {formatDurationMs(call.duration_ms)}
                            </div>
                            {call.error_message && <div style={styles.warning}>{call.error_message}</div>}
                          </div>
                        ))}
                      </div>
                      {runCalls.length > visibleRunCalls.length && (
                        <div style={styles.smallText}>
                          Showing latest {visibleRunCalls.length} of {runCalls.length} model calls.
                        </div>
                      )}
                    </div>
                  )}
                  {run.error_message && <div style={styles.warning}>{run.error_message}</div>}
                  <div style={{ ...styles.detailLabel, marginTop: 8 }}>Input / Output / Attachments</div>
                  {runArtifacts.length > 0 ? (
                    <div style={styles.list}>
                      {runArtifacts.map((artifact) => (
                        <button
                          key={artifact.id}
                          style={{ ...styles.ghostBtn, textAlign: "left", width: "100%" }}
                          onClick={() => onOpenArtifact(artifact)}
                        >
                          {getArtifactFileName(artifact)} · {artifact.artifact_type}
                        </button>
                      ))}
                    </div>
                  ) : (
                    <div style={styles.warning}>No attachments generated for this run yet.</div>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <div style={styles.warning}>No agent runs recorded for this stage yet.</div>
        )}

        <div style={{ ...styles.detailLabel, marginTop: 12 }}>Stage Transition History</div>
        {selectedDagNode.actualStageIds.length === 0 ? (
          <div style={styles.smallText}>No direct transition history is attached to this structural node.</div>
        ) : stageHistoryForFocusedStage.length > 0 ? (
          <div style={styles.list}>
            {stageHistoryForFocusedStage.slice(-8).map((entry) => (
              <div key={entry.id} style={styles.listItem}>
                <div style={styles.taskTitle}>
                  {entry.from_stage.replace(/_/g, " ")} → {entry.to_stage.replace(/_/g, " ")}
                </div>
                <div style={styles.smallText}>{entry.trigger} · {entry.transitioned_at}</div>
                <div style={styles.smallText}>{entry.notes}</div>
              </div>
            ))}
          </div>
        ) : (
          <div style={styles.warning}>No transitions recorded for this stage yet.</div>
        )}
      </div>
    </div>
  );
}

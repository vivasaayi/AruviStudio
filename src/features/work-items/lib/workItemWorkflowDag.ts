import workflowTopology from "../../../../src-tauri/resources/workflow-topology.json";
import type {
  AgentRun,
  Artifact,
  ModelCall,
  WorkflowRun,
  WorkflowStageHistory,
} from "../../../lib/types";

export type WorkflowDagNode = {
  id: string;
  label: string;
  x: number;
  y: number;
  kind?: "stage" | "split" | "merge";
  actualStageIds: string[];
};

export type WorkflowDagLane = {
  id: string;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  nodeIds: string[];
};

export const WORKFLOW_DAG_NODES: WorkflowDagNode[] = workflowTopology.nodes as WorkflowDagNode[];
export const WORKFLOW_DAG_LINKS: Array<[string, string]> = workflowTopology.links.map(([from, to]) => [from, to]);
export const WORKFLOW_DAG_LANES: WorkflowDagLane[] = workflowTopology.lanes;

export function parseSqliteUtcTimestamp(value?: string | null): number | null {
  if (!value) return null;
  const parsed = Date.parse(value.includes("T") ? value : `${value.replace(" ", "T")}Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

export function findLatestAgentRunForStage(agentRuns: AgentRun[] | null | undefined, stage: string | null | undefined): AgentRun | null {
  if (!stage || !agentRuns?.length) {
    return null;
  }
  for (let index = agentRuns.length - 1; index >= 0; index -= 1) {
    if (agentRuns[index].stage === stage) {
      return agentRuns[index];
    }
  }
  return null;
}

export function getRunningAgentRunStartMs(agentRun: AgentRun | null | undefined): number | null {
  if (!agentRun || agentRun.status !== "running") {
    return null;
  }
  return parseSqliteUtcTimestamp(agentRun.started_at);
}

export function formatWorkflowElapsedLabel(startMs: number | null, nowMs = Date.now()): string | null {
  if (!startMs) {
    return null;
  }
  const elapsedMs = nowMs - startMs;
  if (elapsedMs < 0) {
    return null;
  }
  const mins = Math.floor(elapsedMs / 60000);
  const secs = Math.floor((elapsedMs % 60000) / 1000);
  return `${mins}m ${secs}s`;
}

export function isWorkflowRunStale(startMs: number | null, workflowStatus: WorkflowRun["status"] | null | undefined, nowMs = Date.now()): boolean {
  if (!startMs || workflowStatus !== "running") {
    return false;
  }
  return nowMs - startMs > 7 * 60 * 1000;
}

export function filterArtifactsForWorkflowStages(
  artifacts: Artifact[] | null | undefined,
  stageNames: string[],
  workflowRunId: string | null | undefined,
): Artifact[] {
  return (artifacts ?? []).filter((artifact) => {
    if (workflowRunId && artifact.workflow_run_id !== workflowRunId) {
      return false;
    }
    if (stageNames.some((stageName) => artifact.artifact_type.startsWith(`${stageName}_`))) {
      return true;
    }
    if (stageNames.includes("coding")) {
      return artifact.artifact_type === "coding_tool_trace" || artifact.artifact_type === "coding_applied_files";
    }
    return false;
  });
}

export function filterWorkflowHistoryForStages(
  workflowHistory: WorkflowStageHistory[] | null | undefined,
  stageNames: string[],
): WorkflowStageHistory[] {
  return (workflowHistory ?? []).filter(
    (entry) => stageNames.includes(entry.from_stage) || stageNames.includes(entry.to_stage),
  );
}

export function groupArtifactsByAgentRunId(artifacts: Artifact[]): Map<string, Artifact[]> {
  const map = new Map<string, Artifact[]>();
  for (const artifact of artifacts) {
    if (!artifact.agent_run_id) {
      continue;
    }
    const list = map.get(artifact.agent_run_id) ?? [];
    list.push(artifact);
    map.set(artifact.agent_run_id, list);
  }
  return map;
}

export function groupModelCallsByAgentRunId(modelCalls: ModelCall[] | null | undefined): Map<string, ModelCall[]> {
  const map = new Map<string, ModelCall[]>();
  for (const call of modelCalls ?? []) {
    if (!call.agent_run_id) {
      continue;
    }
    const list = map.get(call.agent_run_id) ?? [];
    list.push(call);
    map.set(call.agent_run_id, list);
  }
  for (const calls of map.values()) {
    calls.sort((a, b) => a.call_index - b.call_index || a.created_at.localeCompare(b.created_at));
  }
  return map;
}

export type WorkflowLaneStatus = {
  done: number;
  active: number;
  pending: number;
  failed: number;
};

export function buildWorkflowLaneStatusById({
  lanes,
  nodes,
  completedStages,
  activeWorkflowStage,
  workflowStatus,
}: {
  lanes: WorkflowDagLane[];
  nodes: WorkflowDagNode[];
  completedStages: Set<string>;
  activeWorkflowStage: string | null | undefined;
  workflowStatus: WorkflowRun["status"] | null | undefined;
}): Map<string, WorkflowLaneStatus> {
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const map = new Map<string, WorkflowLaneStatus>();
  for (const lane of lanes) {
    const status: WorkflowLaneStatus = { done: 0, active: 0, pending: 0, failed: 0 };
    for (const nodeId of lane.nodeIds) {
      const node = nodeById.get(nodeId);
      if (!node) {
        continue;
      }
      if (node.actualStageIds.length === 0) {
        status.pending += 1;
        continue;
      }
      const hasFailed = node.actualStageIds.some(
        (stageId) => stageId === "failed" || (workflowStatus === "failed" && activeWorkflowStage === stageId),
      );
      const isActive = node.actualStageIds.includes(activeWorkflowStage ?? "");
      const isDone = node.actualStageIds.every((stageId) => completedStages.has(stageId) || stageId === "done");
      if (hasFailed) {
        status.failed += 1;
      } else if (isActive) {
        status.active += 1;
      } else if (isDone) {
        status.done += 1;
      } else {
        status.pending += 1;
      }
    }
    map.set(lane.id, status);
  }
  return map;
}

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

export const WORKFLOW_DAG_NODES: WorkflowDagNode[] = [
  { id: "draft", label: "Draft", x: 90, y: 120, actualStageIds: ["draft"] },
  { id: "requirement_analysis", label: "Requirement Analysis", x: 280, y: 120, actualStageIds: ["requirement_analysis"] },
  { id: "planning_split", label: "Plan Split", x: 470, y: 120, kind: "split", actualStageIds: [] },
  { id: "architecture_plan", label: "Architecture Plan", x: 660, y: 40, actualStageIds: ["planning"] },
  { id: "unit_test_plan", label: "Unit Test Plan", x: 660, y: 100, actualStageIds: ["planning"] },
  { id: "integration_test_plan", label: "Integration Plan", x: 660, y: 160, actualStageIds: ["planning"] },
  { id: "ui_test_plan", label: "UI Test Plan", x: 660, y: 220, actualStageIds: ["planning"] },
  { id: "planning_merge", label: "Lead Merge", x: 860, y: 120, kind: "merge", actualStageIds: ["pending_plan_approval"] },
  { id: "coding", label: "Coding", x: 1060, y: 120, actualStageIds: ["coding"] },
  { id: "verification_split", label: "Verify Split", x: 1260, y: 120, kind: "split", actualStageIds: [] },
  { id: "unit_test_generation", label: "Unit Tests", x: 1460, y: 40, actualStageIds: ["unit_test_generation"] },
  { id: "integration_test_generation", label: "Integration Tests", x: 1460, y: 100, actualStageIds: ["integration_test_generation"] },
  { id: "ui_test_planning", label: "UI Verification", x: 1460, y: 160, actualStageIds: ["ui_test_planning"] },
  { id: "qa_validation", label: "QA Validation", x: 1660, y: 40, actualStageIds: ["qa_validation"] },
  { id: "security_review", label: "Security Review", x: 1660, y: 100, actualStageIds: ["security_review"] },
  { id: "performance_review", label: "Performance Review", x: 1660, y: 160, actualStageIds: ["performance_review"] },
  { id: "verification_merge", label: "Test Review", x: 1860, y: 120, kind: "merge", actualStageIds: ["pending_test_review"] },
  { id: "push_preparation", label: "Push Prep", x: 2060, y: 120, actualStageIds: ["push_preparation"] },
  { id: "git_push", label: "Git Push", x: 2240, y: 120, actualStageIds: ["git_push"] },
  { id: "done", label: "Done", x: 2420, y: 120, actualStageIds: ["done"] },
];

export const WORKFLOW_DAG_LINKS: Array<[string, string]> = [
  ["draft", "requirement_analysis"],
  ["requirement_analysis", "planning_split"],
  ["planning_split", "architecture_plan"],
  ["planning_split", "unit_test_plan"],
  ["planning_split", "integration_test_plan"],
  ["planning_split", "ui_test_plan"],
  ["architecture_plan", "planning_merge"],
  ["unit_test_plan", "planning_merge"],
  ["integration_test_plan", "planning_merge"],
  ["ui_test_plan", "planning_merge"],
  ["planning_merge", "coding"],
  ["coding", "verification_split"],
  ["verification_split", "unit_test_generation"],
  ["verification_split", "integration_test_generation"],
  ["verification_split", "ui_test_planning"],
  ["verification_split", "qa_validation"],
  ["verification_split", "security_review"],
  ["verification_split", "performance_review"],
  ["unit_test_generation", "verification_merge"],
  ["integration_test_generation", "verification_merge"],
  ["ui_test_planning", "verification_merge"],
  ["qa_validation", "verification_merge"],
  ["security_review", "verification_merge"],
  ["performance_review", "verification_merge"],
  ["verification_merge", "push_preparation"],
  ["push_preparation", "git_push"],
  ["git_push", "done"],
];

export const WORKFLOW_DAG_LANES: WorkflowDagLane[] = [
  { id: "intake", label: "Intake", x: 20, y: 12, width: 360, height: 236, nodeIds: ["draft", "requirement_analysis"] },
  { id: "planning_swarm", label: "Planning Swarm", x: 400, y: 12, width: 540, height: 236, nodeIds: ["planning_split", "architecture_plan", "unit_test_plan", "integration_test_plan", "ui_test_plan", "planning_merge"] },
  { id: "execution", label: "Execution", x: 960, y: 12, width: 260, height: 236, nodeIds: ["coding"] },
  { id: "verification_swarm", label: "Verification Swarm", x: 1240, y: 12, width: 660, height: 236, nodeIds: ["verification_split", "unit_test_generation", "integration_test_generation", "ui_test_planning", "qa_validation", "security_review", "performance_review", "verification_merge"] },
  { id: "delivery", label: "Delivery", x: 1920, y: 12, width: 560, height: 236, nodeIds: ["push_preparation", "git_push", "done"] },
];

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

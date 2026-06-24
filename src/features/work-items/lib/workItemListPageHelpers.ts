import type React from "react";

import type {
  AgentRun,
  Artifact,
  Capability,
  ExternalCliRun,
  ExternalCliRunEvent,
  ModelCall,
  ProductArea,
  WorkItem,
  WorkflowRun,
  WorkflowStageHistory,
} from "../../../lib/types";

export const statusColors: Record<string, string> = {
  draft: "#444",
  ready_for_review: "#569cd6",
  approved: "#4ec9b0",
  in_planning: "#dcdcaa",
  in_progress: "#ce9178",
  in_validation: "#c586c0",
  waiting_human_review: "#d7ba7d",
  done: "#6a9955",
  blocked: "#f44747",
  failed: "#f44747",
  cancelled: "#666",
};

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

export type WorkspaceBranchMode = "default" | "work_item" | "custom";
export type ExternalCliProvider = "codex" | "claude" | "cursor" | "copilot";

export const EXTERNAL_CLI_PROVIDERS: Array<{ provider: ExternalCliProvider; label: string }> = [
  { provider: "codex", label: "Codex" },
  { provider: "claude", label: "Claude" },
  { provider: "cursor", label: "Cursor" },
  { provider: "copilot", label: "Copilot" },
];
export const EXTERNAL_CLI_TRACE_LIMIT = 500;
export const WORK_ITEM_PAGE_SIZE = 100;
export const SUB_WORK_ITEM_PAGE_SIZE = 500;
export const BACKLOG_ROW_ESTIMATED_HEIGHT = 116;
export const BACKLOG_OVERSCAN_ROWS = 6;

export function workItemBranchName(title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
  return `work/${slug || "work-item"}`;
}

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

export function getArtifactFileName(artifact: Artifact): string {
  const segments = artifact.storage_path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? artifact.artifact_type;
}

export type ModelUsageSummary = {
  source: "per_call" | "agent_run" | "none";
  callCount: number;
  failedCallCount: number;
  inputTokens: number | null;
  outputTokens: number | null;
  promptChars: number;
  responseChars: number;
  durationMs: number | null;
  providerLabels: string[];
};

export function formatInteger(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  return new Intl.NumberFormat().format(value);
}

export function formatDurationMs(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  if (value < 1000) return `${Math.max(0, Math.round(value))}ms`;
  const totalSeconds = Math.round(value / 1000);
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
}

export function formatExternalCliTerminalEvent(event: ExternalCliRunEvent): string {
  const stream = event.stream === "lifecycle" ? "INFO" : event.stream.toUpperCase();
  const prefix = `[${event.created_at}] #${event.sequence} ${stream}: `;
  const message = event.message || "";
  const lines = message.split(/\r?\n/);
  return lines
    .map((line, index) => (index === 0 ? `${prefix}${line}` : `${" ".repeat(prefix.length)}${line}`))
    .join("\n");
}

export function formatExternalCliTerminal(events: ExternalCliRunEvent[]): string {
  return events
    .slice()
    .sort((a, b) => a.sequence - b.sequence)
    .map(formatExternalCliTerminalEvent)
    .join("\n");
}

export function sumKnown(values: Array<number | null | undefined>): number | null {
  let total = 0;
  let hasValue = false;
  for (const value of values) {
    if (value === null || value === undefined || !Number.isFinite(value)) continue;
    total += value;
    hasValue = true;
  }
  return hasValue ? total : null;
}

export function summarizeModelUsage(calls: ModelCall[], runs: AgentRun[]): ModelUsageSummary {
  if (calls.length > 0) {
    const providerLabels = Array.from(
      new Set(calls.map((call) => `${call.provider_name || call.provider_id} / ${call.model_name}`).filter(Boolean)),
    );
    return {
      source: "per_call",
      callCount: calls.length,
      failedCallCount: calls.filter((call) => call.status === "failed").length,
      inputTokens: sumKnown(calls.map((call) => call.token_count_input)),
      outputTokens: sumKnown(calls.map((call) => call.token_count_output)),
      promptChars: calls.reduce((total, call) => total + call.prompt_chars, 0),
      responseChars: calls.reduce((total, call) => total + call.response_chars, 0),
      durationMs: sumKnown(calls.map((call) => call.duration_ms)),
      providerLabels,
    };
  }

  const inputTokens = sumKnown(runs.map((run) => run.token_count_input));
  const outputTokens = sumKnown(runs.map((run) => run.token_count_output));
  if (inputTokens !== null || outputTokens !== null) {
    return {
      source: "agent_run",
      callCount: 0,
      failedCallCount: 0,
      inputTokens,
      outputTokens,
      promptChars: 0,
      responseChars: 0,
      durationMs: sumKnown(runs.map((run) => run.duration_ms)),
      providerLabels: [],
    };
  }

  return {
    source: "none",
    callCount: 0,
    failedCallCount: 0,
    inputTokens: null,
    outputTokens: null,
    promptChars: 0,
    responseChars: 0,
    durationMs: null,
    providerLabels: [],
  };
}

export function formatWorkItemTypeLabel(workItemType: WorkItem["work_item_type"]): string {
  const canonicalLabels: Record<WorkItem["work_item_type"], string> = {
    story: "story",
    task: "task",
    setup: "setup",
    bug: "bug fix",
    refactor: "refactor",
    test: "test",
    review: "review",
    security_fix: "security fix",
    performance_improvement: "performance",
  };
  return canonicalLabels[workItemType] ?? workItemType.replace(/_/g, " ");
}

export function getWorkItemExecutionSteps(workItem: WorkItem, workspaceName?: string | null): string[] {
  if (workItem.work_item_type === "setup") {
    return [
      "Create the local workspace folder for this product.",
      "Register the workspace inside AruviStudio.",
      "Enable version history and create the default branch.",
      "Create baseline files such as README, .gitignore, and tests/ scaffold.",
      `Attach the workspace to the current product or product area scope${workspaceName ? ` (${workspaceName})` : ""}.`,
      "Verify downstream stories and tasks can inherit the workspace automatically.",
    ];
  }

  const steps = [
    "Confirm requirements, constraints, and acceptance criteria are complete.",
    "Resolve the workspace and working branch for this delivery story.",
    "Implement the scoped change in code.",
    "Produce verification artifacts for review.",
  ];

  if (workItem.work_item_type === "story") {
    steps.push("Run or generate unit, integration, and UI validation coverage as required.");
  }

  return steps;
}

export function buildCapabilityPath(
  capability: Capability,
  productAreaById: Map<string, ProductArea>,
  capabilityById: Map<string, Capability>,
) {
  const capabilityNames: string[] = [];
  const visited = new Set<string>();
  let current: Capability | undefined = capability;

  while (current && !visited.has(current.id)) {
    visited.add(current.id);
    capabilityNames.push(current.name);
    current = current.parent_capability_id ? capabilityById.get(current.parent_capability_id) : undefined;
  }

  const productArea = productAreaById.get(capability.product_area_id);
  return [...(productArea ? [productArea.name] : []), ...capabilityNames.reverse()];
}

export function moveTaskIdToIndex(ids: string[], id: string, nextIndex: number): string[] {
  const currentIndex = ids.indexOf(id);
  if (currentIndex === -1 || nextIndex < 0 || nextIndex >= ids.length) {
    return ids;
  }
  const nextIds = [...ids];
  const [item] = nextIds.splice(currentIndex, 1);
  nextIds.splice(nextIndex, 0, item);
  return nextIds;
}

export function orderWorkItemsByIds(workItems: WorkItem[], orderedIds: string[]) {
  if (orderedIds.length === 0) {
    return workItems;
  }
  const rank = new Map(orderedIds.map((id, index) => [id, index]));
  return [...workItems].sort((a, b) => (rank.get(a.id) ?? Number.MAX_SAFE_INTEGER) - (rank.get(b.id) ?? Number.MAX_SAFE_INTEGER));
}

export function formatExternalCliCommand(run: ExternalCliRun): string {
  const args = run.args.map((arg) => (arg.includes(" ") ? `"${arg.slice(0, 160)}${arg.length > 160 ? "..." : ""}"` : arg));
  return [run.command, ...args].join(" ");
}

export type RuntimeTone = "neutral" | "info" | "success" | "warning" | "danger";

export function describeWorkItemRuntime(workItem: WorkItem, workflowRun: WorkflowRun | null) {
  if (!workflowRun) {
    if (workItem.status === "approved") {
      return {
        label: "Ready",
        tone: "info" as RuntimeTone,
        detail: "Approved and ready to start",
        stageLabel: null,
      };
    }
    if (workItem.status === "done") {
      return {
        label: "Completed",
        tone: "success" as RuntimeTone,
        detail: "Work item marked done",
        stageLabel: null,
      };
    }
    if (workItem.status === "failed" || workItem.status === "blocked" || workItem.status === "cancelled") {
      return {
        label: workItem.status.replace(/_/g, " "),
        tone: "danger" as RuntimeTone,
        detail: `Work item ${workItem.status.replace(/_/g, " ")}`,
        stageLabel: null,
      };
    }
    return {
      label: "Not started",
      tone: "neutral" as RuntimeTone,
      detail: `Work item ${workItem.status.replace(/_/g, " ")}`,
      stageLabel: null,
    };
  }

  const stageLabel = workflowRun.current_stage.replace(/_/g, " ");
  if (workflowRun.status === "running") {
    return {
      label: workflowRun.current_stage.startsWith("pending_") ? "Awaiting review" : "Running",
      tone: workflowRun.current_stage.startsWith("pending_") ? "warning" as RuntimeTone : "info" as RuntimeTone,
      detail: `Workflow ${workflowRun.status} at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "completed" || workflowRun.current_stage === "done") {
    return {
      label: "Completed",
      tone: "success" as RuntimeTone,
      detail: "Workflow completed successfully",
      stageLabel,
    };
  }
  if (workflowRun.status === "failed") {
    return {
      label: "Failed",
      tone: "danger" as RuntimeTone,
      detail: `Workflow failed at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "cancelled") {
    return {
      label: "Cancelled",
      tone: "danger" as RuntimeTone,
      detail: `Workflow cancelled at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "paused") {
    return {
      label: "Paused",
      tone: "warning" as RuntimeTone,
      detail: `Workflow paused at ${stageLabel}`,
      stageLabel,
    };
  }

  return {
    label: workflowRun.status,
    tone: "neutral" as RuntimeTone,
    detail: `Workflow ${workflowRun.status} at ${stageLabel}`,
    stageLabel,
  };
}

export function getToneBadgeStyle(tone: RuntimeTone): React.CSSProperties {
  switch (tone) {
    case "info":
      return { backgroundColor: "#0e639c", color: "#fff" };
    case "success":
      return { backgroundColor: "#2d6a3f", color: "#fff" };
    case "warning":
      return { backgroundColor: "#7a5b16", color: "#fff" };
    case "danger":
      return { backgroundColor: "#8b2d2d", color: "#fff" };
    default:
      return { backgroundColor: "#444", color: "#fff" };
  }
}

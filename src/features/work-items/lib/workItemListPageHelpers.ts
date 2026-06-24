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
} from "../../../lib/types";

export * from "./workItemWorkflowDag";

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

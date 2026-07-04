import { describe, expect, it } from "vitest";

import {
  buildWorkflowLaneStatusById,
  filterArtifactsForWorkflowStages,
  filterWorkflowHistoryForStages,
  findLatestAgentRunForStage,
  formatWorkflowElapsedLabel,
  getRunningAgentRunStartMs,
  groupArtifactsByAgentRunId,
  groupModelCallsByAgentRunId,
  isWorkflowRunStale,
  parseSqliteUtcTimestamp,
} from "./workItemListPageHelpers";
import type { AgentRun, Artifact, ModelCall, WorkflowRun, WorkflowStageHistory } from "../../../lib/types";

const agentRun = (overrides: Partial<AgentRun>): AgentRun => ({
  id: "run-1",
  workflow_run_id: "workflow-1",
  agent_id: "agent-1",
  stage: "coding",
  status: "completed",
  prompt_snapshot_path: null,
  output_snapshot_path: null,
  token_count_input: null,
  token_count_output: null,
  duration_ms: null,
  error_message: null,
  started_at: "2026-01-01 00:00:00",
  ended_at: null,
  created_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

const artifact = (overrides: Partial<Artifact>): Artifact => ({
  id: "artifact-1",
  work_item_id: "work-1",
  workflow_run_id: "workflow-1",
  agent_run_id: null,
  artifact_type: "coding_tool_trace",
  storage_path: "/tmp/artifact.txt",
  summary: "",
  content_type: "text/plain",
  size_bytes: null,
  created_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

const modelCall = (overrides: Partial<ModelCall>): ModelCall => ({
  id: "call-1",
  source_kind: "agent_run",
  source_id: "run-1",
  source_label: "Agent",
  workflow_run_id: "workflow-1",
  agent_run_id: "run-1",
  work_item_id: "work-1",
  product_id: "product-1",
  session_id: null,
  agent_id: "agent-1",
  stage: "coding",
  provider_id: "provider-1",
  provider_name: "Provider",
  provider_type: "openai",
  provider_base_url: "",
  model_id: "model-1",
  model_name: "model",
  call_index: 0,
  request_message_count: 1,
  prompt_chars: 10,
  response_chars: 20,
  request_snapshot_path: null,
  response_snapshot_path: null,
  max_tokens: null,
  temperature: null,
  token_count_input: null,
  token_count_output: null,
  duration_ms: null,
  status: "completed",
  error_message: null,
  created_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

const historyEntry = (overrides: Partial<WorkflowStageHistory>): WorkflowStageHistory => ({
  id: "history-1",
  workflow_run_id: "workflow-1",
  from_stage: "draft",
  to_stage: "coding",
  trigger: "advance",
  notes: "",
  transitioned_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("workItemListPageHelpers workflow derivations", () => {
  it("selects the latest agent run for a stage", () => {
    expect(findLatestAgentRunForStage([
      agentRun({ id: "old", stage: "coding" }),
      agentRun({ id: "other", stage: "planning" }),
      agentRun({ id: "new", stage: "coding" }),
    ], "coding")?.id).toBe("new");
  });

  it("formats running durations and stale state", () => {
    const startMs = getRunningAgentRunStartMs(agentRun({ status: "running", started_at: "2026-01-01 00:00:00" }));
    expect(startMs).toBe(parseSqliteUtcTimestamp("2026-01-01 00:00:00"));
    expect(formatWorkflowElapsedLabel(startMs, startMs! + 125_000)).toBe("2m 5s");
    expect(isWorkflowRunStale(startMs, "running", startMs! + 7 * 60 * 1000 + 1)).toBe(true);
    expect(isWorkflowRunStale(startMs, "completed", startMs! + 10 * 60 * 1000)).toBe(false);
  });

  it("filters stage artifacts and includes legacy coding artifact names", () => {
    const artifacts = [
      artifact({ id: "coding-trace", artifact_type: "coding_tool_trace" }),
      artifact({ id: "coding-files", artifact_type: "coding_applied_files" }),
      artifact({ id: "qa", artifact_type: "qa_validation_report" }),
      artifact({ id: "other-workflow", workflow_run_id: "workflow-2", artifact_type: "coding_tool_trace" }),
    ];

    expect(filterArtifactsForWorkflowStages(artifacts, ["coding"], "workflow-1").map((item) => item.id))
      .toEqual(["coding-trace", "coding-files"]);
    expect(filterArtifactsForWorkflowStages(artifacts, ["qa_validation"], "workflow-1").map((item) => item.id))
      .toEqual(["qa"]);
  });

  it("filters workflow history by either side of the transition", () => {
    const entries = [
      historyEntry({ id: "from", from_stage: "coding", to_stage: "qa_validation" }),
      historyEntry({ id: "to", from_stage: "planning", to_stage: "coding" }),
      historyEntry({ id: "miss", from_stage: "draft", to_stage: "planning" }),
    ];

    expect(filterWorkflowHistoryForStages(entries, ["coding"]).map((entry) => entry.id)).toEqual(["from", "to"]);
  });

  it("groups artifacts and model calls by agent run id", () => {
    expect(Array.from(groupArtifactsByAgentRunId([
      artifact({ id: "a1", agent_run_id: "run-1" }),
      artifact({ id: "ignored", agent_run_id: null }),
      artifact({ id: "a2", agent_run_id: "run-1" }),
    ]).get("run-1") ?? []).map((item) => item.id)).toEqual(["a1", "a2"]);

    expect((groupModelCallsByAgentRunId([
      modelCall({ id: "second", agent_run_id: "run-1", call_index: 2, created_at: "2026-01-01T00:00:00Z" }),
      modelCall({ id: "first", agent_run_id: "run-1", call_index: 1, created_at: "2026-01-01T00:00:00Z" }),
      modelCall({ id: "ignored", agent_run_id: null }),
    ]).get("run-1") ?? []).map((call) => call.id)).toEqual(["first", "second"]);
  });

  it("builds lane status counts from workflow stage state", () => {
    const statusById = buildWorkflowLaneStatusById({
      lanes: [{ id: "lane", label: "Lane", x: 0, y: 0, width: 1, height: 1, nodeIds: ["done-node", "active-node", "pending-node", "split-node"] }],
      nodes: [
        { id: "done-node", label: "Done", x: 0, y: 0, actualStageIds: ["draft"] },
        { id: "active-node", label: "Active", x: 0, y: 0, actualStageIds: ["coding"] },
        { id: "pending-node", label: "Pending", x: 0, y: 0, actualStageIds: ["qa_validation"] },
        { id: "split-node", label: "Split", x: 0, y: 0, actualStageIds: [] },
      ],
      completedStages: new Set(["draft"]),
      activeWorkflowStage: "coding",
      workflowStatus: "running" as WorkflowRun["status"],
    });

    expect(statusById.get("lane")).toEqual({ done: 1, active: 1, pending: 2, failed: 0 });
  });
});

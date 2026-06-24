import { invoke } from "./core";
import type {
  AgentRun,
  ExternalCliRun,
  ExternalCliRunEvent,
  ModelCall,
  WorkflowRun,
  WorkflowStageHistory,
} from "../types";

// Workflow commands
export const startWorkItemWorkflow = (workItemId: string) =>
  invoke<string>("start_work_item_workflow", { workItemId, work_item_id: workItemId });
export const getWorkflowRun = (workflowRunId: string) =>
  invoke<WorkflowRun>("get_workflow_run", { workflowRunId, workflow_run_id: workflowRunId });
export const getLatestWorkflowRunForWorkItem = (workItemId: string) =>
  invoke<WorkflowRun | null>("get_latest_workflow_run_for_work_item", { workItemId, work_item_id: workItemId });
export const getWorkflowHistory = (workflowRunId: string) =>
  invoke<WorkflowStageHistory[]>("get_workflow_history", { workflowRunId, workflow_run_id: workflowRunId });
export const handleWorkflowUserAction = (data: {
  workflowRunId: string;
  action: "approve" | "reject" | "pause" | "resume" | "cancel";
  notes?: string;
}) =>
  invoke<void>("handle_workflow_user_action", {
    workflowRunId: data.workflowRunId,
    workflow_run_id: data.workflowRunId,
    action: data.action,
    notes: data.notes ?? null,
  });
export const listAgentRunsForWorkflow = (workflowRunId: string) =>
  invoke<AgentRun[]>("list_agent_runs_for_workflow", {
    workflowRunId,
    workflow_run_id: workflowRunId,
  });
export const listAgentModelCallsForWorkflow = (workflowRunId: string) =>
  invoke<ModelCall[]>("list_agent_model_calls_for_workflow", {
    workflowRunId,
    workflow_run_id: workflowRunId,
  });
export const invokeExternalCliForWorkItem = (data: { workItemId: string; provider: "codex" | "claude" | "cursor" | "copilot" }) =>
  invoke<ExternalCliRun>("invoke_external_cli_for_work_item", {
    workItemId: data.workItemId,
    work_item_id: data.workItemId,
    provider: data.provider,
  });
export const listExternalCliRunsForWorkItem = (workItemId: string) =>
  invoke<ExternalCliRun[]>("list_external_cli_runs_for_work_item", {
    workItemId,
    work_item_id: workItemId,
  });
export const listExternalCliRunEvents = (runId: string, limit = 500) =>
  invoke<ExternalCliRunEvent[]>("list_external_cli_run_events", {
    runId,
    run_id: runId,
    limit,
  });
export const markWorkflowRunFailed = (workflowRunId: string, reason?: string) =>
  invoke<void>("mark_workflow_run_failed", {
    workflowRunId,
    workflow_run_id: workflowRunId,
    reason: reason ?? null,
  });
export const restartWorkflowRun = (workflowRunId: string) =>
  invoke<string>("restart_workflow_run", {
    workflowRunId,
    workflow_run_id: workflowRunId,
  });

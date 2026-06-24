import type { ModelCall } from "../../../lib/types";

export type CallFilter = "all" | "desktop" | "mobile" | "workflow" | "planner" | "failed";

export interface ModelCallSession {
  key: string;
  label: string;
  sessionId: string | null;
  sourceKind: string;
  sourceId: string | null;
  status: string;
  calls: ModelCall[];
  callCount: number;
  startedAt: string;
  endedAt: string;
  providerLine: string;
  modelLine: string;
  tokenCountInput: number | null;
  tokenCountOutput: number | null;
  durationMs: number | null;
}

export const MODEL_CALL_FILTERS: Array<{ id: CallFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "desktop", label: "Desktop" },
  { id: "mobile", label: "Mobile" },
  { id: "workflow", label: "Workflow" },
  { id: "planner", label: "Planner" },
  { id: "failed", label: "Failed" },
];

export function formatInteger(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  return new Intl.NumberFormat().format(value);
}

export function formatDurationMs(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  if (value < 1000) return `${Math.max(0, Math.round(value))}ms`;
  const seconds = Math.round(value / 1000);
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
}

export function buildModelCallSessions(calls: ModelCall[]): ModelCallSession[] {
  const grouped = new Map<string, ModelCall[]>();
  calls.forEach((call) => {
    const key = modelCallSessionKey(call);
    grouped.set(key, [...(grouped.get(key) ?? []), call]);
  });

  return Array.from(grouped.entries())
    .map(([key, groupedCalls]) => {
      const orderedCalls = [...groupedCalls].sort(
        (a, b) => a.call_index - b.call_index || a.created_at.localeCompare(b.created_at),
      );
      const latestCall = orderedCalls.reduce(
        (latest, call) => (call.created_at > latest.created_at ? call : latest),
        orderedCalls[0],
      );
      const firstCall = orderedCalls[0];
      const providers = Array.from(
        new Set(orderedCalls.map((call) => call.provider_name || call.provider_id).filter(Boolean)),
      );
      const models = Array.from(new Set(orderedCalls.map((call) => call.model_name).filter(Boolean)));
      return {
        key,
        label: sourceLabel(latestCall),
        sessionId: latestCall.session_id,
        sourceKind: latestCall.source_kind,
        sourceId: latestCall.source_id,
        status: sessionStatus(orderedCalls),
        calls: orderedCalls,
        callCount: orderedCalls.length,
        startedAt: firstCall.created_at,
        endedAt: latestCall.created_at,
        providerLine: providers.length > 1 ? `${providers.length} providers` : providers[0] ?? "n/a",
        modelLine: models.length > 1 ? `${models.length} models` : models[0] ?? "n/a",
        tokenCountInput: finiteTotal(orderedCalls.map((call) => call.token_count_input)),
        tokenCountOutput: finiteTotal(orderedCalls.map((call) => call.token_count_output)),
        durationMs: finiteTotal(orderedCalls.map((call) => call.duration_ms)),
      };
    })
    .sort((a, b) => b.endedAt.localeCompare(a.endedAt));
}

export function matchesModelCallFilter(call: ModelCall, filter: CallFilter) {
  if (filter === "all") return true;
  if (filter === "failed") return call.status === "failed";
  if (filter === "desktop") return call.source_kind.startsWith("desktop");
  if (filter === "mobile") return call.source_kind.startsWith("mobile");
  if (filter === "workflow") return call.source_kind === "workflow_agent";
  if (filter === "planner") return call.source_kind.includes("planner") || call.source_kind.includes("repository_analysis");
  return true;
}

export function modelCallDetailRows(call: ModelCall) {
  return [
    ["Source Kind", call.source_kind],
    ["Source ID", call.source_id ?? "n/a"],
    ["Provider", call.provider_name || call.provider_id],
    ["Provider Type", call.provider_type || "n/a"],
    ["Provider URL", call.provider_base_url || "n/a"],
    ["Model", call.model_name],
    ["Model ID", call.model_id ?? "n/a"],
    ["Call Index", formatInteger(call.call_index)],
    ["Status", call.status],
    ["Created", call.created_at],
    ["Duration", formatDurationMs(call.duration_ms)],
    ["Messages", formatInteger(call.request_message_count)],
    ["Input Tokens", formatInteger(call.token_count_input)],
    ["Output Tokens", formatInteger(call.token_count_output)],
    ["Prompt Chars", formatInteger(call.prompt_chars)],
    ["Response Chars", formatInteger(call.response_chars)],
    ["Request Snapshot", call.request_snapshot_path ?? "n/a"],
    ["Response Snapshot", call.response_snapshot_path ?? "n/a"],
    ["Max Tokens", formatInteger(call.max_tokens)],
    ["Temperature", call.temperature === null ? "n/a" : String(call.temperature)],
    ["Workflow Run", call.workflow_run_id ?? "n/a"],
    ["Agent Run", call.agent_run_id ?? "n/a"],
    ["Work Item", call.work_item_id ?? "n/a"],
    ["Product", call.product_id ?? "n/a"],
    ["Session", call.session_id ?? "n/a"],
    ["Agent", call.agent_id ?? "n/a"],
    ["Stage", call.stage ?? "n/a"],
  ];
}

function sourceLabel(call: ModelCall) {
  if (call.source_label) return call.source_label;
  return call.source_kind
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function modelCallSessionKey(call: ModelCall) {
  if (call.session_id) return `session:${call.session_id}`;
  if (call.workflow_run_id) return `workflow:${call.workflow_run_id}`;
  if (call.agent_run_id) return `agent-run:${call.agent_run_id}`;
  if (call.source_id) return `source:${call.source_kind}:${call.source_id}`;
  return `call:${call.id}`;
}

function finiteTotal(values: Array<number | null | undefined>) {
  let total = 0;
  let hasValue = false;
  values.forEach((value) => {
    if (value !== null && value !== undefined && Number.isFinite(value)) {
      total += value;
      hasValue = true;
    }
  });
  return hasValue ? total : null;
}

function sessionStatus(calls: ModelCall[]) {
  if (calls.some((call) => call.status === "failed")) return "failed";
  if (calls.every((call) => call.status === "completed")) return "completed";
  return calls[0]?.status ?? "unknown";
}

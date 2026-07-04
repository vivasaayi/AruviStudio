import type { ModelCall, MobilePlannerToolTraceEntry } from "../types";

export function formatBytes(bytes?: number) {
  if (!bytes || bytes <= 0) return "";
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

export function formatInteger(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  return new Intl.NumberFormat().format(value);
}

export function formatDurationMs(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  if (value < 1000) return `${Math.max(0, Math.round(value))}ms`;
  const totalSeconds = Math.round(value / 1000);
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
}

export function describeError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object") {
    const errorRecord = error as Record<string, unknown>;
    const directMessage = errorRecord.message ?? errorRecord.error ?? errorRecord.description ?? errorRecord.reason;
    if (typeof directMessage === "string" && directMessage.trim()) {
      return directMessage;
    }
    try {
      return JSON.stringify(errorRecord);
    } catch {
      return Object.prototype.toString.call(error);
    }
  }
  return String(error);
}

export function compactJson(value: unknown, maxLength = 220) {
  if (value === null || value === undefined) return "";
  let rendered = "";
  try {
    rendered = typeof value === "string" ? value : JSON.stringify(value);
  } catch {
    rendered = String(value);
  }
  return rendered.length > maxLength ? `${rendered.slice(0, maxLength - 1)}...` : rendered;
}

export function formatPlannerToolTrace(entry: MobilePlannerToolTraceEntry) {
  const action = entry.tool_name.split(".").slice(-2).join(".");
  return `${entry.step}. ${action}${entry.error ? " failed" : " completed"}`;
}

export type ModelCallSessionSummary = {
  key: string;
  label: string;
  sessionId: string | null;
  sourceId: string | null;
  status: string;
  calls: ModelCall[];
  callCount: number;
  endedAt: string;
  providerLine: string;
  modelLine: string;
  tokenCountInput: number | null;
  tokenCountOutput: number | null;
  durationMs: number | null;
};

export function formatSourceKind(value: string) {
  return value
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function modelCallSourceLabel(call: ModelCall) {
  return call.source_label || formatSourceKind(call.source_kind);
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

export function buildModelCallSessions(calls: ModelCall[]): ModelCallSessionSummary[] {
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
      const latestCall = orderedCalls.reduce((latest, call) => (
        call.created_at > latest.created_at ? call : latest
      ), orderedCalls[0]);
      const providers = Array.from(
        new Set(orderedCalls.map((call) => call.provider_name || call.provider_id).filter(Boolean)),
      );
      const models = Array.from(new Set(orderedCalls.map((call) => call.model_name).filter(Boolean)));
      return {
        key,
        label: modelCallSourceLabel(latestCall),
        sessionId: latestCall.session_id,
        sourceId: latestCall.source_id,
        status: sessionStatus(orderedCalls),
        calls: orderedCalls,
        callCount: orderedCalls.length,
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

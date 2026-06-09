import React, { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listModelCalls, readModelCallSnapshot } from "../../../lib/tauri";
import type { ModelCall } from "../../../lib/types";

type CallFilter = "all" | "desktop" | "mobile" | "workflow" | "planner" | "failed";

interface ModelCallSession {
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

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 12, height: "100%" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  title: { fontSize: 22, fontWeight: 800, color: "#ffffff", margin: 0 },
  subtitle: { fontSize: 12, color: "#8f96a3", marginTop: 4 },
  actions: { display: "flex", gap: 8, alignItems: "center" },
  btn: { padding: "7px 12px", fontSize: 12, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer", fontWeight: 700 },
  filterBar: { display: "flex", gap: 8, flexWrap: "wrap", borderBottom: "1px solid #32353d", paddingBottom: 10 },
  filter: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#cfd6e4", cursor: "pointer" },
  filterActive: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#173247", color: "#ffffff", cursor: "pointer" },
  body: { display: "grid", gridTemplateColumns: "minmax(300px, 0.85fr) minmax(360px, 0.95fr) minmax(420px, 1.2fr)", gap: 12, minHeight: 0, flex: 1 },
  listPanel: { border: "1px solid #32353d", borderRadius: 8, backgroundColor: "#212327", minHeight: 0, overflow: "hidden", display: "flex", flexDirection: "column" },
  panelHeader: { padding: 12, borderBottom: "1px solid #32353d", display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" },
  panelTitle: { color: "#f3f3f3", fontSize: 13, fontWeight: 800 },
  panelMeta: { color: "#8f96a3", fontSize: 11 },
  list: { overflow: "auto", padding: 10, display: "flex", flexDirection: "column", gap: 8 },
  row: { width: "100%", textAlign: "left", border: "1px solid #2d3139", borderRadius: 8, backgroundColor: "#1b1d22", color: "#d7deea", padding: 10, cursor: "pointer" },
  rowActive: { border: "1px solid #57b0e5", backgroundColor: "#172536" },
  rowTop: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 },
  source: { fontSize: 13, fontWeight: 800, color: "#f3f3f3", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  status: { fontSize: 10, fontWeight: 800, color: "#4ec9b0", textTransform: "uppercase" },
  statusFailed: { color: "#ff9b9b" },
  small: { fontSize: 11, color: "#8f96a3", marginTop: 4, lineHeight: 1.4 },
  sessionSummary: { padding: 12, borderBottom: "1px solid #32353d" },
  sessionStats: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 8 },
  sessionStat: { border: "1px solid #32353d", borderRadius: 8, backgroundColor: "#1b1d22", padding: 10, minWidth: 0 },
  sessionStatValue: { color: "#f4f7fb", fontSize: 14, fontWeight: 800, overflowWrap: "anywhere" },
  sessionStatLabel: { color: "#8f96a3", fontSize: 10, fontWeight: 800, textTransform: "uppercase", marginTop: 4 },
  callsSubTitle: { color: "#f3f3f3", fontSize: 13, fontWeight: 800, margin: "6px 0 8px" },
  callList: { display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 },
  compactCall: { width: "100%", textAlign: "left", border: "1px solid #303640", borderRadius: 8, backgroundColor: "#191c22", color: "#d7deea", padding: 10, cursor: "pointer" },
  compactCallActive: { border: "1px solid #57b0e5", backgroundColor: "#142437" },
  compactCallMeta: { display: "flex", gap: 8, flexWrap: "wrap", color: "#8f96a3", fontSize: 11, marginTop: 5 },
  detailPanel: { border: "1px solid #32353d", borderRadius: 8, backgroundColor: "#212327", minHeight: 0, overflow: "auto", padding: 14 },
  detailTitle: { fontSize: 18, fontWeight: 800, color: "#ffffff", marginBottom: 4 },
  detailSub: { fontSize: 12, color: "#8f96a3", marginBottom: 12 },
  detailGrid: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  detailItem: { borderTop: "1px solid #32353d", paddingTop: 8, minWidth: 0 },
  label: { fontSize: 10, fontWeight: 800, textTransform: "uppercase", color: "#8f96a3", marginBottom: 4 },
  value: { fontSize: 12, color: "#e8edf7", overflowWrap: "anywhere", lineHeight: 1.45 },
  snapshotGrid: { display: "grid", gridTemplateColumns: "1fr", gap: 10, marginTop: 12 },
  snapshotBox: { border: "1px solid #32353d", borderRadius: 8, backgroundColor: "#171a1f", overflow: "hidden" },
  snapshotHeader: { padding: "8px 10px", borderBottom: "1px solid #32353d", display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" },
  snapshotTitle: { fontSize: 11, fontWeight: 800, color: "#f3f3f3" },
  snapshotPath: { fontSize: 10, color: "#8f96a3", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  snapshotPre: { margin: 0, padding: 10, maxHeight: 260, overflow: "auto", color: "#d7deea", fontSize: 11, lineHeight: 1.45, whiteSpace: "pre-wrap", fontFamily: "JetBrains Mono, Menlo, Monaco, monospace" },
  errorBox: { marginTop: 12, border: "1px solid #5a2f35", backgroundColor: "#2b1d22", borderRadius: 8, padding: 10, color: "#ffb4b4", fontSize: 12, whiteSpace: "pre-wrap" },
  empty: { color: "#666", textAlign: "center", padding: 40, fontSize: 13 },
};

const FILTERS: Array<{ id: CallFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "desktop", label: "Desktop" },
  { id: "mobile", label: "Mobile" },
  { id: "workflow", label: "Workflow" },
  { id: "planner", label: "Planner" },
  { id: "failed", label: "Failed" },
];

function formatInteger(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  return new Intl.NumberFormat().format(value);
}

function formatDurationMs(value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
  if (value < 1000) return `${Math.max(0, Math.round(value))}ms`;
  const seconds = Math.round(value / 1000);
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
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

function buildModelCallSessions(calls: ModelCall[]): ModelCallSession[] {
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

function matchesFilter(call: ModelCall, filter: CallFilter) {
  if (filter === "all") return true;
  if (filter === "failed") return call.status === "failed";
  if (filter === "desktop") return call.source_kind.startsWith("desktop");
  if (filter === "mobile") return call.source_kind.startsWith("mobile");
  if (filter === "workflow") return call.source_kind === "workflow_agent";
  if (filter === "planner") return call.source_kind.includes("planner") || call.source_kind.includes("repository_analysis");
  return true;
}

function detailRows(call: ModelCall) {
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

export function ModelCallsPage() {
  const [filter, setFilter] = useState<CallFilter>("all");
  const [selectedSessionKey, setSelectedSessionKey] = useState<string | null>(null);
  const [selectedCallId, setSelectedCallId] = useState<string | null>(null);
  const callsQuery = useQuery({
    queryKey: ["modelCalls"],
    queryFn: () => listModelCalls(300),
    refetchInterval: 5000,
  });
  const calls = callsQuery.data ?? [];
  const filteredCalls = useMemo(
    () => calls.filter((call) => matchesFilter(call, filter)),
    [calls, filter],
  );
  const sessions = useMemo(() => buildModelCallSessions(filteredCalls), [filteredCalls]);
  const selectedSession = sessions.find((session) => session.key === selectedSessionKey)
    ?? sessions[0]
    ?? null;
  const selectedCall = selectedSession?.calls.find((call) => call.id === selectedCallId)
    ?? selectedSession?.calls[selectedSession.calls.length - 1]
    ?? null;
  const snapshotsQuery = useQuery({
    queryKey: [
      "modelCallSnapshots",
      selectedCall?.id,
      selectedCall?.request_snapshot_path,
      selectedCall?.response_snapshot_path,
    ],
    enabled: Boolean(selectedCall?.id && (selectedCall.request_snapshot_path || selectedCall.response_snapshot_path)),
    queryFn: async () => {
      if (!selectedCall) return { request: null as string | null, response: null as string | null };
      const [request, response] = await Promise.all([
        selectedCall.request_snapshot_path
          ? readModelCallSnapshot(selectedCall.id, "request").catch((error) => `Unable to read request snapshot: ${String(error)}`)
          : Promise.resolve(null),
        selectedCall.response_snapshot_path
          ? readModelCallSnapshot(selectedCall.id, "response").catch((error) => `Unable to read response snapshot: ${String(error)}`)
          : Promise.resolve(null),
      ]);
      return { request, response };
    },
  });

  const selectSession = (session: ModelCallSession) => {
    setSelectedSessionKey(session.key);
    setSelectedCallId(session.calls[session.calls.length - 1]?.id ?? null);
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Calls</h1>
          <div style={styles.subtitle}>Model-call ledger across desktop, mobile, planner, and workflow sources.</div>
        </div>
        <div style={styles.actions}>
          <button style={styles.btn} onClick={() => callsQuery.refetch()}>
            {callsQuery.isFetching ? "Refreshing..." : "Refresh"}
          </button>
        </div>
      </div>

      <div style={styles.filterBar}>
        {FILTERS.map((item) => (
          <button
            key={item.id}
            style={filter === item.id ? styles.filterActive : styles.filter}
            onClick={() => setFilter(item.id)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div style={styles.body}>
        <section style={styles.listPanel}>
          <div style={styles.panelHeader}>
            <div style={styles.panelTitle}>Recent Sessions</div>
            <div style={styles.panelMeta}>
              {sessions.length} sessions · {filteredCalls.length} calls · {calls.length} loaded
            </div>
          </div>
          <div style={styles.list}>
            {sessions.length > 0 ? (
              sessions.map((session) => (
                <button
                  key={session.key}
                  style={selectedSession?.key === session.key ? { ...styles.row, ...styles.rowActive } : styles.row}
                  onClick={() => selectSession(session)}
                >
                  <div style={styles.rowTop}>
                    <div style={styles.source}>{session.label}</div>
                    <div style={session.status === "failed" ? { ...styles.status, ...styles.statusFailed } : styles.status}>
                      {session.status}
                    </div>
                  </div>
                  <div style={styles.small}>
                    {session.callCount} calls · {session.providerLine} / {session.modelLine}
                  </div>
                  <div style={styles.small}>
                    input {formatInteger(session.tokenCountInput)} · output {formatInteger(session.tokenCountOutput)} · {formatDurationMs(session.durationMs)}
                  </div>
                  <div style={styles.small}>{session.endedAt}</div>
                </button>
              ))
            ) : (
              <div style={styles.empty}>No sessions match this filter.</div>
            )}
          </div>
        </section>

        <section style={styles.listPanel}>
          <div style={styles.panelHeader}>
            <div style={styles.panelTitle}>Calls In Session</div>
            <div style={styles.panelMeta}>
              {selectedSession ? `${selectedSession.callCount} calls` : "No session"}
            </div>
          </div>
          {selectedSession ? (
            <>
              <div style={styles.sessionSummary}>
                <div style={styles.detailTitle}>{selectedSession.label}</div>
                <div style={styles.detailSub}>
                  {selectedSession.sessionId ?? selectedSession.sourceId ?? selectedSession.key}
                </div>
                <div style={styles.sessionStats}>
                  <div style={styles.sessionStat}>
                    <div style={styles.sessionStatValue}>{formatInteger(selectedSession.callCount)}</div>
                    <div style={styles.sessionStatLabel}>Calls</div>
                  </div>
                  <div style={styles.sessionStat}>
                    <div style={styles.sessionStatValue}>{formatDurationMs(selectedSession.durationMs)}</div>
                    <div style={styles.sessionStatLabel}>Duration</div>
                  </div>
                  <div style={styles.sessionStat}>
                    <div style={styles.sessionStatValue}>{formatInteger(selectedSession.tokenCountInput)}</div>
                    <div style={styles.sessionStatLabel}>Input Tokens</div>
                  </div>
                  <div style={styles.sessionStat}>
                    <div style={styles.sessionStatValue}>{formatInteger(selectedSession.tokenCountOutput)}</div>
                    <div style={styles.sessionStatLabel}>Output Tokens</div>
                  </div>
                </div>
              </div>

              <div style={styles.list}>
                {selectedSession.calls.map((call) => (
                  <button
                    key={call.id}
                    style={selectedCall.id === call.id ? { ...styles.compactCall, ...styles.compactCallActive } : styles.compactCall}
                    onClick={() => setSelectedCallId(call.id)}
                  >
                    <div style={styles.rowTop}>
                      <div style={styles.source}>Call #{formatInteger(call.call_index)}</div>
                      <div style={call.status === "failed" ? { ...styles.status, ...styles.statusFailed } : styles.status}>
                        {call.status}
                      </div>
                    </div>
                    <div style={styles.compactCallMeta}>
                      <span>{call.provider_name || call.provider_id} / {call.model_name}</span>
                      <span>input {formatInteger(call.token_count_input)}</span>
                      <span>output {formatInteger(call.token_count_output)}</span>
                      <span>{formatInteger(call.prompt_chars)} prompt chars</span>
                      <span>{formatInteger(call.response_chars)} response chars</span>
                      <span>{formatDurationMs(call.duration_ms)}</span>
                      <span>{call.created_at}</span>
                    </div>
                  </button>
                ))}
              </div>
            </>
          ) : (
            <div style={styles.empty}>Select a session to inspect calls.</div>
          )}
        </section>

        <aside style={styles.detailPanel}>
          {selectedCall ? (
            <>
              <div style={styles.callsSubTitle}>Selected Call Details</div>
              <div style={styles.detailGrid}>
                {detailRows(selectedCall).map(([label, value]) => (
                  <div key={label} style={styles.detailItem}>
                    <div style={styles.label}>{label}</div>
                    <div style={styles.value}>{value}</div>
                  </div>
                ))}
              </div>
              {selectedCall.error_message && (
                <div style={styles.errorBox}>{selectedCall.error_message}</div>
              )}

              <div style={styles.callsSubTitle}>Input And Output Snapshots</div>
              <div style={styles.snapshotGrid}>
                <div style={styles.snapshotBox}>
                  <div style={styles.snapshotHeader}>
                    <div style={styles.snapshotTitle}>Request Messages</div>
                    <div style={styles.snapshotPath}>{selectedCall.request_snapshot_path ?? "No request snapshot"}</div>
                  </div>
                  <pre style={styles.snapshotPre}>
                    {selectedCall.request_snapshot_path
                      ? snapshotsQuery.isFetching
                        ? "Loading request snapshot..."
                        : snapshotsQuery.data?.request ?? "Request snapshot not loaded."
                      : "No request snapshot was captured for this call."}
                  </pre>
                </div>

                <div style={styles.snapshotBox}>
                  <div style={styles.snapshotHeader}>
                    <div style={styles.snapshotTitle}>Response Text</div>
                    <div style={styles.snapshotPath}>{selectedCall.response_snapshot_path ?? "No response snapshot"}</div>
                  </div>
                  <pre style={styles.snapshotPre}>
                    {selectedCall.response_snapshot_path
                      ? snapshotsQuery.isFetching
                        ? "Loading response snapshot..."
                        : snapshotsQuery.data?.response ?? "Response snapshot not loaded."
                      : "No response snapshot was captured for this call."}
                  </pre>
                </div>
              </div>
            </>
          ) : (
            <div style={styles.empty}>Select a call to inspect details.</div>
          )}
        </aside>
      </div>
    </div>
  );
}

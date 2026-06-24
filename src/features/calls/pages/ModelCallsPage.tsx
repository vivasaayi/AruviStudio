import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listModelCalls, readModelCallSnapshot } from "../../../lib/tauri";
import { styles } from "./ModelCallsPage.styles";
import {
  MODEL_CALL_FILTERS,
  buildModelCallSessions,
  formatDurationMs,
  formatInteger,
  matchesModelCallFilter,
  modelCallDetailRows,
  type CallFilter,
  type ModelCallSession,
} from "../lib/modelCallSessions";

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
    () => calls.filter((call) => matchesModelCallFilter(call, filter)),
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
        {MODEL_CALL_FILTERS.map((item) => (
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
                {modelCallDetailRows(selectedCall).map(([label, value]) => (
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

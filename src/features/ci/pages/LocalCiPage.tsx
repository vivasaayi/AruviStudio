import React, { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getLocalCiStatus,
  listLocalCiRuns,
  listLocalCiTargets,
  openLocalPreview,
  queueLocalPreview,
  type LocalCiRun,
} from "../../../lib/tauri/ci";

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 1080, margin: "0 auto", color: "#d4d4d4" },
  header: { display: "flex", justifyContent: "space-between", gap: 16, alignItems: "start", marginBottom: 20 },
  card: { background: "#252526", border: "1px solid #3a3a3a", borderRadius: 8, padding: 16, marginBottom: 12 },
  row: { display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" },
  button: { border: 0, borderRadius: 4, background: "#0e639c", color: "white", padding: "7px 11px", cursor: "pointer" },
  secondary: { border: "1px solid #555", borderRadius: 4, background: "#2d2d30", color: "white", padding: "7px 11px", cursor: "pointer" },
  input: { flex: 1, minWidth: 340, padding: "8px", background: "#1e1e1e", color: "white", border: "1px solid #555", borderRadius: 4, fontFamily: "monospace" },
  badge: { borderRadius: 99, padding: "2px 8px", background: "#333", fontSize: 12 },
  muted: { color: "#9da0a6", fontSize: 13 },
};

const runColor = (status: string) => status === "passed" ? "#3fb950" : status === "failed" ? "#f85149" : "#d29922";

function RunCard({ run }: { run: LocalCiRun }) {
  return <div style={styles.card}>
    <div style={{ ...styles.row, justifyContent: "space-between" }}>
      <strong>#{run.id} · {run.target}</strong>
      <span style={{ ...styles.badge, color: runColor(run.status) }}>{run.status}</span>
    </div>
    <div style={{ ...styles.muted, fontFamily: "monospace", marginTop: 8 }}>{run.commit}</div>
    <div style={{ ...styles.row, marginTop: 10 }}>
      {run.stages.map((stage) => <span key={stage.name} style={{ ...styles.badge, color: runColor(stage.status) }}>{stage.name}: {stage.status}</span>)}
    </div>
    {run.error && <div style={{ color: "#f85149", marginTop: 10 }}>{run.error}</div>}
  </div>;
}

export function LocalCiPage() {
  const client = useQueryClient();
  const [commit, setCommit] = useState("");
  const status = useQuery({ queryKey: ["local-ci-status"], queryFn: getLocalCiStatus, retry: false });
  const targets = useQuery({ queryKey: ["local-ci-targets"], queryFn: listLocalCiTargets, retry: false });
  const runs = useQuery({ queryKey: ["local-ci-runs"], queryFn: listLocalCiRuns, retry: false, refetchInterval: 3000 });
  const queue = useMutation({
    mutationFn: () => queueLocalPreview(commit),
    onSuccess: () => { setCommit(""); void client.invalidateQueries({ queryKey: ["local-ci-runs"] }); },
  });
  const open = useMutation({ mutationFn: openLocalPreview });
  const error = status.error ?? targets.error ?? runs.error ?? queue.error ?? open.error;
  const previewConfigured = (targets.data ?? []).some((target) => target.id === "aruvi-studio-preview");
  return <div style={styles.page}>
    <div style={styles.header}>
      <div><h1 style={{ margin: 0 }}>Local CI & Preview</h1><p style={styles.muted}>Build verified candidates locally. Production promotion is intentionally not available here.</p></div>
      <button style={styles.secondary} onClick={() => void open.mutateAsync()}>Open Preview</button>
    </div>
    <div style={styles.card}>
      <div style={styles.row}><strong>Service</strong><span style={{ ...styles.badge, color: status.data?.status === "online" ? "#3fb950" : "#f85149" }}>{status.data?.status ?? "offline"}</span><span style={styles.muted}>{previewConfigured ? "Preview target approved/configured" : "Preview target not available"}</span></div>
      <p style={styles.muted}>Queue only a committed revision. Aruvici performs the isolated build, verification, and safe side-by-side Preview update.</p>
      <div style={styles.row}><input aria-label="Committed revision" style={styles.input} value={commit} onChange={(event) => setCommit(event.target.value.trim())} placeholder="40-character committed Git SHA" /><button style={styles.button} disabled={!previewConfigured || queue.isPending} onClick={() => queue.mutate()}>{queue.isPending ? "Queueing…" : "Build Preview"}</button></div>
    </div>
    {error && <div style={{ ...styles.card, color: "#f85149" }}>{String(error)}</div>}
    <h2>Build history</h2>
    {(runs.data ?? []).map((run) => <RunCard key={run.id} run={run} />)}
    {!runs.data?.length && !runs.isLoading && <div style={styles.muted}>No local CI runs yet.</div>}
  </div>;
}

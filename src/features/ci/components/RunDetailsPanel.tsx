import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cancelLocalCiRun,
  getCiPromotionReadiness,
  getLocalCiLog,
  getLocalCiRun,
  revealCiArtifact,
  retryLocalCiRun,
  type CiPromotionReadiness,
  type LocalCiRun,
} from "../../../lib/tauri/ci";
import { CiFeedbackButtons } from "./CiFeedbackButtons";

const muted: React.CSSProperties = { color: "#657085", fontSize: 13, lineHeight: 1.45 };
const code: React.CSSProperties = { fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12, color: "#485974" };
const secondary: React.CSSProperties = { border: "1px solid #c8d1e1", borderRadius: 7, background: "#fff", color: "#27344b", padding: "7px 10px", cursor: "pointer", fontWeight: 600 };
const terminal = new Set(["passed", "succeeded", "complete", "failed", "blocked", "cancelled", "interrupted"]);
const retryable = new Set(["failed", "blocked", "cancelled", "interrupted"]);
const tone = (status: string) => status === "passed" || status === "succeeded" || status === "complete" ? "#16834b" : status === "failed" || status === "blocked" ? "#c23636" : "#9a5a05";
const pretty = (value: string) => value.replaceAll("_", " ");
const bytes = (size: number) => size < 1024 ? `${size} B` : size < 1024 * 1024 ? `${(size / 1024).toFixed(1)} KiB` : `${(size / (1024 * 1024)).toFixed(1)} MiB`;

function Status({ value, children }: { value: string; children?: React.ReactNode }) {
  return <span style={{ borderRadius: 99, padding: "3px 8px", color: tone(value), background: `${tone(value)}12`, fontSize: 12, fontWeight: 650 }}>{children ?? pretty(value)}</span>;
}

export function RunDetailsPanel({ summary }: { summary: LocalCiRun }) {
  const client = useQueryClient();
  const [expanded, setExpanded] = useState(false);
  const [stage, setStage] = useState("");
  const [log, setLog] = useState("");
  const [logError, setLogError] = useState("");
  const [readiness, setReadiness] = useState<CiPromotionReadiness | null>(null);
  const [actionMessage, setActionMessage] = useState("");
  const active = summary.status === "queued" || summary.status === "running";
  const run = useQuery({
    queryKey: ["local-ci-run", summary.id],
    queryFn: () => getLocalCiRun(summary.id),
    enabled: expanded,
    initialData: summary,
    refetchInterval: expanded && active ? 2000 : false,
  });
  const detail = run.data ?? summary;
  const stages = detail.stages ?? [];
  const selectedStage = stage || stages.find((item) => item.status === "running")?.name || stages.find((item) => item.status === "failed")?.name || stages[0]?.name || "";

  useEffect(() => {
    if (!expanded || !selectedStage) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let offset = 0;
    setLog("");
    setLogError("");
    const poll = async () => {
      try {
        const chunk = await getLocalCiLog(summary.id, selectedStage, offset);
        if (stopped) return;
        if (chunk.text) {
          setLog((current) => `${current}${chunk.text}`);
          offset = chunk.next_offset;
          timer = setTimeout(poll, 50);
        } else if (!terminal.has(detail.status)) {
          timer = setTimeout(poll, 1500);
        }
      } catch (error) {
        if (!stopped) {
          setLogError(String(error));
          if (!terminal.has(detail.status)) timer = setTimeout(poll, 1500);
        }
      }
    };
    void poll();
    return () => { stopped = true; if (timer) clearTimeout(timer); };
  }, [detail.status, expanded, selectedStage, summary.id]);

  const cancel = useMutation({
    mutationFn: () => cancelLocalCiRun(summary.id),
    onSuccess: async () => { setActionMessage("Cancellation requested."); await client.invalidateQueries({ queryKey: ["local-ci-runs"] }); await run.refetch(); },
  });
  const retry = useMutation({
    mutationFn: () => retryLocalCiRun(summary.id),
    onSuccess: async (next) => { setActionMessage(`Queued retry as run #${next.run_id}.`); await client.invalidateQueries({ queryKey: ["local-ci-runs"] }); },
  });
  const verify = useMutation({
    mutationFn: getCiPromotionReadiness,
    onSuccess: (value) => setReadiness(value),
  });
  const reveal = useMutation({ mutationFn: revealCiArtifact });
  const diagnostics = useMemo(() => JSON.stringify({
    run: detail.id, target: detail.target, commit: detail.commit, plan_digest: detail.plan_digest,
    status: detail.status, error: detail.error, stages: detail.stages, artifacts: detail.artifacts,
  }, null, 2), [detail]);

  return <article style={{ background: "#fff", border: "1px solid #d9dfeb", borderRadius: 12, padding: 16, boxShadow: "0 1px 2px rgba(35,50,80,.04)" }}>
    <div style={{ display: "flex", gap: 10, alignItems: "center", justifyContent: "space-between", flexWrap: "wrap" }}>
      <div><strong>Run #{summary.id} · {summary.target}</strong><div style={{ ...code, marginTop: 5 }}>{summary.commit.slice(0, 12)} · {summary.created_at}</div></div>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}><Status value={detail.status} /><button style={secondary} onClick={() => setExpanded((value) => !value)}>{expanded ? "Hide details" : "View details"}</button></div>
    </div>
    {!expanded && <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 11 }}>{stages.map((item) => <Status key={item.name} value={item.status}>{item.name}: {pretty(item.status)}</Status>)}</div>}
    {detail.error && <div style={{ color: "#c23636", marginTop: 10, fontSize: 13 }}>{detail.error}</div>}
    {expanded && <div style={{ marginTop: 16, borderTop: "1px solid #e4e8f0", paddingTop: 14 }}>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
        {(detail.status === "queued" || detail.status === "running") && <button style={secondary} disabled={cancel.isPending} onClick={() => cancel.mutate()}>{cancel.isPending ? "Cancelling…" : "Cancel build"}</button>}
        {retryable.has(detail.status) && <button style={secondary} disabled={retry.isPending} onClick={() => retry.mutate()}>{retry.isPending ? "Queueing…" : "Retry build"}</button>}
        <button style={secondary} onClick={() => void navigator.clipboard.writeText(diagnostics).then(() => setActionMessage("Diagnostics copied."), (error) => setActionMessage(`Could not copy diagnostics: ${error}`))}>Copy diagnostics</button>
      </div>
      {actionMessage && <div style={{ ...muted, marginBottom: 10 }}>{actionMessage}</div>}
      {(cancel.error || retry.error) && <div style={{ color: "#c23636", marginBottom: 10 }}>{String(cancel.error ?? retry.error)}</div>}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 8, ...muted }}>
        <div><strong>Commit</strong><div style={code}>{detail.commit}</div></div>
        <div><strong>Pipeline digest</strong><div style={code}>{detail.plan_digest}</div></div>
        <div><strong>Started</strong><div>{detail.started_at ?? "Not started"}</div></div>
        <div><strong>Finished</strong><div>{detail.finished_at ?? "In progress"}</div></div>
      </div>
      <h3 style={{ marginBottom: 8 }}>Stages and logs</h3>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>{stages.map((item) => <button key={item.name} style={{ ...secondary, borderColor: selectedStage === item.name ? "#2864dc" : "#c8d1e1" }} onClick={() => setStage(item.name)}>{item.name} · {pretty(item.status)}</button>)}</div>
      {selectedStage && <pre style={{ margin: 0, minHeight: 90, maxHeight: 320, overflow: "auto", background: "#111827", color: "#d8e2f1", padding: 12, borderRadius: 8, whiteSpace: "pre-wrap", fontSize: 12 }}>{log || (logError ? `Log unavailable: ${logError}` : "Waiting for log output…")}</pre>}
      <h3 style={{ marginBottom: 8 }}>Artifacts</h3>
      {!detail.artifacts.length && <div style={muted}>No artifacts recorded for this run.</div>}
      <div style={{ display: "grid", gap: 8 }}>{detail.artifacts.map((artifact) => <div key={artifact.id} style={{ border: "1px solid #e2e7f0", borderRadius: 8, padding: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap" }}><div><strong>{artifact.kind}</strong> · {artifact.stage}<div style={code}>{artifact.sha256}</div><div style={muted}>{bytes(artifact.size)}</div></div><div style={{ display: "flex", gap: 6, alignItems: "center" }}><button style={secondary} disabled={reveal.isPending} onClick={() => reveal.mutate(artifact.id)}>Show in Finder</button><button style={secondary} disabled={verify.isPending} onClick={() => { setReadiness(null); verify.mutate(artifact.id); }}>{verify.isPending ? "Verifying…" : "Verify again"}</button></div></div>
        {readiness?.artifact_id === artifact.id && <div style={{ marginTop: 8, color: readiness.eligible ? "#16834b" : "#9a5a05", fontSize: 13 }}><strong>{readiness.eligible ? "Promotion ready" : "Verified, not promotable"}</strong> · {readiness.reason}</div>}
      </div>)}</div>
      {(verify.error || reveal.error) && <div style={{ color: "#c23636", marginTop: 8 }}>{String(verify.error ?? reveal.error)}</div>}
      <CiFeedbackButtons run={detail} />
    </div>}
  </article>;
}

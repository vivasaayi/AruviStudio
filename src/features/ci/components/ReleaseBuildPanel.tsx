import React, { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queueLocalRelease } from "../../../lib/tauri/ci";

export function ReleaseBuildPanel({ online, targets }: { online: boolean; targets: Array<{ id: string }> }) {
  const client = useQueryClient();
  const [commit, setCommit] = useState("");
  const [target, setTarget] = useState<"aruvi-studio" | "aruvi-studio-intel">("aruvi-studio");
  const configured = targets.some((item) => item.id === target);
  const build = useMutation({ mutationFn: () => queueLocalRelease(target, commit), onSuccess: () => { setCommit(""); void client.invalidateQueries({ queryKey: ["local-ci-runs"] }); } });
  return <section style={{ background: "#fff", border: "1px solid #d9dfeb", borderRadius: 12, padding: 16, marginBottom: 16 }}>
    <strong>Build production artifact</strong>
    <p style={{ color: "#657085", fontSize: 13 }}>This creates a signed, verified ZIP only. It cannot install, replace, or launch the production app.</p>
    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
      <select value={target} onChange={(event) => setTarget(event.target.value as typeof target)}><option value="aruvi-studio">Apple Silicon production</option><option value="aruvi-studio-intel">Intel production</option></select>
      <input aria-label="Production commit" value={commit} onChange={(event) => setCommit(event.target.value.trim())} placeholder="40-character committed Git SHA" style={{ minWidth: 330, padding: 8 }} />
      <button disabled={!online || !configured || build.isPending} onClick={() => build.mutate()}>{build.isPending ? "Queueing…" : "Build production artifact"}</button>
    </div>
    {build.isSuccess && <div style={{ color: "#16834b", marginTop: 8 }}>Production build queued. Review its artifact and checksum before promotion.</div>}
    {build.error && <div style={{ color: "#c23636", marginTop: 8 }}>{String(build.error)}</div>}
  </section>;
}

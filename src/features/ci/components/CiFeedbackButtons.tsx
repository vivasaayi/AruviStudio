import React from "react";
import { useMutation } from "@tanstack/react-query";
import { createCiFeedback, type LocalCiRun } from "../../../lib/tauri/ci";

export function CiFeedbackButtons({ run }: { run: LocalCiRun }) {
  const save = useMutation({ mutationFn: ({ verdict, notes }: { verdict: "works" | "needs_changes" | "blocked"; notes: string }) => createCiFeedback({ ciRunId: run.id, commitSha: run.commit, verdict, notes }) });
  const record = (verdict: "works" | "needs_changes" | "blocked") => { const notes = verdict === "works" ? "Preview accepted" : window.prompt("What did you observe in this Preview?") ?? ""; if (verdict !== "works" && !notes) return; save.mutate({ verdict, notes }); };
  if (run.status !== "passed") return null;
  return <div style={{ display: "flex", gap: 7, alignItems: "center", marginTop: 12 }}><span style={{ color: "#657085", fontSize: 12 }}>Preview feedback:</span><button onClick={() => record("works")}>Works</button><button onClick={() => record("needs_changes")}>Needs changes</button><button onClick={() => record("blocked")}>Blocked</button>{save.isSuccess && <span style={{ color: "#16834b", fontSize: 12 }}>Recorded</span>}{save.error && <span style={{ color: "#c23636", fontSize: 12 }}>{String(save.error)}</span>}</div>;
}

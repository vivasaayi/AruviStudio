import net from "node:net";
import fs from "node:fs";

const path = "/ci/ci.sock";
try { fs.unlinkSync(path); } catch (error) { if (error.code !== "ENOENT") throw error; }
const server = net.createServer((socket) => {
  let input = "";
  socket.on("data", (chunk) => {
    input += chunk.toString();
    if (!input.includes("\n")) return;
    const { method, id, stage, offset, key, target, commit } = JSON.parse(input.split("\n")[0]);
    const result = method === "health" ? { status: "online", schema: 1 } : method === "targets" ? [{ id: "fixture-preview", repository: "/repositories/ci-fixture", approval: { approved: true } }, { id: "unapproved-preview", repository: "/repositories/ci-fixture", approval: { approved: false } }, { id: "mismatch-preview", repository: "/repositories", approval: { approved: true } }] : method === "runs" ? [{ id: 42, target: "fixture-preview", status: "failed" }, { id: 43, target: "fixture-preview", status: "running" }, { id: 44, target: "fixture-preview", status: "passed" }] : method === "run" && [42, 43, 44].includes(id) ? { id, commit: "a".repeat(40), status: id === 42 ? "failed" : id === 43 ? "running" : "passed", stages: [{ name: "build", status: "failed" }], artifacts: [{ id: 88, kind: "archive" }] } : method === "promotion_readiness" && id === 88 ? { artifact_id: 88, eligible: true, reason: "fixture verified", manual_promotion_only: true } : method === "logs" && id === 42 && stage === "build" ? { text: offset === 0 ? "build output\n" : "", next_offset: 13 } : method === "logs" && id === 43 && stage === "build" ? { text: offset === 0 ? "live one\n" : offset === 9 ? "live two\n" : "", next_offset: offset === 0 ? 9 : 18 } : method === "cancel" && id === 43 ? { id, status: "cancelling" } : method === "retry" && id === 42 && typeof key === "string" ? { run_id: 44 } : method === "enqueue" && target === "fixture-preview" && /^[0-9a-f]{40}$/.test(commit) ? { run_id: 45, commit } : null;
    socket.end(JSON.stringify(result === null ? { ok: false, error: "unsupported" } : { ok: true, result }) + "\n");
  });
});
server.listen(path, () => fs.chmodSync(path, 0o777));

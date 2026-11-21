// Disposable Studio and provider fixture for the browser voice-session test.
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import http from "node:http";
import net from "node:net";

const root = await mkdtemp(join(tmpdir(), "aruvi-voice-conversation-"));
const provider = http.createServer((request, response) => {
  const chunks = [];
  request.on("data", (chunk) => chunks.push(chunk));
  request.on("end", () => {
    response.setHeader("Content-Type", "application/json");
    const body = Buffer.concat(chunks);
    if (request.url === "/v1/audio/transcriptions" && body.includes("RIFF") && body.includes("synthetic-stt-model")) {
      response.end(JSON.stringify({ text: "Synthetic microphone transcript" }));
    } else if (request.url === "/v1/chat/completions") {
      response.end(JSON.stringify({ choices: [{ message: { role: "assistant", content: "Container provider reply" }, finish_reason: "stop" }] }));
    } else {
      response.statusCode = 400; response.end(JSON.stringify({ error: "Unexpected fixture request" }));
    }
  });
});
let server;
try {
  await new Promise((resolveListen, reject) => { provider.once("error", reject); provider.listen(0, "127.0.0.1", resolveListen); });
  const port = await new Promise((resolvePort) => { const socket = net.createServer(); socket.listen(0, "127.0.0.1", () => { const value = socket.address().port; socket.close(() => resolvePort(value)); }); });
  const token = "voice-conversation-fixture-token", base = `http://127.0.0.1:${port}`;
  const env = { ...process.env, ARUVI_APP_DATA_DIR: root, ARUVI_DB_PATH: join(root, "db.sqlite"), ARUVI_WEBHOOK_HOST: "127.0.0.1", ARUVI_WEBHOOK_PORT: String(port), ARUVI_MOBILE_API_TOKEN: token, ARUVI_MCP_API_TOKEN: token, ARUVI_REMOTE_URL: base, ARUVI_TEST_MODEL_URL: `http://127.0.0.1:${provider.address().port}/v1`, ARUVI_ISOLATED_WORKFLOW: "1", ARUVI_CHROME_PATH: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" };
  server = spawn(resolve("src-tauri/target/debug/aruvi-studio-server"), [], { stdio: "ignore", env });
  for (let attempt = 0;; attempt++) {
    if (server.exitCode !== null || attempt === 120) throw new Error("Native fixture did not start");
    try { if ((await fetch(base + "/health")).ok) break; } catch {}
    await new Promise((resolveWait) => setTimeout(resolveWait, 250));
  }
  const browser = spawn(resolve("node_modules/.bin/playwright"), ["test", "-c", "playwright.remote.config.js", "workflow.spec.js", "-g", "voice records, reviews, and sends a transcribed message|browser continuous voice conversation uses Studio|browser navigation discards manual voice capture", "--workers=1"], { stdio: "inherit", env });
  const status = await new Promise((resolveStatus, reject) => { browser.once("error", reject); browser.once("exit", (code, signal) => resolveStatus({ code, signal })); });
  if (status.code !== 0) throw new Error(`Voice browser test failed: ${status.code ?? status.signal}`);
} finally {
  if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise((resolveExit) => server.once("exit", resolveExit)); }
  await new Promise((resolveClose) => provider.close(resolveClose));
  await rm(root, { recursive: true, force: true });
}
